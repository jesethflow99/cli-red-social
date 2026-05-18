# Manual Técnico — cli-red-social

Red social en terminal accesible vía SSH. Sin navegador, sin JavaScript, sin cookies, sin rastreo.

---

## Índice

1. [Arquitectura general](#1-arquitectura-general)
2. [Modelo de seguridad](#2-modelo-de-seguridad)
3. [Ventajas frente a redes sociales web](#3-ventajas-frente-a-redes-sociales-web)
4. [Requisitos del sistema](#4-requisitos-del-sistema)
5. [Instalación y despliegue](#5-instalación-y-despliegue)
6. [Guía de uso](#6-guía-de-uso)
7. [Estructura del código](#7-estructura-del-código)
8. [API de base de datos](#8-api-de-base-de-datos)
9. [Limitaciones conocidas](#9-limitaciones-conocidas)

---

## 1. Arquitectura general

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

### Componentes

| Capa | Tecnología | Rol |
|---|---|---|
| Transporte | `russh` 0.46 | Servidor SSH con cifrado end-to-end |
| Terminal | `ratatui` 0.29 + `crossterm` 0.28 | Interfaz de usuario en terminal |
| Base de datos | PostgreSQL 17 + `r2d2` pool (10 conexiones) | Persistencia |
| Imágenes | `reqwest` (descarga bajo demanda) + `chafa`/`kitten`/`viu`/`catimg` | Visualización de imágenes |

### Flujo de conexión

1. Cliente SSH conecta al puerto 2222 (`russh`)
2. Servidor autentica con contraseña (opcional vía `SSH_PASSWORD`)
3. Abre canal: `channel_open_session` → `pty_request` → `shell_request`
4. `forkpty()` crea un nuevo proceso hijo con un pseudo-terminal
5. El hijo ejecuta `run_tui()` que inicia el loop de render + eventos de Ratatui
6. Padre reenvía datos entre el PTY y el canal SSH en ambas direcciones
7. Al cerrar sesión: `waitpid`, `close(fd)`, `channel_close`

---

## 2. Modelo de seguridad

### 2.1 Sin superficie web

No hay servidor HTTP, no hay puertos web, no hay cookies, no hay JavaScript. El único vector de ataque es el protocolo SSH, que es maduro, auditado y resistente.

### 2.2 Cifrado SSH

Todo el tráfico viaja cifrado con los algoritmos estándar de SSH (chacha20-poly1305, aes256-ctr, etc.). La clave de host se genera automáticamente como Ed25519.

### 2.3 Autenticación

```
SSH_PASSWORD=
  vacío → acepta cualquier password (modo desarrollo)
  fijo  → todos los usuarios usan la misma contraseña SSH

Autenticación de aplicación:
  usuario:contraseña → bcrypt(contraseña) → match
```

La contraseña de la aplicación se hashea con bcrypt (costo por defecto) y nunca se almacena en texto plano.

### 2.4 Rate limiting

| Acción | Límite | Ventana |
|---|---|---|
| Registro de usuarios | 3 | 60 segundos (global) |
| Creación de posts | 5 | 60 segundos (por usuario) |
| Follow/unfollow | 10 | 60 segundos (por usuario) |
| Comentarios | 10 | 60 segundos (por usuario) |
| Mensajes directos | 10 | 60 segundos (por usuario) |

Implementado con `HashMap<String, Vec<Instant>>` protegido por `Mutex`.

### 2.5 Validación de URLs de imágenes

- Solo se aceptan URLs que terminen en `jpg`, `jpeg`, `png`, `gif` o `webp`
- Se rechazan IPs privadas, loopback y unspecified (127.0.0.1, 10.x.x.x, 172.16-31.x.x, 192.168.x.x, ::1, etc.)
- Los hosts DNS se resuelven y se verifica que no apunten a IPs privadas
- Esto previene ataques SSRF (Server-Side Request Forgery)

### 2.6 Imágenes: sin almacenamiento local

Las imágenes **nunca se guardan en el servidor**:
- Solo se almacena la URL en la base de datos
- El servidor descarga la imagen a `/tmp/` **solo** cuando el usuario presiona `i` para verla
- Se elimina inmediatamente después de mostrarla
- El servidor no actúa como host de contenido multimedia

### 2.7 Firewall recomendado (`firewall.sh`)

```
Puertos abiertos:
  22     → SSH de administración (con rate-limit)
  2222   → Red social (con rate-limit)

Protecciones:
  - Política por defecto: DROP
  - Rate-limit: 10 conexiones/minuto (red social), 5/minuto (admin)
  - SYN flood: 1 paquete/s con burst de 3
  - ICMP rate-limit: 1 ping/s
  - Logging de paquetes denegados
```

### 2.8 Docker hardening

- PostgreSQL **no expone puertos al host**
- Contraseña de BD configurable via `DB_PASSWORD`
- El usuario del contenedor no es root
- Healthcheck de PostgreSQL antes de iniciar la app

### 2.9 Limpieza automática de datos viejos

Mensajes y notificaciones anteriores a 90 días se eliminan automáticamente cada 24 horas.

---

## 3. Ventajas frente a redes sociales web

### Privacidad

| Aspecto | Red web tradicional | cli-red-social |
|---|---|---|
| Rastreo | Cookies, fingerprint, píxeles | **Cero** |
| Publicidad | Segmentación por perfil | **No existe** |
| Datos personales | Email, teléfono, ubicación requeridos | **Solo username + contraseña** |
| Algoritmos | Feed manipulador | **Cronológico inverso** |
| Términos de servicio | Letra chica cambiante | **AGPL-3.0: código abierto, sin sorpresas** |

### Rendimiento

- Consume ~20 MB de RAM por sesión activa
- Sin JavaScript, sin CSS, sin procesamiento del lado del cliente
- Funciona en conexiones lentas (SSH comprime)
- Carga instantánea incluso en dispositivos viejos, Raspberry Pi, etc.

### Operación

- Sin mantenimiento de frontend web
- Sin certificados TLS (SSH los maneja)
- Sin CDN, sin DNS dinámico, sin proxy inverso
- Un solo binario estático (Rust)
- Despliegue con `docker compose up -d`

### Accesibilidad

- Funciona desde cualquier cliente SSH: PuTTY, OpenSSH, Termux, mobaXterm
- No requiere navegador moderno
- Ideal para acceder desde servidores sin entorno gráfico
- Funciona sobre Tor, VPN, o cualquier medio que transporte TCP

---

## 4. Requisitos del sistema

### Mínimos (producción)

| Recurso | Mínimo |
|---|---|
| RAM | 256 MB + ~20 MB por sesión |
| CPU | 1 núcleo |
| Disco | 1 GB (imagen Docker + datos) |
| SO | Linux con iptables (opcional) |
| Cliente SSH | Cualquier cliente compatible |

### Recomendados (servidor)

| Recurso | Recomendado |
|---|---|
| RAM | 1 GB |
| CPU | 2 núcleos |
| Disco | 10 GB SSD |
| SO | Ubuntu 22.04+ / Debian 12+ |

### Dependencias (ejecución local sin Docker)

- Rust toolchain (edición 2024)
- PostgreSQL 16+
- Visores de imagen opcionales: `chafa`, `kitten`, `viu`, `catimg`

---

## 5. Instalación y despliegue

### 5.1 Docker (recomendado)

```bash
git clone <repo> && cd cli-red-social

# Opcional: configurar contraseñas
export DB_PASSWORD=clave_segura_40_caracteres
export SSH_PASSWORD=otra_clave_segura

# Iniciar
docker compose up -d

# Ver logs
docker compose logs -f

# Conectarse
ssh localhost -p 2222 -t
```

### 5.2 Producción segura

```bash
# 1. Firewall
sudo ./firewall.sh

# 2. Variables de entorno
export SSH_PASSWORD="$(openssl rand -base64 32)"
export DB_PASSWORD="$(openssl rand -base64 32)"

# 3. Iniciar con contraseñas
SSH_PASSWORD="$SSH_PASSWORD" DB_PASSWORD="$DB_PASSWORD" docker compose up -d

# 4. Hacer persistente el firewall
apt install iptables-persistent
iptables-save > /etc/iptables/rules.v4
```

### 5.3 Local (sin Docker)

```bash
# Base de datos
createdb social
psql social -c "CREATE USER social WITH PASSWORD 'social';"
psql social -c "GRANT ALL ON SCHEMA public TO social;"

# Ejecutar
cargo run -- --tui                    # Modo local
cargo run                             # Servidor SSH (puerto 2222)
DATABASE_URL=postgres://... cargo run # Conexión personalizada
```

---

## 6. Guía de uso

### 6.1 Inicio de sesión

```
Formato: usuario:contraseña
Ejemplo: juan:mi_clave_segura
```

### 6.2 Registro

```
Formato: usuario:contraseña:nombre
Ejemplo: juan:mi_clave_segura:Juan Pérez

Restricciones:
  - Mínimo 4 caracteres de contraseña
  - Máximo 3 registros por minuto (global)
  - Username único
```

### 6.3 Atajos de teclado

#### Timeline

| Tecla | Acción |
|---|---|
| `j` / `k` o ↑/↓ | Navegar posts |
| `Enter` | Ver detalle del post |
| `n` | Nuevo post |
| `/` | Buscar posts |
| `s` | Buscar usuarios |
| `p` | Mi perfil |
| `m` | Mensajes directos |
| `Ctrl+n` | Notificaciones |
| `i` | Ver imagen (descarga bajo demanda) |
| `d` | Mostrar URL de imagen para descargar |
| `q` | Salir |

#### Nuevo post

| Tecla | Acción |
|---|---|
| Escribir | Redactar contenido |
| `Ctrl+u` | Adjuntar imagen desde URL |
| `Enter` | Publicar |
| `Esc` | Cancelar |

#### Detalle de post

| Tecla | Acción |
|---|---|
| `c` | Comentar |
| `i` | Ver imagen |
| `e` | Editar (solo dueño) |
| `D` | Eliminar post (solo dueño, mayúscula) |
| `d` | Eliminar comentario seleccionado (solo dueño) |
| `↑/↓` o `j/k` | Navegar comentarios |
| `b` / `Esc` | Volver |

#### Perfil

| Tecla | Acción |
|---|---|
| `f` | Seguir / dejar de seguir |
| `w` | Ver seguidores |
| `g` | Ver siguiendo |
| `e` | Editar perfil |
| `m` | Enviar mensaje |
| `x` | Borrar cuenta (pide confirmación con contraseña) |
| `↑/↓` | Navegar posts del perfil |
| `Enter` | Ver detalle del post |

#### Búsqueda de posts

| Tecla | Acción |
|---|---|
| Escribir | Término de búsqueda |
| `Tab` | Cambiar filtro de fecha (all/24h/7d/30d) |
| `Enter` | Buscar / seleccionar resultado |
| `↑/↓` | Navegar resultados |
| `Esc` | Volver |

#### Mensajes

| Tecla | Acción |
|---|---|
| `Enter` | Abrir conversación |
| `b` / `Esc` | Volver |
| `q` | Salir |

#### Chat

| Tecla | Acción |
|---|---|
| Escribir | Redactar mensaje |
| `Enter` | Enviar |
| `Esc` | Volver a lista |
| `Ctrl+q` | Salir |

---

## 7. Estructura del código

```
cli-red-social/
├── Cargo.toml          # Dependencias y configuración del crate
├── docker-compose.yml  # Orquestación Docker (app + PostgreSQL)
├── Dockerfile          # Build multi-stage (compilación Rust → Alpine)
├── firewall.sh         # Script de iptables para producción
├── LICENSE             # AGPL-3.0
├── README.md           # Inicio rápido
├── MANUAL_TECNICO.md   # Este documento
└── src/
    ├── main.rs         # Entry point, CLI parser, cleanup thread
    ├── app.rs          # TUI: App, renderers, key handlers (~2040 líneas)
    ├── db.rs           # Database layer: trait + PostgreSQL impl + mock
    ├── models.rs       # Structs: User, Post, Comment, Message, Notification, Screen
    ├── ssh.rs          # Servidor SSH: SshServer + SshSession (~308 líneas)
    └── theme.rs        # Colores y estilos de la interfaz
```

### 7.1 main.rs

Punto de entrada. Parsea argumentos CLI con `clap`:
- `--tui`: modo local sin SSH
- `--port`: puerto SSH (default 2222)
- `--db`: URL de base de datos
- `--key`: ruta a la clave de host SSH

Lee `SSH_PASSWORD` y `DATABASE_URL` de entorno (prioridad sobre CLI). Inicia un hilo de limpieza diaria de datos viejos.

### 7.2 app.rs

El núcleo de la interfaz. Contiene:

- **`App` struct**: 50+ campos de estado (usuario actual, pantalla activa, timeline, input, etc.)
- **`run()`**: loop principal con `catch_unwind` alrededor de render y eventos
- **`handle_key()`**: dispatch a `handle_*_key()` según `Screen` activo
- **`render()`**: dispatch a `render_*()` según `Screen` activo
- **`draw_safe()`**: helper que envuelve `terminal.draw()`
- **Manejadores**: login, register, timeline, create_post, post_detail, profile, search, messages, chat, edit_profile, notifications, post_search
- **Visor de imágenes**: descarga a `/tmp/`, delega a `chafa`/`kitten`/`viu`/`catimg`, limpia al cerrar

Pantallas (enum `Screen`):
`Login`, `Register`, `Timeline`, `CreatePost`, `PostDetail(i64)`, `Profile(i64)`, `UserSearch`, `Messages`, `Chat(i64)`, `EditProfile`, `Notifications`, `PostSearch`, `PostSearchFilter`

### 7.3 db.rs

Capa de acceso a datos. Sigue el patrón trait + impl:

- **`DatabaseOps` trait**: define todas las operaciones (registro, auth, posts, comments, follows, mensajes, notificaciones, búsqueda)
- **`Database` struct**: implementación real con `r2d2` pool + `rate_limiter`
- **`MockDatabase` struct**: implementación en memoria para tests
- **`init_schema()`**: migración automática con `BIGSERIAL`/`BIGINT`
- **Rate limiting**: in-memory con `HashMap<String, Vec<Instant>>`

### 7.4 ssh.rs

Servidor SSH completo con:

- `load_or_generate_key()`: genera clave Ed25519 si no existe
- `auth_password()`: verifica contra `SSH_PASSWORD`
- `pty_request()`: captura tamaño de ventana del cliente
- `shell_request()`: `forkpty()`, establece tamaño de ventana con `ioctl(TIOCSWINSZ)`, bucle bidireccional de datos
- `window_change_request()`: propaga cambios de tamaño al PTY
- `channel_eof()` / `channel_close()`: limpieza de recursos

### 7.5 models.rs

Define las estructuras de datos:

- `User`, `Post`, `Comment`, `Message`, `Notification`
- `Screen` enum (13 variantes)
- Tests unitarios para cada modelo

### 7.6 theme.rs

Paleta de colores y helpers de estilo. Colores oscuros (zebra impar: `rgb(22,22,28)`, status bar: `rgb(18,18,24)`).

---

## 8. API de base de datos

### Esquema

```
users       (id BIGSERIAL, username TEXT UNIQUE, password_hash TEXT, display_name TEXT, bio TEXT, created_at TEXT)
posts       (id BIGSERIAL, user_id BIGINT → users, content TEXT, image_path TEXT, created_at TEXT)
follows     (follower_id BIGINT → users, following_id BIGINT → users, PK compuesta)
comments    (id BIGSERIAL, post_id BIGINT → posts, user_id BIGINT → users, content TEXT, created_at TEXT)
messages    (id BIGSERIAL, sender_id BIGINT → users, receiver_id BIGINT → users, content TEXT, created_at TEXT, read INT)
notifications (id BIGSERIAL, user_id BIGINT → users, from_user_id BIGINT → users, type TEXT, created_at TEXT, read INT)
```

### Operaciones principales

| Método | SQL | Descripción |
|---|---|---|
| `register_user` | `INSERT INTO users ... RETURNING id` | Crea usuario con bcrypt |
| `authenticate` | `SELECT ... WHERE username = $1` | Verifica bcrypt |
| `create_post` | `INSERT INTO posts ... RETURNING id` | Crea post (rate-limited) |
| `get_timeline` | `JOIN follows ... ORDER BY created_at DESC LIMIT 50` | Feed cronológico |
| `search_posts` | `ILIKE $1 ... WHERE created_at > NOW() - interval` | Búsqueda con filtro temporal |
| `search_users` | `ILIKE $1 OR display_name ILIKE $1 LIMIT 20` | Búsqueda de usuarios |
| `send_message` | `INSERT INTO messages ...` | Mensaje directo (rate-limited) |
| `cleanup_old_data` | `DELETE WHERE created_at < NOW() - interval` | Limpieza automática |

### Pool de conexiones

- Tamaño máximo: 10 conexiones
- Mínimo idle: 0
- Timeout de conexión: 3 segundos

---

## 9. Limitaciones conocidas

| Limitación | Detalle |
|---|---|
| Sin cifrado end-to-end en mensajes | Los mensajes viajan cifrados por SSH pero se almacenan en texto plano en la BD |
| Una sola contraseña SSH | Todos los usuarios comparten el mismo gate SSH; la autenticación de aplicación es independiente |
| Imágenes solo por URL | No hay upload de archivos, solo referencias a URLs externas |
| Sin soporte multilingüe | Interfaz en español |
| Sin websockets | No hay actualización en tiempo real; el usuario debe refrescar manualmente |
| Sin paginación | Timeline limitado a 50 posts, resultados a 20-50 |
| Rate limiter en memoria | Se pierde al reiniciar el servidor |
| Sin moderación | No hay reportes, bloqueos ni roles de administrador |
| Sin recovery de contraseña | No hay email; si olvida la contraseña, pierde la cuenta |
| Sin búsqueda full-text de PostgreSQL | Usa `ILIKE` con `%query%`, no usa índices GIN/tsvector |
