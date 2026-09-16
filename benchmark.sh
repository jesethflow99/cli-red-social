#!/usr/bin/env bash
# =============================================================================
#  benchmark.sh — Benchmark de rendimiento para AGORA
#
#  Valida las tres capacidades principales:
#    1. RAM por sesión SSH (~8 MB con la TUI nativa)
#    2. Throughput y latencia de escritura en SQLite (WAL, escritor encolado)
#    3. Throughput de lectura (timeline concurrente)
#
#  Uso:
#    ./benchmark.sh [--users N] [--writes N] [--reads N] [--port P] [--db PATH]
#
#  Requiere: el binario release (target/release/agora) compilado.
# =============================================================================
set -uo pipefail

# ── Configuración por defecto ────────────────────────────────────────────────
USERS=${USERS:-50}          # sesiones SSH simultáneas (test de RAM)
WRITES=${WRITES:-200}       # escrituras concurrentes (posts)
READS=${READS:-100}         # lecturas concurrentes (timeline)
PORT=${PORT:-2224}
DB_PATH=${DB_PATH:-/tmp/agora_bench/agora.db}
BIN="${BIN:-$(cd "$(dirname "$0")" && pwd)/target/release/agora}"
SSH_PASS="${SSH_PASS:-benchpass}"
WORK=/tmp/agora_bench
KEY="$WORK/host_key"

# ── Parámetros CLI ───────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --users)  USERS="$2";  shift 2 ;;
    --writes) WRITES="$2"; shift 2 ;;
    --reads)  READS="$2";  shift 2 ;;
    --port)   PORT="$2";   shift 2 ;;
    --db)     DB_PATH="$2"; shift 2 ;;
    --bin)    BIN="$2";    shift 2 ;;
    --ssh-pass) SSH_PASS="$2"; shift 2 ;;
    *) echo "Argumento desconocido: $1" >&2; exit 2 ;;
  esac
done

# ── Utilidades ───────────────────────────────────────────────────────────────
fatal() { echo "❌ $*" >&2; exit 1; }
hr()    { printf '=%.0s' {1..72}; echo; }
fmt_ms() { awk -v ns="$1" 'BEGIN{printf "%.2f ms", ns/1e6}'; }

say() { echo ""; echo "▶ $*"; }

[[ -x "$BIN" ]] || fatal "Binario no encontrado: $BIN. Compilá con: cargo build --release"
command -v sqlite3 >/dev/null || fatal "Se necesita sqlite3 (sqlite3 CLI)"
command -v ssh >/dev/null || fatal "Se necesita ssh"

mkdir -p "$WORK"
RESULT_FILE="$WORK/benchmark-results.txt"

# ── Resultados ───────────────────────────────────────────────────────────────
RESULTS=()
record() { RESULTS+=("$1"); }

# =============================================================================
#  TEST 1 — RAM por sesión SSH
# =============================================================================
ram_test() {
  say "TEST 1: RAM por sesión SSH (TUI nativa)"

  # Generar host key y DB limpia con seed
  [[ -f "$KEY" ]] || ssh-keygen -t ed25519 -f "$KEY" -N "" -q
  rm -f "$DB_PATH" "$DB_PATH-wal" "$DB_PATH-shm"
  "$BIN" --db "$DB_PATH" --seed >/dev/null 2>&1 || fatal "Fallo el seed"

  # Askpass para no depender de sshpass
  cat > "$WORK/askpass.sh" <<EOF
#!/bin/bash
echo "$SSH_PASS"
EOF
  chmod +x "$WORK/askpass.sh"

  # Levantar servidor
  SSH_PASSWORD="$SSH_PASS" "$BIN" --port "$PORT" --key "$KEY" --db "$DB_PATH" \
    >/dev/null 2>&1 &
  local srv_pid=$!
  sleep 2

  # Lanzar N sesiones SSH idle
  local ssh_pids=()
  for ((i=1; i<=USERS; i++)); do
    DISPLAY=:0 SSH_ASKPASS="$WORK/askpass.sh" setsid \
      ssh -p "$PORT" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
          -o PreferredAuthentications=password -o NumberOfPasswordPrompts=1 \
          -t user@127.0.0.1 -t </dev/null >/dev/null 2>&1 &
    ssh_pids+=($!)
  done

  sleep 4  # dejar que las sesiones se establezcan

  # Medir RAM de todos los procesos agora (servidor + N hijos forkpty).
  # Los hijos de forkpty se llaman "tokio-rt-worker" (comm), así que hay que
  # matchear por línea de comando, no por comm.
  #
  # IMPORTANTE — por qué usamos PSS y no RSS:
  # Cada sesión se crea con forkpty() tras inicializar el runtime de tokio.
  # El fork() comparte el espacio de memoria del padre (copy-on-write), y el
  # RSS de cada hijo cuenta esas páginas compartidas como privadas. Con muchas
  # sesiones, el RSS infla hasta 2-8x la RAM real. El PSS (Proportional Set
  # Size) reparte las páginas compartidas proporcionalmente y es la métrica
  # correcta de RAM privada por sesión.
  local server_line="agora --port $PORT"
  local pids
  pids=$(pgrep -f "$server_line" 2>/dev/null | tr '\n' ' ')

  local child_pss child_rss child_count
  child_pss=0
  child_rss=0
  child_count=0
  for pid in $pids; do
    [[ "$pid" == "$srv_pid" ]] && continue
    child_pss=$(( child_pss + $(grep -E '^Pss:' /proc/$pid/smaps_rollup 2>/dev/null | awk '{print $2}') ))
    child_rss=$(( child_rss + $(awk '/VmRSS/{print $2}' /proc/$pid/status 2>/dev/null) ))
    child_count=$(( child_count + 1 ))
  done

  # Limpiar sesiones y servidor
  for p in "${ssh_pids[@]}"; do kill "$p" 2>/dev/null; done
  kill "$srv_pid" 2>/dev/null
  pkill -f "agora --port $PORT" 2>/dev/null
  sleep 1

  local per_session_pss per_session_rss
  if [[ "$child_count" -gt 0 && "$child_pss" -gt 0 ]]; then
    per_session_pss=$(awk -v c="$child_count" -v r="$child_pss" 'BEGIN{printf "%.1f", r/1024/c}')
    per_session_rss=$(awk -v c="$child_count" -v r="$child_rss" 'BEGIN{printf "%.1f", r/1024/c}')
  else
    per_session_pss="N/A"
    per_session_rss="N/A"
    child_count="N/A (sesiones no se establecieron)"
  fi

  record "TEST 1: RAM por sesión SSH"
  record "  Sesiones lanzadas:  $USERS"
  record "  Hijos forkpty vivos: $child_count"
  record "  PSS por sesión:     ${per_session_pss} MB  ← RAM privada REAL"
  record "  RSS por sesión:     ${per_session_rss} MB  (inflado por páginas compartidas del fork)"
  record ""

  echo "  Sesiones lanzadas:  $USERS"
  echo "  Hijos forkpty vivos: $child_count"
  echo "  PSS por sesión:     ${per_session_pss} MB"
  echo "  RSS por sesión:     ${per_session_rss} MB"
}

# =============================================================================
#  TEST 2 — Escritura concurrente (SQLite WAL, escritor encolado)
# =============================================================================
write_test() {
  say "TEST 2: Escritura concurrente (posts) — SQLite WAL"

  [[ -f "$DB_PATH" ]] || { "$BIN" --db "$DB_PATH" --seed >/dev/null 2>&1; }

  # Cargar el esquema en sqlite3: crea un usuario de prueba para los inserts
  sqlite3 "$DB_PATH" "SELECT 1 FROM users LIMIT 1;" >/dev/null || fatal "DB no legible"

  local times_file="$WORK/write_times.txt"
  rm -f "$times_file"

  local start_ns end_ns
  for ((i=1; i<=WRITES; i++)); do
    start_ns=$(date +%s%N)
    sqlite3 "$DB_PATH" "INSERT INTO posts (user_id, content, image_path, created_at)
                        VALUES (1, 'bench post $i', '', datetime('now'));" 2>/dev/null &
    local_pid=$!
    wait $local_pid
    end_ns=$(date +%s%N)
    echo $(( end_ns - start_ns )) >> "$times_file"
  done

  local n ops
  n=$(wc -l < "$times_file")
  ops=$(( n * 1000 ))  # estimado
  # Calcular p50, p95, p99 y media
  local stats
  stats=$(sort -n "$times_file" | awk -v n="$n" '
    NR==n { sum=$0 }   # último = máximo tras sort
    { arr[NR]=$1; s+=$1 }
    END {
      p50=arr[int(n*0.50+0.5)];
      p95=arr[int(n*0.95+0.5)];
      p99=arr[int(n*0.99+0.5)];
      mean=s/n;
      printf "%.2f %.2f %.2f %.2f\n", mean/1e6, p50/1e6, p95/1e6, p99/1e6;
    }')
  read -r mean_ms p50_ms p95_ms p99_ms <<< "$stats"

  record "TEST 2: Escritura concurrente (SQLite WAL)"
  record "  Escrituras:          $n"
  record "  Media por escritura: ${mean_ms} ms"
  record "  p50:                 ${p50_ms} ms"
  record "  p95:                 ${p95_ms} ms"
  record "  p99:                 ${p99_ms} ms"
  record ""

  echo "  Escrituras:          $n"
  echo "  Media por escritura: ${mean_ms} ms"
  echo "  p50: ${p50_ms} ms   p95: ${p95_ms} ms   p99: ${p99_ms} ms"
}

# =============================================================================
#  TEST 3 — Lectura concurrente (timeline)
# =============================================================================
read_test() {
  say "TEST 3: Lectura concurrente (timeline) — SQLite WAL"

  [[ -f "$DB_PATH" ]] || { "$BIN" --db "$DB_PATH" --seed >/dev/null 2>&1; }

  local times_file="$WORK/read_times.txt"
  rm -f "$times_file"

  local start_ns end_ns pids=()
  for ((i=1; i<=READS; i++)); do
    (
      local s e
      s=$(date +%s%N)
      sqlite3 "$DB_PATH" "SELECT p.content FROM posts p JOIN users u ON u.id = p.user_id
                          ORDER BY p.created_at DESC LIMIT 50;" >/dev/null 2>&1
      e=$(date +%s%N)
      echo $(( e - s )) >> "$times_file"
    ) &
    pids+=($!)
  done
  for p in "${pids[@]}"; do wait "$p" 2>/dev/null; done

  local n
  n=$(wc -l < "$times_file")
  local stats
  stats=$(sort -n "$times_file" | awk -v n="$n" '
    { arr[NR]=$1; s+=$1 }
    END {
      p50=arr[int(n*0.50+0.5)];
      p95=arr[int(n*0.95+0.5)];
      p99=arr[int(n*0.99+0.5)];
      printf "%.2f %.2f %.2f %.2f\n", (s/n)/1e6, p50/1e6, p95/1e6, p99/1e6;
    }')
  read -r mean_ms p50_ms p95_ms p99_ms <<< "$stats"

  record "TEST 3: Lectura concurrente (SQLite WAL)"
  record "  Lecturas:            $n"
  record "  Media por lectura:   ${mean_ms} ms"
  record "  p50:                 ${p50_ms} ms"
  record "  p95:                 ${p95_ms} ms"
  record "  p99:                 ${p99_ms} ms"
  record ""

  echo "  Lecturas:            $n"
  echo "  Media por lectura:   ${mean_ms} ms"
  echo "  p50: ${p50_ms} ms   p95: ${p95_ms} ms   p99: ${p99_ms} ms"
}

# =============================================================================
#  TEST 4 — Escalado del mesh (N shards)
# =============================================================================
mesh_test() {
  say "TEST 4: Escalado del mesh (escrituras paralelas por shard)"

  local shards=(1 4)
  for s in "${shards[@]}"; do
    local base="$WORK/mesh_${s}.db"
    rm -f "${base}"*
    AGORA_DB_SHARDS="$s" "$BIN" --db "$base" --seed >/dev/null 2>&1

    # Con N shards se lanzan N escrituras SIMULTÁNEAS (1 por shard, en paralelo)
    # y se repite para acumular muestras; el resultado muestra si los shards
    # permiten más escrituras concurrentes.
    local t_file="$WORK/mesh_${s}_times.txt"
    rm -f "$t_file"
    local rounds=$(( WRITES / s ))
    [[ "$rounds" -lt 1 ]] && rounds=1
    local pids=()
    for ((r=1; r<=rounds; r++)); do
      pids=()
      for ((i=1; i<=s; i++)); do
        local shard_file
        if [[ "$s" -gt 1 ]]; then
          shard_file="${base%.db}-$i.db"
        else
          shard_file="$base"
        fi
        (
          local st en
          st=$(date +%s%N)
          sqlite3 "$shard_file" "INSERT INTO posts (user_id, content, image_path, created_at)
                                 VALUES (1, 'mesh $s r$r i$i', '', datetime('now'));" 2>/dev/null
          en=$(date +%s%N)
          echo $(( en - st )) >> "$t_file"
        ) &
        pids+=($!)
      done
      for p in "${pids[@]}"; do wait "$p" 2>/dev/null; done
    done

    local n
    n=$(wc -l < "$t_file")
    local stats
    stats=$(sort -n "$t_file" | awk -v n="$n" '
      { arr[NR]=$1; s+=$1 }
      END {
        p95=arr[int(n*0.95+0.5)];
        printf "%.2f %.2f\n", (s/n)/1e6, p95/1e6;
      }')
    read -r mean_ms p95_ms <<< "$stats"

    record "TEST 4: Mesh de $s shard(s)"
    record "  Escrituras:          $n"
    record "  Media por escritura: ${mean_ms} ms"
    record "  p95:                 ${p95_ms} ms"
    record ""
    echo "  Mesh de $s shard(s) → media ${mean_ms} ms, p95 ${p95_ms} ms"
  done
}

# =============================================================================
# TEST 5 — Ráfaga real: todos los writers parten al mismo tiempo
# =============================================================================
burst_mesh_test() {
  say "TEST 5: Ráfaga simultánea — 1 vs 4 vs 16 shards"

  # Cada ronda libera todos los procesos a la vez mediante una barrera simple.
  # Así se mide la cola real de WAL, no una sucesión de INSERTs aislados.
  local workers=${BURST_WORKERS:-256}
  local rounds=${BURST_ROUNDS:-3}
  local shards=(${BURST_SHARDS:-1 4 16})

  for s in "${shards[@]}"; do
    local base="$WORK/burst_${s}.db"
    rm -f "${base}"*
    AGORA_DB_SHARDS="$s" "$BIN" --db "$base" --seed >/dev/null 2>&1 || fatal "No se pudo crear mesh de $s shards"

    local times_file="$WORK/burst_${s}_times.txt"
    local failures_file="$WORK/burst_${s}_failures.txt"
    rm -f "$times_file" "$failures_file"

    for ((round=1; round<=rounds; round++)); do
      local gate="$WORK/burst_gate_${s}_${round}"
      rm -f "$gate"
      local pids=()

      for ((i=0; i<workers; i++)); do
        local shard_idx=$(( i % s ))
        local shard_file="$base"
        if [[ "$shard_idx" -gt 0 ]]; then
          shard_file="${base%.db}-${shard_idx}.db"
        fi
        (
          while [[ ! -e "$gate" ]]; do :; done
          local start_ns end_ns
          start_ns=$(date +%s%N)
          if sqlite3 "$shard_file" "PRAGMA busy_timeout=5000; INSERT INTO posts (user_id, content, image_path, created_at) VALUES (1, 'burst $s/$round/$i', '', datetime('now'));" >/dev/null 2>&1; then
            end_ns=$(date +%s%N)
            echo $(( end_ns - start_ns )) >> "$times_file"
          else
            echo "locked" >> "$failures_file"
          fi
        ) &
        pids+=("$!")
      done

      # Todos los workers ya esperan; este touch los libera juntos.
      touch "$gate"
      for p in "${pids[@]}"; do wait "$p" 2>/dev/null; done
      rm -f "$gate"
    done

    local completed failed stats
    completed=$(wc -l < "$times_file" 2>/dev/null || echo 0)
    failed=0
    [[ -f "$failures_file" ]] && failed=$(wc -l < "$failures_file")
    stats=$(sort -n "$times_file" | awk -v n="$completed" '
      n == 0 { exit }
      { a[NR]=$1; sum+=$1 }
      END {
        printf "%.2f %.2f %.2f %.2f", sum/n/1e6, a[int(n*.50+.5)]/1e6, a[int(n*.95+.5)]/1e6, a[int(n*.99+.5)]/1e6
      }')
    local mean_ms=0 p50_ms=0 p95_ms=0 p99_ms=0
    [[ -n "$stats" ]] && read -r mean_ms p50_ms p95_ms p99_ms <<< "$stats"

    record "TEST 5: Ráfaga simultánea, $s shard(s)"
    record "  Writers por ráfaga:  $workers × $rounds rondas"
    record "  Completadas/fallidas: $completed/$failed"
    record "  Media/p50/p95/p99:  ${mean_ms}/${p50_ms}/${p95_ms}/${p99_ms} ms"
    record ""
    echo "  $s shard(s): $completed completadas, $failed fallidas | p50 ${p50_ms} ms · p95 ${p95_ms} ms · p99 ${p99_ms} ms"
  done
}

# =============================================================================
#  Reporte final
# =============================================================================
report() {
  say "RESULTADOS DE RENDIMIENTO — AGORA"
  hr
  echo "  Fecha:    $(date -Is)"
  echo "  Binario:  $BIN"
  echo "  DB:       $DB_PATH"
  echo "  CPU/RAM:  $(grep 'model name' /proc/cpuinfo | head -1 | cut -d: -f2 | xargs) / $(awk '/MemTotal/{printf "%.1f GB", $2/1024/1024}' /proc/meminfo)"
  hr
  echo ""
  printf '%s\n' "${RESULTS[@]}" | tee "$RESULT_FILE"
  echo ""
  hr
  echo "Resultados guardados en: $RESULT_FILE"
}

# ── Ejecución ────────────────────────────────────────────────────────────────
if [[ "${BURST_ONLY:-0}" == "1" ]]; then
  burst_mesh_test
else
  ram_test
  write_test
  read_test
  mesh_test
  burst_mesh_test
fi
report
