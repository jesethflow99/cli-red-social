#!/bin/bash
set -e

mkdir -p keys

KEY="keys/agora_host_key"
if [ ! -f "$KEY" ]; then
    ssh-keygen -t ed25519 -f "$KEY" -N "" -C "agora" -q
    echo "Generada: $KEY"
else
    echo "Ya existe: $KEY"
fi

echo ""
echo "✔ Listo. AGORA tiene su clave Ed25519."
echo ""
echo "Para desplegar:"
echo "  docker compose up -d"
echo ""
echo "El puerto público es ${SSH_PORT:-2222}."