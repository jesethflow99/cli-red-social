#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || "$2" != "--yes" ]]; then
  echo "Uso: $0 <directorio-backup> --yes" >&2
  echo "La restauración reemplaza los datos actuales de SQLite." >&2
  exit 2
fi

backup_dir="${1%/}"
db_file="$backup_dir/agora.db"

if [[ ! -f "$db_file" || ! -f "$backup_dir/SHA256SUMS" ]]; then
  echo "El respaldo no contiene agora.db y SHA256SUMS." >&2
  exit 1
fi

(
  cd "$backup_dir"
  sha256sum --check SHA256SUMS
)

echo "Deteniendo instancias de Agora durante la restauración..."
docker compose stop agora

restart_agora() {
  docker compose start agora >/dev/null
}
trap restart_agora EXIT

echo "Restaurando base de datos SQLite..."
rm -f data/agora.db data/agora.db-wal data/agora.db-shm
cp "$db_file" data/agora.db

if [[ -f "$backup_dir/uploads.tar.gz" ]]; then
  echo "Restaurando imágenes..."
  mkdir -p data
  tar -C data -xzf "$backup_dir/uploads.tar.gz" --overwrite
fi

echo "Reiniciando Agora..."
docker compose start agora
trap - EXIT
echo "Restauración completada."