# cli-red-social

Red social en la terminal, accesible vía SSH.

## Requisitos

- Docker y Docker Compose (recomendado)
- O Rust toolchain + PostgreSQL para ejecución local

## Inicio rápido (Docker)

```bash
# Clonar
git clone <repo> && cd cli-red-social

# Opcional: configurar contraseñas y puerto
export DB_PASSWORD=clave_segura
export SSH_PASSWORD=otra_clave
export SSH_PORT=2222  # Usá 22 en producción para ssh directo

# Levantar
docker compose up -d

# Conectarse
ssh localhost -p 2222 -t
# O si usaste puerto 22: ssh localhost -t
```

## Ejecución local

```bash
# Asegurate de tener PostgreSQL corriendo
cargo run -- --tui          # Modo local (sin SSH)
cargo run                   # Servidor SSH en puerto 2222
cargo run -- --port 22      # Servidor SSH en puerto 22
```

## Variables de entorno

| Variable | Default | Descripción |
|---|---|---|
| `DATABASE_URL` | `postgres://social:social@localhost/social` | Conexión a PostgreSQL |
| `SSH_PASSWORD` | `agora` | Contraseña SSH |
| `SSH_PORT` | `2222` | Puerto del servidor SSH |
| `DB_PASSWORD` | `agora` | Contraseña de PostgreSQL (solo Docker) |
| `RUST_LOG` | — | Nivel de logging (`info`, `debug`, etc.) |
| `TOR_PROXY` | `127.0.0.1:9050` | Proxy SOCKS5 para URLs .onion |
| `LANG` | `es` | Idioma (`es` o `en`) |

## Uso

### Autenticación

- **Login:** `usuario:contraseña`
- **Registro:** `usuario:contraseña:nombre`

### Teclas

| Tecla | Acción |
|---|---|---|
| `j` / `k` o flechas | Navegar |
| `Enter` | Ver post / seleccionar |
| `n` | Nuevo post |
| `/` | Buscar posts (Tab: filtrar por fecha) |
| `c` | Comentar |
| `s` | Buscar usuarios |
| `p` | Mi perfil |
| `m` | Mensajes directos |
| `Ctrl+n` | Notificaciones |
| `e` | Editar post / perfil |
| `i` | Ver imagen adjunta |
| `f` | Seguir / dejar de seguir |
| `Tab` | Cambiar entre login y registro / cambiar filtro |
| `Esc` o `q` | Volver / salir |

## Producción en VPS

```bash
# Puerto 22 para ssh directo (sin -p)
export SSH_PORT=22
export SSH_PASSWORD=muylarga_y_segura
export DB_PASSWORD=otra_muy_segura

# Firewall
sudo ./firewall.sh

# Iniciar
SSH_PORT=22 SSH_PASSWORD=muylarga_y_segura DB_PASSWORD=otra_muy_segura docker compose up -d

# Conectarse directo (el username de SSH no importa)
ssh tu-servidor.com -t
```

> **Nota:** Si tu VPS ya usa el puerto 22 para OpenSSH, usá otro puerto (2222, 8022, etc.) o pará el sshd existente primero.

## Licencia

AGPL-3.0 — Cualquiera puede usar, modificar y distribuir el código,
pero si lo servís en una red pública, tenés que publicar los cambios.

## Arquitectura

```
┌──────────────┐     ┌──────────────┐     ┌────────────┐
│ SSH Client   │────▶│ SSH Server   │────▶│ Ratatui    │
│ (PuTTY, etc) │     │ (russh:2222) │     │ TUI        │
└──────────────┘     └──────┬───────┘     └────────────┘
                            │ forkpty
                    ┌───────┴───────┐
                    │ PostgreSQL    │
                    └───────────────┘
```
