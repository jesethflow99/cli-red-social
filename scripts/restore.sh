#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || "$2" != "--yes" ]]; then
  echo "Uso: $0 <directorio-backup> --yes" >&2
  echo "La restauración reemplaza los datos actuales de PostgreSQL." >&2
  exit 2
fi

backup_dir="${1%/}"
dump_file="$backup_dir/database.dump"

if [[ ! -f "$dump_file" || ! -f "$backup_dir/SHA256SUMS" ]]; then
  echo "El respaldo no contiene database.dump y SHA256SUMS." >&2
  exit 1
fi

(
  cd "$backup_dir"
  sha256sum --check SHA256SUMS
)

echo "Deteniendo instancias de Agora durante la restauración..."
docker compose stop agora1 agora2 agora3

restart_agora() {
  docker compose start agora1 agora2 agora3 >/dev/null
}
trap restart_agora EXIT

echo "Restaurando PostgreSQL..."
docker compose exec -T db pg_restore \
  --username social \
  --dbname social \
  --clean \
  --if-exists \
  --no-owner \
  --no-privileges < "$dump_file"

if [[ -f "$backup_dir/uploads.tar.gz" ]]; then
  echo "Restaurando imágenes..."
  mkdir -p data
  tar -C data -xzf "$backup_dir/uploads.tar.gz" --overwrite
fi

echo "Reiniciando Agora..."
docker compose start agora1 agora2 agora3
trap - EXIT
echo "Restauración completada."
