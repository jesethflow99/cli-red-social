#!/bin/bash
set -e

# ─── Config ───────────────────────────────────────────────
SSH_PORT=2222          # Puerto de la red social
SERVER_SSH_PORT=22     # Puerto SSH tradicional para administrar el servidor
# ───────────────────────────────────────────────────────────

echo "Aplicando reglas de iptables..."

# Limpiar reglas existentes
iptables -F
iptables -X
iptables -t nat -F
iptables -t mangle -F

# Política por defecto: denegar todo
iptables -P INPUT DROP
iptables -P FORWARD DROP
iptables -P OUTPUT ACCEPT

# Permitir tráfico local (loopback)
iptables -A INPUT -i lo -j ACCEPT

# Permitir conexiones establecidas y relacionadas
iptables -A INPUT -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT

# Permitir SSH tradicional para administración del servidor
iptables -A INPUT -p tcp --dport $SERVER_SSH_PORT -j ACCEPT

# Permitir la red social por SSH
iptables -A INPUT -p tcp --dport $SSH_PORT -j ACCEPT

# Rate-limit conexiones nuevas al puerto de la red social
iptables -A INPUT -p tcp --dport $SSH_PORT -m conntrack --ctstate NEW \
  -m limit --limit 10/minute --limit-burst 5 -j ACCEPT
iptables -A INPUT -p tcp --dport $SSH_PORT -m conntrack --ctstate NEW \
  -j DROP

# Rate-limit conexiones nuevas al SSH tradicional
iptables -A INPUT -p tcp --dport $SERVER_SSH_PORT -m conntrack --ctstate NEW \
  -m limit --limit 5/minute --limit-burst 3 -j ACCEPT
iptables -A INPUT -p tcp --dport $SERVER_SSH_PORT -m conntrack --ctstate NEW \
  -j DROP

# Protección contra escaneo de puertos (SYN flood)
iptables -A INPUT -p tcp --syn -m limit --limit 1/s --limit-burst 3 -j ACCEPT
iptables -A INPUT -p tcp --syn -j DROP

# Permitir ICMP (ping) con rate-limit
iptables -A INPUT -p icmp --icmp-type echo-request -m limit --limit 1/s -j ACCEPT
iptables -A INPUT -p icmp --icmp-type echo-request -j DROP

# Log de paquetes denegados (opcional, comentar si llena los logs)
iptables -A INPUT -m limit --limit 5/min -j LOG --log-prefix "[FW BLOCKED] " --log-level 4

echo "✔ Reglas aplicadas correctamente."
echo ""
echo "Puertos abiertos:"
echo "  - $SERVER_SSH_PORT (SSH administración, con rate-limit)"
echo "  - $SSH_PORT       (Red social, con rate-limit)"
echo ""
echo "Para hacer persistente:"
echo "  Debian/Ubuntu: apt install iptables-persistent"
echo "  o: iptables-save > /etc/iptables/rules.v4"
