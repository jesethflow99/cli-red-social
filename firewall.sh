#!/bin/bash
set -e

# ─── Config ───────────────────────────────────────────────
SSH_PORT=2222          # Puerto de la red social
SERVER_SSH_PORT=22     # Puerto SSH tradicional para administrar el servidor
TABLE_NAME="agora_fw"
# ───────────────────────────────────────────────────────────

echo "Aplicando reglas de nftables..."

# Cargar la tabla si ya existe (para poder flushearla)
nft list table inet "$TABLE_NAME" &>/dev/null && nft delete table inet "$TABLE_NAME" || true

nft -f - <<EOF
table inet ${TABLE_NAME} {
    # ── Sets ──────────────────────────────────────────
    set scp_allowed {
        type ipv4_addr
        flags timeout
        timeout 10m
    }

    set ssh_ratelimit_v4 {
        type ipv4_addr
        flags dynamic,timeout
        timeout 1m
        size 65535
    }

    # ── Chains ────────────────────────────────────────
    chain input {
        type filter hook input priority filter; policy drop;

        # Loopback
        iif lo accept

        # Conexiones establecidas/relacionadas
        ct state established,related accept

        # SSH administración (con rate-limit)
        tcp dport ${SERVER_SSH_PORT} ct state new \
            add @ssh_ratelimit_v4 { ip saddr limit rate over 5/minute } \
            log prefix "[FW ADMIN BLOCK] " drop
        tcp dport ${SERVER_SSH_PORT} accept

        # Red social (con rate-limit)
        tcp dport ${SSH_PORT} ct state new \
            add @ssh_ratelimit_v4 { ip saddr limit rate over 10/minute } \
            log prefix "[FW AGORA BLOCK] " drop
        tcp dport ${SSH_PORT} accept

        # SCP temporal: solo IPs en el set scp_allowed (upload/download efímero)
        tcp dport ${SSH_PORT} ip saddr @scp_allowed accept

        # Protección SYN flood
        tcp flags syn ct state new limit rate 5/second accept
        tcp flags syn ct state new drop

        # ICMP (ping) con rate-limit
        icmp type echo-request limit rate 1/second accept
        icmp type echo-request drop

        # Log de paquetes denegados
        limit rate 5/minute log prefix "[FW BLOCKED] "
    }

    chain forward {
        type filter hook forward priority filter; policy drop;
    }

    chain output {
        type filter hook output priority filter; policy accept;
    }
}
EOF

echo "✔ Reglas nftables aplicadas correctamente."
echo ""
echo "Puertos abiertos:"
echo "  - $SERVER_SSH_PORT (SSH administración, rate-limit 5/min)"
echo "  - $SSH_PORT       (Red social, rate-limit 10/min)"
echo ""
echo "SCP temporal:"
echo "  - nft add element inet ${TABLE_NAME} scp_allowed { <IP> }"
echo "  - nft delete element inet ${TABLE_NAME} scp_allowed { <IP> }"
echo ""
echo "Para hacer persistente:"
echo "  Debian/Ubuntu: apt install nftables && systemctl enable nftables"
echo "  Guardar reglas: nft list ruleset > /etc/nftables.conf"
