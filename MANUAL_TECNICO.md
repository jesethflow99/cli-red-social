# Manual Técnico — AGORA

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
9. [Subida de imágenes (SCP/SFTP)](#9-subida-de-imágenes-scpsftp)
10. [Sistema de plugins](#10-sistema-de-plugins)
11. [Escalabilidad](#11-escalabilidad)

---

## 1. Arquitectura general

```
     ┌──────────────┐
     │ SSH Client   │
     └──────┬───────┘
            │ TCP :2222
     ┌──────▼──────┐
     │ nginx stream │  (balanceo TCP, hash por IP)
     └──┬────┬────┬─┘
        │    │    │
   ┌────▼┐ ┌▼──┐ ┌▼────┐
   │agora│ │2  │ │agora│   (N instancias Rust)
   └──┬──┘ └┬──┘ └──┬──┘
      │ forkpty    │
      ▼            ▼
   ┌──────┐    ┌──────┐
   │ TUI  │    │ TUI  │    (ratatui + crossterm)
   └──┬───┘    └──┬───┘
      └──────────┘
            │
      ┌─────▼─────┐
      │ PostgreSQL │        (r2d2 pool 25 conexiones)
      └───────────┘
```

### Componentes

| Capa | Tecnología | Rol |
|---|---|---|
| Proxy | nginx `stream` | Balanceo TCP capa 4, hash por IP |
| Transporte | `russh` 0.46 + `russh-sftp` 2.1 | Servidor SSH + subsistema SFTP |
| Terminal | `ratatui` 0.29 + `crossterm` 0.28 | Interfaz TUI (14 pantallas) |
| Base de datos | PostgreSQL 17 + `r2d2` pool (25) | Persistencia con conexiones reciclables |
| Imágenes | `image` crate 0.25 + `chafa`/`kitten`/`viu` | Procesamiento y visualización |
| Concurrencia | `forkpty` (nix) | 1 proceso hijo por sesión SSH |

### Flujo de conexión

1. Cliente SSH conecta al puerto 2222 → nginx reenvía a una instancia (`hash $remote_addr consistent`)
2. Servidor `russh` autentica con contraseña compartida (`SSH_PASSWORD`)
3. Abre canal: `channel_open_session` → `shell_request`
4. `forkpty()` crea proceso hijo con pseudo-terminal
5. Hijo ejecuta `run_tui()` con Ratatui (loop de render + eventos)
6. Variables `SSH_CLIENT_IP` y `SSH_SESSION_TOKEN` se inyectan en el hijo
7. Padre reenvía datos bidireccionalmente entre PTY y canal SSH
8. Al cerrar: `waitpid`, `PtyMaster` cierra fd automáticamente

---

## 2. Modelo de seguridad

### 2.1 Sin superficie web

No hay HTTP, cookies, JavaScript, ni formularios web. El único vector de ataque es SSH.

### 2.2 Cifrado

- SSH con Ed25519 (host key auto-generada o provista)
- bcrypt (costo 12) para contraseñas de aplicación
- Tráfico comprimido por SSH

### 2.3 Autenticación en dos capas

```
Capa 1 (SSH):    contraseña compartida → acceso al túnel
Capa 2 (TUI):    usuario:contraseña → bcrypt → acceso a Agora
```

El username SSH no se asocia a ningún usuario de Agora. Solo sirve para el túnel.

### 2.4 Rate limiting

| Acción | Límite | Ventana | Tipo |
|---|---|---|---|
| Registro | 3 | 60s | Global (memoria) |
| Posts | 5 | 60s | Por usuario (DB) |
| Follow/unfollow | 10 | 60s | Por usuario (DB) |
| Comentarios | 10 | 60s | Por usuario (DB) |
| Mensajes | 10 | 60s | Por usuario (DB) |

DB rate limiter incluye **baneo progresivo**: si excedés el límite, se aplica un baneo de `window_secs * 1`, `* 2`, o `* 4` según reincidencia.

### 2.5 URLs de imágenes (anti-SSRF)

- Solo extensiones: `jpg`, `jpeg`, `png`, `gif`, `webp`
- IPs rechazadas: privadas, loopback, unspecified
- Resolución DNS con verificación de IPs
- Descarga con timeout 15s y límite 10 MB

### 2.6 Imágenes sin almacenamiento (URLs)

- URLs externas: se descarga, muestra y **elimina** de `/tmp/`
- Imágenes SCP: se validan, limpian de metadata, redimensionan a ≤512px, convierten a JPEG

### 2.7 Firewall (nftables)

```
Tabla inet agora_fw:
├── input (policy drop)
│   ├── loopback → accept
│   ├── established/related → accept
│   ├── SSH admin (22) → rate-limit 5/min
│   ├── Red social (2222) → rate-limit 10/min
│   ├── SYN flood → 5/s
│   └── ICMP → 1/s
```

### 2.8 Limpieza automática

| Dato | Expiración |
|---|---|
| Mensajes directos | 90 días |
| Notificaciones | 90 días |
| Cuentas inactivas | 2 años (sin login mínimo) |
| Rate limits | 1 hora |

Ejecutado cada 24h por hilo en segundo plano. Las cuentas inactivas se borran en cascada (posts, comentarios, follows, mensajes).

---

## 3. Ventajas frente a redes sociales web

| Aspecto | Red web | AGORA |
|---|---|---|
| Rastreo | Cookies, fingerprint, píxeles | **Cero** |
| Publicidad | Segmentada | **No existe** |
| Datos requeridos | Email, teléfono, ubicación | **Solo username + contraseña** |
| Algoritmo | Feed manipulador | **Cronológico inverso** |
| Licencia | Términos cambiantes | **AGPL-3.0** |
| RAM por usuario | 50-200 MB (navegador) | **~5 MB** |
| Carga inicial | 2-10 segundos | **Instantánea** |
| Conexiones lentas | Inusable | **SSH comprime, funciona con 2G** |
| Accesibilidad | Requiere navegador moderno | **Cualquier cliente SSH** |

---

## 4. Requisitos del sistema

### Mínimos (1 instancia)

| Recurso | Valor |
|---|---|
| RAM | 256 MB + ~5 MB por sesión |
| CPU | 1 núcleo |
| Disco | 1 GB + imágenes |
| SO | Linux con nftables |

### Recomendados (3 instancias + nginx)

| Recurso | Valor |
|---|---|
| RAM | 4 GB |
| CPU | 4 núcleos |
| Disco | 20 GB SSD |

---

## 5. Instalación y despliegue

### 5.1 Docker (multi-instancia)

```bash
./setup-keys.sh

export DB_PASSWORD=clave_segura
export SSH_PASSWORD=otra_clave

docker compose up -d
docker compose exec agora1 agora --seed   # datos de prueba

ssh localhost -p 2222 -t
```

### 5.2 Desarrollo local

```bash
docker compose up -d db          # solo PostgreSQL
cargo run -- --seed              # datos de prueba
cargo run -- --tui               # TUI directo
cargo run -- --port 2222         # servidor SSH
cargo run -- --port 2222 --log agora.log  # con logs
```

### 5.3 Comandos CLI

| Comando | Descripción |
|---|---|
| `agora --tui` | TUI directo (sin SSH) |
| `agora --port 2222` | Servidor SSH |
| `agora --seed` | Insertar datos de prueba |
| `agora --export --user <u>` | Exportar datos como JSON |
| `agora --log <archivo>` | Logs a archivo |

---

## 6. Guía de uso

### 6.1 Timeline

| Tecla | Acción |
|---|---|
| `j`/`k` o ↑/↓ | Navegar posts |
| `Enter` | Ver detalle |
| `n` | Nuevo post |
| `Ctrl+U` | Subir imagen (modo recepción SCP) |
| `Ctrl+L` | Adjuntar desde URL |
| `/` | Buscar posts |
| `#` | Trending hashtags |
| `R` | Modo Radio (ticker automático) |
| `s` | Buscar usuarios |
| `p` | Perfil propio |
| `m` | Mensajes |
| `Ctrl+N` | Notificaciones |
| `i` | Ver imagen |
| `Ctrl+F`/`Ctrl+B` | Paginación |
| `Ctrl+Q` | Salir |

### 6.2 Modo Radio

| Tecla | Acción |
|---|---|
| `r` | Pausar/reanudar rotación |
| `n` | Siguiente hashtag |
| `Enter` | Ver post |
| `b`/`Esc` | Volver |

### 6.3 Subida de imágenes

| Tecla | Acción |
|---|---|
| `Ctrl+U` | Entrar en modo recepción |
| `↑/↓` | Navegar archivos |
| `Enter` | Adjuntar imagen seleccionada |
| `d` | Borrar imagen |
| `r` | Refrescar lista |
| `Esc` | Cancelar |

### 6.4 Post detail

| Tecla | Acción |
|---|---|
| `c` | Comentar |
| `r` | Responder a comentario |
| `i` | Ver imagen |
| `e` | Editar (dueño) |
| `D` | Eliminar post (confirma `y`/`n`) |
| `d` | Eliminar comentario (dueño) |
| `b`/`Esc` | Volver |

### 6.5 Perfil

| Tecla | Acción |
|---|---|
| `f` | Seguir/dejar de seguir |
| `w` | Ver seguidores |
| `g` | Ver siguiendo |
| `e` | Editar perfil |
| `E` | Exportar datos |
| `m` | Enviar mensaje |
| `x` | Borrar cuenta |

---

## 7. Estructura del código

```
src/
├── main.rs         # Entry point, CLI, logging, cleanup thread
├── app.rs          # TUI: App struct, 14 renderers, key handlers (~3000 líneas)
├── db.rs           # DatabaseOps trait + Database impl + MockDatabase + seed
├── models.rs       # User, Post, Comment, Message, Notification, Screen
├── ssh.rs          # SSH server + SFTP handler + process_image + upload_dir
├── i18n.rs         # Internacionalización (es/en) + macro t!()
├── theme.rs        # Paleta oscura + estilos
├── plugins.rs      # Sistema de middleware: SpamFilter, ProfanityFilter, LinkFilter
└── firewall.rs     # nftables dinámico (allow_scp / revoke_scp)
```

### 7.1 Pantallas (Screen enum)

`Login`, `Register`, `Timeline`, `CreatePost`, `PostDetail(i64)`, `Profile(i64)`, `UserSearch`, `Messages`, `Chat(i64)`, `EditProfile`, `Notifications`, `PostSearch`, `HashtagView`, `HashtagTrending`, `Radio`

### 7.2 app.rs

- **`App` struct**: 60+ campos de estado
- **`run()`**: loop con `catch_unwind`, poll de eventos cada 80ms, watchdog de uploads y radio
- **`handle_key()`**: dispatch a 15 manejadores según pantalla
- **`render()`**: dispatch a 15 renderers + status bar
- **Visor de imágenes**: limpia pantalla, muestra con chafa/kitten, espera input, limpia

### 7.3 db.rs

- **`DatabaseOps` trait**: 59 métodos (CRUD, búsqueda, follows, mensajes, notificaciones, rate limiting, export, hashtags)
- **`Database`**: PostgreSQL via `r2d2` pool, rate limiter en memoria + DB
- **`MockDatabase`**: En memoria para 32 tests unitarios
- **Schema**: 8 tablas con migración automática y ALTER incremental
- **Seed**: 52 usuarios, 250+ posts, 160 comentarios, 120 mensajes, 170 follows

### 7.4 ssh.rs

- `SshServer` + `SshSession` (russh Handler)
- `SftpSession` (russh-sftp Handler): open, close, read, write, readdir, stat, fstat, fsetstat, setstat, remove, rename, mkdir, rmdir
- `process_image()`: validación con `image` crate, redimensionado ≤512px Lanczos3, conversión a JPEG
- `sanitize_path_for_user()`: restringe paths a `<upload_dir>/<usuario>/`
- `generate_session_token()`: token único por conexión SSH

---

## 8. API de base de datos

### Esquema

```sql
users         (id BIGSERIAL PK, username TEXT UNIQUE, password_hash TEXT,
               display_name TEXT, bio TEXT, utc_offset INT,
               created_at TEXT, last_login_at TEXT, login_count INT)

posts         (id BIGSERIAL PK, user_id BIGINT FK, content TEXT,
               image_path TEXT, created_at TEXT)

follows       (follower_id BIGINT FK, following_id BIGINT FK, PK compuesta)

comments      (id BIGSERIAL PK, post_id BIGINT FK, user_id BIGINT FK,
               content TEXT, created_at TEXT, parent_comment_id BIGINT)

messages      (id BIGSERIAL PK, sender_id BIGINT FK, receiver_id BIGINT FK,
               content TEXT, created_at TEXT, read INT)

notifications (id BIGSERIAL PK, user_id BIGINT FK, from_user_id BIGINT FK,
               type TEXT, created_at TEXT, read INT, related_id BIGINT)

rate_limits   (user_id BIGINT, action TEXT, window_start TEXT,
               count INT, banned_until TEXT, PK compuesta)

post_hashtags (post_id BIGINT FK, tag TEXT, PK compuesta, INDEX on tag)
```

### Operaciones principales

| Método | Descripción |
|---|---|
| `register_user` | INSERT con bcrypt, validación de duplicados |
| `authenticate` | SELECT + bcrypt::verify, actualiza last_login |
| `create_post` | INSERT + extract hashtags + extract mentions + notify |
| `get_timeline` | JOIN follows, ORDER BY created_at DESC, LIMIT/OFFSET |
| `get_trending_hashtags` | GROUP BY tag, COUNT, ORDER BY cnt DESC |
| `get_posts_by_hashtag` | JOIN post_hashtags, WHERE tag = $1 |
| `search_posts` | ILIKE con filtro temporal opcional (24h/7d/30d) |
| `search_posts_by_user` | JOIN users, ILIKE username |
| `export_user_data` | SELECT agregado de toda la actividad, JSON |
| `cleanup_old_data` | DELETE mensajes y notificaciones > 90 días |
| `cleanup_inactive_users` | DELETE usuarios sin login > 2 años (cascada) |
| `clear_image_from_posts` | UPDATE posts SET image_path = '' |

---

## 9. Subida de imágenes (SCP/SFTP)

### Flujo

```
Usuario TUI          Terminal externa        Servidor
─────┬─────              ─────┬─────          ───┬───
     │ Ctrl+U                 │                  │
     ├── "Modo recepción" ──► │                  │
     │                        │ scp file.jpg     │
     │                        │ localhost:        │
     │                        │ jeseth/file.jpg ─►│
     │                        │                  ├── auth_password("jeseth", pass)
     │                        │                  ├── subsystem_request("sftp")
     │                        │                  ├── sanitize_path("jeseth/file.jpg")
     │                        │                  │   → ./uploads/jeseth/file.jpg
     │                        │                  ├── open(WRITE) → write → close
     │                        │                  ├── process_image()
     │                        │                  │   → validate, resize ≤512px, JPEG
     │                  [OK] ◄┘                  │
     │  ← watchdog detecta ──────────────────────┤
     │  ← 🆕 Nuevo archivo ─────────────────────┤
     │  ← lista actualizada ────────────────────┤
```

### Restricciones

- Formatos: PNG, JPG, JPEG, GIF, WebP (SVG bloqueado)
- Tamaño máximo: 10 MB
- Redimensionado: ≤512px (Lanczos3)
- Conversión: siempre a JPEG (elimina metadata)
- Aislamiento: cada usuario SSH solo sube a su directorio de Agora
- Validación: archivos no-imagen se borran automáticamente

---

## 10. Sistema de plugins

### Arquitectura

```rust
pub trait ModerationPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn filter_post(&self, user_id: i64, content: &str) -> Result<()>;
    fn filter_comment(&self, user_id: i64, content: &str) -> Result<()>;
    fn filter_message(&self, sender: i64, receiver: i64, content: &str) -> Result<()>;
    fn can_register(&self, username: &str, display_name: &str) -> Result<()>;
}
```

### Plugins incluidos

| Plugin | Hook | Comportamiento |
|---|---|---|
| `spam` | filter_post, filter_comment | Bloquea >80% mayúsculas, >8 chars repetidos |
| `profanity` | filter_post, filter_comment, filter_message | Bloquea palabras en lista negra |
| `link` | filter_post | Bloquea URLs sospechosas, shortlinks |

### Activación

```bash
AGORA_MODERATION_PLUGINS=spam,profanity,link cargo run -- --port 2222
```

Los plugins se integran en los handlers de creación de posts, comentarios, mensajes y registro. Si un plugin rechaza, la operación se cancela con mensaje de error.

---

## 11. Escalabilidad

Ver [ESCALABILIDAD.md](ESCALABILIDAD.md) para el análisis completo.

Resumen:

| Configuración | Usuarios simultáneos |
|---|---|
| 1 instancia | ~500 |
| 3 instancias + nginx | ~1500 |
| 10 instancias | ~5000+ |

Factores clave:
- **Procesos independientes** (forkpty): sin contención de locks entre usuarios
- **Pool de conexiones** (r2d2): consultas milisegundos, conexiones reciclables
- **Sin estado compartido** entre instancias (solo PostgreSQL)
- **Escalado horizontal** trivial: más instancias → más capacidad

---

## Donaciones ❤️

**PayPal:** `ceggarr199@gmail.com`

[paypal.me/ceggarr199](https://paypal.me/ceggarr199)
