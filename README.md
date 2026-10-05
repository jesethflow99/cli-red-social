# AGORA — Red Social Terminal-First sobre SSH

Red social minimalista orientada a privacidad. Sin navegador, sin JavaScript, sin cookies: solo tu terminal, SSH y texto.

AGORA es una red social completa — timeline, hashtags, mensajes directos cifrados,
notificaciones, imágenes y moderación — que se usa íntegramente desde un cliente
SSH. El backend está escrito en **Rust** y cada usuario conectado recibe un
proceso independiente con su propia pseudo-terminal (`forkpty`), de modo que una
sesión cuesta ~8 MB de RAM. No hay página web, no hay rastreo, no hay publicidad:
el feed es cronológico inverso y los datos pertenecen al usuario.

La persistencia es **SQLite embebido**: un solo binario self-contained, sin
servidor de base de datos. Cuando la concurrencia crece, AGORA puede repartir los
datos en varios archivos `.db` (un *mesh* de shards) y condensarlos de nuevo en
uno solo cuando la carga baja.

```
ssh agora.social -t          # o vía Tor: torsocks ssh agora.onion -t
```

Al conectar, el servidor sirve la **TUI nativa** (Rust + Ratatui): una interfaz
de terminal completa con timeline, mensajes, notificaciones y más, que corre en
un proceso independiente por sesión.

---

## Inicio rápido

```bash
git clone <repo> && cd cli-red-social

# Generar key de host
./setup-keys.sh

# Configurar secretos (Docker Compose no arranca sin SSH_PASSWORD)
cp .env.example .env
# Editá .env y reemplazá SSH_PASSWORD antes de continuar

# Levantar (una instancia + SQLite embebido)
docker compose up -d

# Conectarse
ssh localhost -p 2222 -t
```

## Desarrollo local

```bash
# Seed de datos (52 usuarios, 250+ posts)
cargo run -- --seed

# TUI nativa directo (sin SSH)
cargo run -- --tui

# Servidor SSH completo (sirve la TUI nativa a cada cliente SSH)
cargo run -- --port 2222

# Con logs a archivo
cargo run -- --port 2222 --log agora.log
```

## Variables de entorno

| Variable | Default | Descripción |
|---|---|---|
| `DATABASE_URL` | `agora.db` | Ruta del archivo SQLite embebido |
| `AGORA_DB_SHARDS` | `1` | Número de archivos `.db` del mesh (1 = archivo único) |
| `SSH_PASSWORD` | `agora` | Contraseña SSH compartida |
| `SSH_PORT` | `2222` | Puerto público |
| `RUST_LOG` | `info` | Nivel de logging |
| `LANG` | `es` | Idioma (`es` o `en`) |
| `AGORA_UPLOAD_DIR` | `./uploads` o `/data/uploads` | Directorio de imágenes |
| `AGORA_MODERATION_PLUGINS` | — | Plugins: `spam,profanity,link` |
| `REGISTRATION_MODE` | `open` | `open`, `invite` o `closed` (ver [Registro privado e invitaciones](#registro-privado-e-invitaciones)) |
| `AGORA_PUBLIC_HOST` | `localhost` | Host/IP público mostrado en el comando `scp` asistido |
| `AGORA_PUBLIC_SSH_PORT` | `2222` | Puerto público mostrado en el comando `scp` asistido |
| `SSH_CLIENT_IP` | — | IP del cliente (seteada por el servidor) |

## Uso

### Autenticación

- **Login:** `usuario:contraseña`
- **Registro:** `usuario:contraseña:nombre`

### Atajos principales

Atajos de la **TUI nativa** (Ratatui):

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
docker compose exec agora agora --invite-create --invite-days 7

# Consultar estado sin revelar códigos
docker compose exec agora agora --invite-list

# Revocar un código todavía no utilizado
docker compose exec agora agora --invite-revoke CODIGO
```

En modo `invite`, el registro usa el formato
`usuario:contraseña:nombre:invitación`. Los códigos se guardan como hashes y se
consumen atómicamente al crear la cuenta.

### Mesh de shards (opcional)

Cuando la concurrencia crece, se puede repartir la base en varios archivos
`.db`. Cada usuario se asigna a un shard por hash de su nombre y las consultas
que cruzan usuarios (timeline, búsqueda, trending) hacen fan-out y combinan los
resultados.

```bash
# Repartir en 4 archivos: agora.db, agora-1.db, agora-2.db, agora-3.db
AGORA_DB_SHARDS=4 cargo run -- --port 2222

# Cuando baja la carga, fusionar todo en un único archivo
AGORA_DB_SHARDS=4 cargo run -- --condense
```

`--condense` deja todos los datos en `agora.db` y elimina los shards
secundarios. El archivo único es el modo recomendado para la mayoría de
despliegues; el mesh es para picos de escritura sostenidos.

### Respaldo y restauración

```bash
# Copia el archivo SQLite, las imágenes y sumas SHA-256
./scripts/backup.sh

# Reemplaza la base actual; requiere confirmación explícita
./scripts/restore.sh backups/agora-AAAAMMDDTHHMMSSZ --yes
```

Conserva los respaldos fuera del servidor y prueba periódicamente que puedan
restaurarse. Si usás el mesh, condensá antes de respaldar.

### Plugins de moderación

```bash
AGORA_MODERATION_PLUGINS=spam,profanity,link cargo run -- --port 2222
```

- `spam`: bloquea mayúsculas excesivas y caracteres repetidos
- `profanity`: bloquea palabras configuradas
- `link`: bloquea URLs sospechosas

## Producción

```bash
# 1. Generar clave de host
./setup-keys.sh

# 2. Firewall
sudo ./firewall.sh

# 3. Contraseña SSH segura en un archivo local no versionado
cp .env.example .env
sed -i "s|replace-with-a-long-random-secret|$(openssl rand -base64 32)|" .env

# 4. Desplegar (instancia única + SQLite embebido)
docker compose up -d

# 5. Seed de datos (opcional)
docker compose exec agora agora --seed
```

## Arquitectura

```
     ┌──────────────┐
     │ SSH Client   │  ssh -p 2222
     └──────┬───────┘
            │
     ┌──────▼──────┐
     │   agora      │  (una instancia, forkpty por sesión)
     └──────┬───────┘
            │        cada sesión sirve la TUI nativa
            │        (Ratatui, llamadas directas a SQLite)
            │
     ┌──────▼───────┐
     │ SQLite (WAL) │  agora.db  (+ agora-N.db en modo mesh)
     └──────────────┘
```

- **Un proceso por sesión** (`forkpty`): sin locks compartidos, ~8 MB de RAM.
- **SQLite en WAL**: lecturas concurrentes ilimitadas, un escritor encolado.
- **Mesh opcional**: varios `.db` con fan-out para picos de concurrencia.
- **Sin dependencias externas**: un binario de ~12 MB lo corre todo.

## Documentación

| Documento | Contenido |
|---|---|
| `MANUAL_TECNICO.md` | Arquitectura, seguridad, código, API |
| `MANUAL_USUARIO.md` | Guía de uso de la TUI nativa, pantalla por pantalla |
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
