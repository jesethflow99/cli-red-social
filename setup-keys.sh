#!/bin/bash
set -e

mkdir -p keys

for i in 1 2 3; do
    KEY="keys/agora${i}_host_key"
    if [ ! -f "$KEY" ]; then
        ssh-keygen -t ed25519 -f "$KEY" -N "" -C "agora${i}" -q
        echo "Generada: $KEY"
    else
        echo "Ya existe: $KEY"
    fi
done

echo ""
echo "✔ Listo. Las 3 instancias tienen su clave Ed25519 única."
echo ""
echo "Para desplegar:"
echo "  docker compose up -d"
echo ""
echo "El puerto público es ${SSH_PORT:-2222} (nginx → 3 instancias internas)."
