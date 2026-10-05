# Escalabilidad — AGORA

Cómo AGORA soporta cientos de usuarios simultáneos con recursos mínimos.

AGORA está diseñado con una premisa simple: **un proceso por usuario en lugar de
un servidor web compartido**. Cada conexión SSH crea un proceso hijo
independiente con su propia pseudo-terminal, sin locks compartidos ni loop de
eventos único. La persistencia es **SQLite embebido** (WAL): lecturas
concurrentes ilimitadas y un escritor encolado por archivo. Para picos de
escritura sostenidos, AGORA puede repartir los datos en varios archivos `.db`
(un *mesh* de shards) y condensarlos en uno solo cuando baja la carga.

> **Nota:** AGORA sirve una sola interfaz, la TUI nativa (Ratatui), que corre en
> el proceso hijo de cada sesión. Las cifras de este documento corresponden a
> esa interfaz. Ver el [Resumen de capacidad](#resumen-de-capacidad).

---

## Resumen de capacidad

| Componente | Límite | Cuello de botella |
|---|---|---|
| SQLite (WAL) | Un escritor por archivo | Lecturas ilimitadas en paralelo |
| Shards (mesh) | N archivos `.db` | N escritores en paralelo |
| Procesos (forkpty) | ~500 antes de ulimit | Unas decenas sin problema |
| RAM | ~8 MB/sesión (TUI nativa) — medido en contenedor | 100 usuarios = 800 MB |
| nftables | 10 conexiones/minuto por IP | Rate-limit a nivel firewall |

**Capacidad realista**: 500-1000 usuarios simultáneos en un VPS de 4 GB RAM / 2 vCPU con un archivo único.

Con el mesh de 4 shards: ~2000 usuarios.

> **Cómo se llegó a estas cifras:** en el contenedor, cada sesión de la TUI
> nativa usa ~8 MB de RAM (`/proc/<pid>/status`, `VmRSS`) y ~0% de CPU en
> idle. Un VPS de 4 GB (tras reservar ~1 GB para SO + procesos base de Agora)
> deja ~3 GB útiles → **~500-1000 sesiones** antes de que el cuello de botella
> pase a ser la CPU o el escritor único de SQLite, no la RAM.

---

## 1. ¿Por qué AGORA escala tan bien?

### 1.1 Sin HTTP, sin websockets

Cada usuario es **un proceso hijo** (`forkpty`) conectado por SSH. No hay loop de eventos compartido, no hay polling HTTP, no hay websockets que mantengan conexiones abiertas en segundo plano.

- Un servidor web típico con 1000 conexiones WebSocket consume cientos de MB solo en buffers de red.
- AGORA con 1000 usuarios consume ~5 GB de RAM distribuidos entre procesos independientes del kernel.

### 1.2 SSH comprime

El protocolo SSH comprime el tráfico por defecto. Una pantalla de TUI (80×24 caracteres) son ~2 KB sin comprimir, ~500 bytes comprimidos. Cada interacción del usuario genera una ráfaga de ~1 KB.

### 1.3 Base de datos: operaciones cortas

| Operación | Duración típica |
|---|---|
| `get_timeline` (50 posts con JOIN) | 2-5 ms |
| `authenticate` (bcrypt verify) | 100-300 ms |
| `create_post` (INSERT + hashtags) | 1-3 ms |
| `search_posts` (ILIKE) | 5-20 ms |
| `send_message` (INSERT) | 0.5-1 ms |

El 99% del tiempo los usuarios están **leyendo**, no escribiendo. Las consultas duran milisegundos y SQLite en WAL permite lecturas concurrentes sin bloqueos.

---

## 2. SQLite en WAL: lecturas paralelas, una escritura encolada

```
┌──────────────────────────────────────┐
│  SQLite WAL (agora.db)               │
│                                      │
│  Lectores  ████████████████████  ∞    │  ← lecturas simultáneas
│  Escritor  ████                    1 │  ← un escritor a la vez
│                                      │
│  Cola de escrituras (busy_timeout):  │
│  [write2] [write3] [write4] ...      │  ← se encolan y se aplican en orden
└──────────────────────────────────────┘
```

### ¿Qué pasa si llegan muchas escrituras a la vez?

SQLite serializa las escrituras: las demás esperan en una cola con
`busy_timeout` (5 s por defecto). Para **comunicación humana** esto es
imperceptible: un post o mensaje entrante espera milisegundos, y aun con cientos
de escrituras simultáneas el retraso es de centenas de milisegundos como máximo.

En la práctica casi nunca hay contención porque:
- Las consultas duran milisegundos
- La mayoría de usuarios están **leyendo** (timeline, perfiles), no escribiendo
- Las escrituras reales (posts, comentarios, mensajes) son escasas por usuario

### ¿Cómo escalar más? El mesh de shards

Si querés paralelizar las escrituras, repartí los datos en N archivos `.db`:

```bash
AGORA_DB_SHARDS=4 cargo run -- --port 2222   # agora.db, agora-1.db, ...
```

- Cada shard tiene **su propio escritor** → N escrituras en paralelo.
- Cada usuario se asigna a un shard por **hash de su username**.
- Las consultas que cruzan usuarios (timeline, búsqueda, trending) hacen
  **fan-out** a todos los shards y combinan los resultados.
- Los ids son globales (`shard_idx * 10.000.000 + local`), así que un id se
  puede enrutar a su shard sin tablas de routing.

Cuando baja la carga, condensá el mesh en un único archivo:

```bash
AGORA_DB_SHARDS=4 cargo run -- --condense   # fusiona todo en agora.db
```

### ¿Cuándo usar el mesh?

| Escenario | Recomendado |
|---|---|
| Hasta ~1000 usuarios | Archivo único (`AGORA_DB_SHARDS=1`) |
| Picos de escritura sostenidos | Mesh de 4-16 shards |
| Backups / migración | Condensar primero, luego copiar |

> **Regla práctica:** el mesh multiplica el *throughput de escritura* (1 escritor
> por archivo), no la RAM. Si el cuello de botella es la CPU o la RAM de las
> sesiones, más shards no ayudan; subí el VPS.

---

## 3. Procesos por usuario

```
Cada conexión SSH → forkpty() → proceso hijo con TUI
                                ~8 MB RAM
                                ~0% CPU en idle
                                ~2% CPU en interacción
```

### ¿Por qué procesos y no threads?

- **Aislamiento total**: si un proceso TUI crashea, no afecta a los demás
- **Seguridad**: cada usuario en su propio espacio de memoria
- **Simplicidad**: el kernel maneja el scheduling, no necesitamos un runtime async para la UI
- **Rust no paga garbage collection**: el proceso se destruye limpiamente al desconectar

### Límite de procesos

```bash
ulimit -u   # Límite de procesos del usuario
```

En Linux típico: 1024-4096 procesos por usuario. Con `forkpty`, cada sesión ocupa 1 proceso hijo. Subir el límite:

```bash
ulimit -u 8192
```

---

## 4. Firewall (nftables)

```
Tabla: agora_fw
├── input chain (policy: drop)
│   ├── loopback → accept
│   ├── established/related → accept
│   ├── puerto SSH admin (22) → rate-limit 5/min
│   ├── puerto red social (2222) → rate-limit 10/min por IP
│   ├── SYN flood protection → 5/s
│   └── ICMP → 1/s
```

El rate-limit a nivel firewall protege contra:
- Fuerza bruta SSH
- DDoS básico
- Escaneo de puertos

---

## 5. Mesh de shards (en lugar de multi-instancia)

En la arquitectura anterior (PostgreSQL) se escalaba con **más instancias** de
Agora + nginx. Con SQLite embebido, una sola instancia ya aprovecha todo el VPS;
para más throughput de escritura se usa el **mesh de archivos**, no más procesos:

```
agora.db      ← shard 0 (base)
agora-1.db    ← shard 1
agora-2.db    ← shard 2
agora-3.db    ← shard 3
```

### Cómo funciona el routing

- Cada usuario se asigna a un shard por **hash de su username**.
- El id global codifica el shard: `id = shard_idx * 10.000.000 + local`.
- Las consultas de un solo usuario van **directo** a su shard.
- Las que cruzan usuarios (timeline, búsqueda, trending, mensajes) hacen
  **fan-out** a todos los shards y combinan resultados en memoria.

### Capacidad según shards

| Recurso | 1 archivo | 4 shards | 16 shards |
|---|---|---|---|
| Escritores en paralelo | 1 | 4 | 16 |
| Usuarios simultáneos | ~500-1000 | ~2000 | ~5000+ |
| RAM | ~3 GB | ~3 GB | ~3 GB (misma) |
| CPU | 2 cores | 2-4 cores | 4+ cores |

> El mesh multiplica el **throughput de escritura**, no la RAM. Cada shard es un
> archivo `.db` separado; los shards comparten el mismo binario y el mismo
> proceso. La condensación (`--condense`) los fusiona en uno solo.

---

## 6. Limpieza automática

Un hilo en segundo plano ejecuta cada 24 horas:

```
cleanup_old_data(90 días):
  - DELETE mensajes > 90 días
  - DELETE notificaciones > 90 días
  - DELETE rate_limits viejos

cleanup_inactive_users(730 días):
  - DELETE usuarios sin login en 2 años
  - CASCADE: borra posts, comentarios, follows, mensajes
```

Esto evita que la base de datos crezca indefinidamente.

---

## 7. Optimizaciones para producción

### SQLite

Los índices ya se crean automáticamente en `init_schema`. Para cargas muy
pesadas de búsqueda se puede agregar un FTS5 (full-text search):

```sql
-- FTS5 para búsqueda de posts (opcional)
CREATE VIRTUAL TABLE IF NOT EXISTS posts_fts USING fts5(content);
-- Sincronizar manualmente o vía trigger
```

El modo WAL ya está activado por defecto (`PRAGMA journal_mode=WAL`), que da
lecturas concurrentes sin bloqueos. El `busy_timeout` (5 s) encola las
escrituras.

### Sistema operativo

```bash
# Aumentar file descriptors
echo "fs.file-max = 65536" >> /etc/sysctl.conf
echo "* soft nofile 65536" >> /etc/security/limits.conf
echo "* hard nofile 65536" >> /etc/security/limits.conf

# Aumentar procesos
echo "* soft nproc 8192" >> /etc/security/limits.conf
```

### Rust

Compilar con optimizaciones agresivas:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

---

## 8. Monitoreo

```bash
# Conexiones SSH activas
ss -tnp | grep 2222 | wc -l

# Procesos AGORA
ps aux | grep agora | wc -l

# Tamaño de los archivos SQLite
ls -lh agora*.db

# Uso de WAL (archivos pendientes de checkpoint)
ls -lh agora.db-wal 2>/dev/null

# Logs
tail -f agora.log
```

---

## 9. Resumen: ¿cuánto escala?

| Configuración | Usuarios simultáneos | RAM necesaria | Costo mensual VPS |
|---|---|---|---|
| Archivo único SQLite | 500-1000 | 2-4 GB | $10-20 |
| Mesh de 4 shards | ~2000 | 4-8 GB | $20-40 |
| Mesh de 16 shards | ~5000+ | 8-16 GB | $40-80 |

> Los límites asumen la TUI nativa (~8 MB/sesión). El mesh multiplica el
> **throughput de escritura** (un escritor por archivo); la RAM la pone el VPS
> (ver [Mesh de shards](#5-mesh-de-shards-en-lugar-de-multi-instancia)).

AGORA está diseñado para hacer lo máximo con lo mínimo: un binario de ~12 MB con
SQLite embebido, un proceso por usuario, y un mesh opcional de archivos que se
expande bajo carga y se condensa en uno solo cuando baja.
