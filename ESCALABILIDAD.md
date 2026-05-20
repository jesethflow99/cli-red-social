# Escalabilidad — AGORA

Cómo AGORA soporta cientos de usuarios simultáneos con recursos mínimos.

---

## Resumen de capacidad

| Componente | Límite | Cuello de botella |
|---|---|---|
| Pool PostgreSQL | 25 conexiones | ~500-1000 usuarios simultáneos |
| Procesos (forkpty) | ~500 antes de ulimit | Unas decenas sin problema |
| RAM | ~5 MB por sesión TUI | 100 usuarios = 500 MB |
| nftables | 10 conexiones/minuto por IP | Rate-limit a nivel firewall |
| nginx (stream) | I/O bound, virtualmente ilimitado | Balancea entre instancias |

**Capacidad realista**: 300-500 usuarios simultáneos en un VPS de 4 GB RAM / 2 vCPU.

Con 3 instancias + nginx: ~1500 usuarios.

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

El 99% del tiempo los usuarios están **leyendo**, no escribiendo. Las consultas duran milisegundos y la conexión se libera inmediatamente al pool.

---

## 2. Pool de conexiones PostgreSQL

```
┌────────────────────────────────────┐
│  r2d2 connection pool (25)         │
│                                    │
│  ████████████████████████████████  │ ← 25 conexiones activas
│                                    │
│  Cola de espera (timeout 3s):      │
│  [req26] [req27] [req28] ...       │ ← si las 25 están ocupadas
└────────────────────────────────────┘
```

### ¿Qué pasa si las 25 están ocupadas?

Las solicitudes se **encolan automáticamente** en `r2d2`. Cada solicitud espera hasta 3 segundos (configurable) a que se libere una conexión. Si en 3s no se libera, se devuelve error.

En la práctica, esto casi nunca ocurre porque:
- Las consultas duran milisegundos
- La mayoría de usuarios no consultan la DB simultáneamente
- El patrón de uso es ráfagas cortas (cada vez que el usuario presiona una tecla)

### ¿Cómo escalar el pool?

```rust
// db.rs
.max_size(50)  // Subir de 25 a 50
```

PostgreSQL maneja cientos de conexiones sin problema. El límite real es la RAM y `max_connections` en `postgresql.conf`.

---

## 3. Procesos por usuario

```
Cada conexión SSH → forkpty() → proceso hijo con TUI
                                ~5 MB RAM
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

## 5. Multi-instancia con nginx (TCP stream)

```
          ┌──────────┐
usuario → │ nginx    │ :2222
          │ (stream) │
          └─┬──┬──┬──┘
            │  │  │
       ┌────▼┐ ┌▼──┐ ┌▼────┐
       │ago1 │ │2  │ │ago3 │   (3 binarios AGORA)
       └──┬──┘ └┬──┘ └──┬──┘
          └──────┼───────┘
              ┌──▼──┐
              │ DB  │          (PostgreSQL compartido)
              └─────┘
```

### Cómo funciona

Nginx actúa como proxy TCP (capa 4). No interpreta SSH, solo reenvía bytes. El balanceo usa `hash $remote_addr consistent`:

```nginx
upstream agora_backend {
    hash $remote_addr consistent;  # Misma IP → misma instancia
    server agora1:2222;
    server agora2:2222;
    server agora3:2222;
}
```

- Misma IP siempre cae en la misma instancia → sesión TUI persistente
- Si una instancia se cae, nginx reintenta en otra
- Las instancias comparten PostgreSQL y volumen de uploads

### Capacidad con 3 instancias

| Recurso | 1 instancia | 3 instancias |
|---|---|---|
| Pool DB total | 25 | 75 (25 c/u) |
| Usuarios simultáneos | ~500 | ~1500 |
| RAM | ~2.5 GB | ~7.5 GB |
| CPU | 2 cores | 6 cores |

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

### PostgreSQL

```sql
-- Índices adicionales para búsquedas frecuentes
CREATE INDEX idx_posts_created_at ON posts(created_at DESC);
CREATE INDEX idx_messages_receiver ON messages(receiver_id, created_at);
CREATE INDEX idx_notifications_user ON notifications(user_id, created_at DESC);

-- Aumentar conexiones máximas
ALTER SYSTEM SET max_connections = 200;
SELECT pg_reload_conf();
```

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
# Conexiones activas
ss -tnp | grep 2222 | wc -l

# Procesos AGORA
ps aux | grep agora | wc -l

# Conexiones PostgreSQL
psql -c "SELECT count(*) FROM pg_stat_activity WHERE datname='social';"

# Logs
tail -f agora.log
```

---

## 9. Resumen: ¿cuánto escala?

| Configuración | Usuarios simultáneos | RAM necesaria | Costo mensual VPS |
|---|---|---|---|
| 1 instancia, 25 pool DB | 300-500 | 2-4 GB | $10-20 |
| 3 instancias + nginx, 75 pool DB | 1000-1500 | 8-16 GB | $40-80 |
| 10 instancias + balanceador dedicado | 5000+ | 32+ GB | $200+ |

AGORA está diseñado para **escalar horizontalmente**: agregar más instancias no requiere cambios de arquitectura, solo más procesos apuntando a la misma base de datos.
