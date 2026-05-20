# AGORA — Red Social Terminal-First sobre SSH

Red social minimalista orientada a privacidad. Sin navegador, sin JavaScript, sin cookies: solo tu terminal, SSH y texto.

```
ssh agora.social -t          # o vía Tor: torsocks ssh agora.onion -t
```

---

## Inicio rápido

```bash
git clone <repo> && cd cli-red-social

# Generar keys de host (para multi-instancia)
./setup-keys.sh

# Opcional: configurar contraseñas
export DB_PASSWORD=clave_segura
export SSH_PASSWORD=otra_clave
export SSH_PORT=2222

# Levantar (nginx + 3 instancias + PostgreSQL)
docker compose up -d

# Conectarse
ssh localhost -p 2222 -t
```

## Desarrollo local

```bash
# Solo PostgreSQL en Docker
docker compose up -d db

# Seed de datos (52 usuarios, 250+ posts)
cargo run -- --seed

# TUI directo (sin SSH)
cargo run -- --tui

# Servidor SSH completo
cargo run -- --port 2222

# Con logs a archivo
cargo run -- --port 2222 --log agora.log
```

## Variables de entorno

| Variable | Default | Descripción |
|---|---|---|
| `DATABASE_URL` | `postgres://social:agora@localhost/social` | Conexión PostgreSQL |
| `SSH_PASSWORD` | `agora` | Contraseña SSH compartida |
| `SSH_PORT` | `2222` | Puerto público |
| `DB_PASSWORD` | `agora` | Contraseña PostgreSQL (Docker) |
| `RUST_LOG` | `info` | Nivel de logging |
| `LANG` | `es` | Idioma (`es` o `en`) |
| `AGORA_UPLOAD_DIR` | `./uploads` o `/data/uploads` | Directorio de imágenes |
| `AGORA_MODERATION_PLUGINS` | — | Plugins: `spam,profanity,link` |
| `SSH_CLIENT_IP` | — | IP del cliente (seteada por el servidor) |

## Uso

### Autenticación

- **Login:** `usuario:contraseña`
- **Registro:** `usuario:contraseña:nombre`

### Atajos principales

| Tecla | Acción |
|---|---|
| `j`/`k` o flechas | Navegar |
| `Enter` | Ver post / seleccionar |
| `n` | Nuevo post |
| `Ctrl+U` | Subir imagen (modo recepción SCP) |
| `Ctrl+L` | Adjuntar imagen desde URL |
| `/` | Buscar posts |
| `#` | Trending hashtags |
| `R` | Modo Radio (ticker de hashtags) |
| `s` | Buscar usuarios |
| `p` | Mi perfil |
| `E` | Exportar datos (JSON) |
| `m` | Mensajes directos |
| `Ctrl+N` | Notificaciones |
| `i` | Ver imagen |
| `d` | Descargar imagen / Borrar imagen en upload |
| `D` | Eliminar post (confirma con `y/n`) |
| `f` | Seguir / dejar de seguir |
| `e` | Editar post / perfil |
| `Ctrl+Q` | Salir |
| `Tab` | Cambiar login/registro / cambiar filtro |

### Subir imágenes (SCP)

Desde la TUI, presioná `Ctrl+U` para entrar en modo recepción. El comando SCP se muestra en pantalla:

```bash
scp -P 2222 archivo.jpg localhost:jeseth/archivo.jpg
```

- La imagen se valida, se limpia de metadata y se convierte a JPEG (máx 512px)
- Se guarda en `uploads/jeseth/`
- Solo se permite subir al directorio de tu usuario de sesión
- Las imágenes se detectan automáticamente en la TUI

### Exportar datos

```bash
# CLI
cargo run -- --export --user jeseth --format json

# TUI: desde tu perfil, presioná E
```

Genera `export_jeseth_20260520_120000.json` con posts, comentarios, mensajes, seguidores.

### Plugins de moderación

```bash
AGORA_MODERATION_PLUGINS=spam,profanity,link cargo run -- --port 2222
```

- `spam`: bloquea mayúsculas excesivas y caracteres repetidos
- `profanity`: bloquea palabras configuradas
- `link`: bloquea URLs sospechosas

## Producción

```bash
# 1. Generar claves
./setup-keys.sh

# 2. Firewall
sudo ./firewall.sh

# 3. Contraseñas seguras
export SSH_PASSWORD="$(openssl rand -base64 32)"
export DB_PASSWORD="$(openssl rand -base64 32)"

# 4. Desplegar (nginx + 3 instancias + PostgreSQL)
SSH_PASSWORD="$SSH_PASSWORD" DB_PASSWORD="$DB_PASSWORD" docker compose up -d

# 5. Seed de datos
docker compose exec agora1 agora --seed
```

## Arquitectura

```
     ┌──────────────┐
     │ SSH Client   │  ssh -p 2222
     └──────┬───────┘
            │
     ┌──────▼──────┐
     │ nginx:2222   │  (TCP stream proxy)
     └──┬────┬────┬─┘
        │    │    │
   ┌────▼┐ ┌▼──┐ ┌▼────┐
   │ago1 │ │2  │ │ago3 │   (3 instancias)
   └──┬──┘ └┬──┘ └──┬──┘
      └──────┼───────┘
          ┌──▼──┐
          │ DB  │          (PostgreSQL)
          └─────┘
```

## Documentación

| Documento | Contenido |
|---|---|
| `MANUAL_TECNICO.md` | Arquitectura, seguridad, código, API |
| `ESCALABILIDAD.md` | Cómo soporta 500-1500 usuarios simultáneos |
| `plan.txt` | Filosofía y concepto original |

## Licencia

AGPL-3.0

---

## Donaciones ❤️

Si AGORA te gusta y querés apoyar el desarrollo:

[![PayPal](https://img.shields.io/badge/PayPal-Donar-blue?style=flat&logo=paypal)](https://paypal.me/ceggarr199)

**PayPal:** `ceggarr199@gmail.com`
