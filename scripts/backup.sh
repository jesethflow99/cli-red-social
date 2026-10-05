#!/usr/bin/env bash
set -euo pipefail

backup_root="${1:-./backups}"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
backup_dir="${backup_root%/}/agora-${timestamp}"

mkdir -p "$backup_dir"

echo "Copiando base de datos SQLite..."
cp data/agora.db "$backup_dir/agora.db"

if [[ -d data/uploads ]]; then
  echo "Creando respaldo de imágenes..."
  tar -C data -czf "$backup_dir/uploads.tar.gz" uploads
fi

(
  cd "$backup_dir"
  sha256sum agora.db > SHA256SUMS
  if [[ -f uploads.tar.gz ]]; then
    sha256sum uploads.tar.gz >> SHA256SUMS
  fi
)

echo "Respaldo creado en: $backup_dir"