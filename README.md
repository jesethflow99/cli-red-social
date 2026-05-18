# cli-red-social

Red social en la terminal, accesible vía SSH.

## Requisitos

- Docker y Docker Compose (recomendado)
- O Rust toolchain + PostgreSQL para ejecución local

## Inicio rápido (Docker)

```bash
# Clonar
git clone <repo> && cd cli-red-social

# Opcional: configurar contraseñas
export DB_PASSWORD=clave_segura
export SSH_PASSWORD=otra_clave

# Levantar
docker compose up -d

# Conectarse
ssh localhost -p 2222 -t
```

## Ejecución local

```bash
# Asegurate de tener PostgreSQL corriendo
cargo run -- --tui          # Modo local (sin SSH)
cargo run                   # Servidor SSH en puerto 2222
```

## Variables de entorno

| Variable | Default | Descripción |
|---|---|---|
| `DATABASE_URL` | `postgres://social:social@localhost/social` | Conexión a PostgreSQL |
| `SSH_PASSWORD` | (vacío) | Contraseña SSH. Vacío = acepta cualquier password |
| `DB_PASSWORD` | `social` | Contraseña de PostgreSQL (solo Docker) |
| `RUST_LOG` | — | Nivel de logging (`info`, `debug`, etc.) |

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

## Seguridad para producción

```bash
# Contraseña SSH obligatoria
export SSH_PASSWORD=muylarga_y_segura

# Contraseña de base de datos
export DB_PASSWORD=otra_muy_segura

# Firewall
sudo ./firewall.sh

# Iniciar
SSH_PASSWORD=... DB_PASSWORD=... docker compose up -d
```

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
