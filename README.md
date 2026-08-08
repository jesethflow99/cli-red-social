# AGORA — Red Social Terminal-First sobre SSH

Red social minimalista orientada a privacidad. Sin navegador, sin JavaScript, sin cookies: solo tu terminal, SSH y texto.

```
ssh agora.social -t          # o vía Tor: torsocks ssh agora.onion -t
```

Al conectar, el servidor sirve una de dos interfaces según cómo esté desplegado
(ver [Interfaces](#interfaces)):

- **AGORA OpenTUI** (React + [OpenTUI](https://github.com/sst/opentui)): la que
  corre en el despliegue Docker por defecto.
- **TUI nativa** (Rust + Ratatui): fallback usado en desarrollo local o cuando
  no hay interfaz OpenTUI disponible.

---

## Inicio rápido

```bash
git clone <repo> && cd cli-red-social

# Generar keys de host (para multi-instancia)
./setup-keys.sh

# Configurar secretos (Docker Compose no arranca sin ellos)
cp .env.example .env
# Editá .env y reemplazá ambos secretos antes de continuar

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

# TUI nativa directo (sin SSH)
cargo run -- --tui

# Servidor SSH completo (sirve OpenTUI si AGORA_OPENTUI_ENTRY apunta a un
# archivo válido; si no, cae a la TUI nativa)
cargo run -- --port 2222

# Con logs a archivo
cargo run -- --port 2222 --log agora.log
```

## Interfaces

AGORA tiene dos interfaces de terminal que hablan con el mismo backend Rust y
la misma base de datos:

| | TUI nativa | AGORA OpenTUI |
|---|---|---|
| Código | `src/app.rs` (Ratatui) | `ui-opentui/` (React + OpenTUI, TypeScript) |
| Cuándo se usa | Siempre con `--tui`, o en SSH si `AGORA_OPENTUI_ENTRY` no está seteada | En SSH cuando `AGORA_OPENTUI_ENTRY` apunta a `ui-opentui/src/index.tsx` (así viene configurado en `docker-compose.yml`) |
| Comunicación con el backend | Llamadas directas a `db::Database` en el mismo proceso | Protocolo JSONL sobre stdin/stdout contra `agora --rpc` (`src/rpc.rs`) |

En cada conexión SSH, `src/ssh.rs` crea un pseudo-terminal (`forkpty`) y decide
cuál de las dos lanzar. Por eso, en el despliegue Docker (el del "Inicio
rápido") lo que ve el cliente SSH es **AGORA OpenTUI**, no la TUI nativa.

Para correr o desarrollar la interfaz OpenTUI de forma standalone, ver
[`ui-opentui/README.md`](ui-opentui/README.md).

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
| `REGISTRATION_MODE` | `open` | `open`, `invite` o `closed` (ver [Registro privado e invitaciones](#registro-privado-e-invitaciones)) |
| `AGORA_PUBLIC_HOST` | `localhost` | Host/IP público mostrado en el comando `scp` asistido |
| `AGORA_PUBLIC_SSH_PORT` | `2222` | Puerto público mostrado en el comando `scp` asistido |
| `AGORA_OPENTUI_ENTRY` | — | Ruta a `ui-opentui/src/index.tsx`; si apunta a un archivo existente, la sesión SSH sirve AGORA OpenTUI en vez de la TUI nativa |
| `SSH_CLIENT_IP` | — | IP del cliente (seteada por el servidor) |

## Uso

### Autenticación

- **Login:** `usuario:contraseña`
- **Registro:** `usuario:contraseña:nombre`

### Atajos principales

Estos son los de la **TUI nativa** (Ratatui). Si tu conexión sirve **AGORA
OpenTUI**, ver los atajos propios en [`ui-opentui/README.md`](ui-opentui/README.md#navegación).

| Tecla | Acción |
|---|---|
| `j`/`k` o flechas | Navegar |
| `Enter` | Ver post / seleccionar |
| `n` | Nuevo post |
| `Ctrl+P` | Subir imagen (modo recepción SCP) |
| `Ctrl+U` | Adjuntar imagen desde URL |
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

Desde la TUI, presioná `Ctrl+P` para entrar en modo recepción. El comando SCP se muestra en pantalla:

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

### Registro privado e invitaciones

`REGISTRATION_MODE` admite tres valores:

- `open`: registro libre.
- `invite`: exige una invitación de un solo uso (predeterminado en Docker).
- `closed`: desactiva nuevos registros.

```bash
# Crear una invitación válida durante 7 días
docker compose exec agora1 agora --invite-create --invite-days 7

# Consultar estado sin revelar códigos
docker compose exec agora1 agora --invite-list

# Revocar un código todavía no utilizado
docker compose exec agora1 agora --invite-revoke CODIGO
```

En modo `invite`, el registro usa el formato
`usuario:contraseña:nombre:invitación`. Los códigos se guardan como hashes y se
consumen atómicamente al crear la cuenta.

### Respaldo y restauración

```bash
# PostgreSQL, imágenes y sumas SHA-256
./scripts/backup.sh

# Reemplaza la base actual; requiere confirmación explícita
./scripts/restore.sh backups/agora-AAAAMMDDTHHMMSSZ --yes
```

La restauración detiene temporalmente las tres instancias de Agora. Conserva los
respaldos fuera del servidor y prueba periódicamente que puedan restaurarse.

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

# 3. Contraseñas seguras en un archivo local no versionado
cp .env.example .env
sed -i "s|replace-with-a-long-random-secret|$(openssl rand -base64 32)|" .env
sed -i "s|replace-with-a-different-long-random-secret|$(openssl rand -base64 32)|" .env

# 4. Desplegar (nginx + 3 instancias + PostgreSQL)
docker compose up -d

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
   │ago1 │ │2  │ │ago3 │   (3 instancias, forkpty por sesión)
   └──┬──┘ └┬──┘ └──┬──┘
      │     │       │        cada sesión sirve:
      │     │       │        · AGORA OpenTUI (Node/React) → rpc JSONL → agora --rpc
      │     │       │        · ó TUI nativa (Ratatui, llamadas directas)
      └──────┼───────┘
          ┌──▼──┐
          │ DB  │          (PostgreSQL)
          └─────┘
```

## Documentación

| Documento | Contenido |
|---|---|
| `MANUAL_TECNICO.md` | Arquitectura, seguridad, código, API |
| `ui-opentui/README.md` | Interfaz AGORA OpenTUI: cómo correrla y sus atajos |
| `ESCALABILIDAD.md` | Cómo soporta 500-1500 usuarios simultáneos |
| `SECURITY.md` | Política de reporte de vulnerabilidades y manejo de secretos |
| `plan.txt` | Filosofía y concepto original |

## Licencia

AGPL-3.0

---

## Donaciones ❤️

Si AGORA te gusta y querés apoyar el desarrollo:

[![PayPal](https://img.shields.io/badge/PayPal-Donar-blue?style=flat&logo=paypal)](https://paypal.me/ceggarr199)

**PayPal:** `ceggarr199@gmail.com`
