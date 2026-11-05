# QA — Hallazgos y plan de corrección

Fecha: 2026-11-04
Alcance: binario `target/release/agora`, SQLite + mesh, TUI por SSH, SCP/SFTP, Docker.
Resultado base: **44/44 tests unitarios OK**, build release OK, contenedor estable (`RestartCount=0`).

Estado de los hallazgos: `[ ]` pendiente · `[x]` corregido · `[~]` parcial / a confirmar.

---

## Críticos
_(ninguno encontrado: no hubo caídas ni pérdida/corrupción de datos)_

---

## Altos

### [x] F1 — La validación de URL de imagen no comprueba la extensión
- **Pasos:** TUI → `n` → `Ctrl+U` → pegar `http://example.com/archivo.txt` → `Enter`.
- **Esperado:** rechazo ("solo jpg/png/gif/webp", como indican la UI y los manuales).
- **Actual:** se acepta y se adjunta como imagen.
- **Evidencia:** status de la TUI `URL: http://example.com/archivo.txt`; `src/app.rs` `is_valid_image_url()` solo valida esquema + host.
- **Sospecha:** `is_valid_image_url()` / flujo `url_mode` en `create_post`.
- **Plan:** exigir extensión `jpg/jpeg/png/gif/webp` en el path (ignorando query/fragment) + test unitario.

### [x] F2 — Anti-SSRF incompleto (no resuelve DNS y no revalida al descargar)
- **Pasos:** adjuntar `http://<hostname-público-que-resuelve-a-127.0.0.1>/x.jpg` y ver la imagen (`i`).
- **Esperado:** rechazo (el manual técnico promete "Resolución DNS con verificación de IPs").
- **Actual:** `is_safe_url()` solo bloquea IPs literales/`localhost`; un hostname que resuelve a una IP privada pasa. `sync_download()` no revalida antes de descargar.
- **Impacto:** SSRF — el servidor puede ser inducido a pedir recursos internos.
- **Plan:** resolver el host (DNS) y rechazar si alguna IP resuelta es privada/loopback/unspecified/link-local; revalidar en `sync_download()` y limitar redirecciones. Tests.

---

## Medios

### [x] F3 — `upload_dir()` elige `/data/uploads` solo por existir `/data`
- **Pasos:** en un host donde `/data` existe pero no es escribible, subir por SCP.
- **Esperado:** usar un directorio escribible (p. ej. `./uploads`).
- **Actual:** SCP falla con `Permission denied`; en logs, `path escape attempt` (porque `canonicalize()` del base no se puede crear).
- **Evidencia:** la fase SCP solo funcionó tras exportar `AGORA_UPLOAD_DIR`; en Docker funciona porque `/data` es escribible.
- **Plan:** elegir `/data/uploads` solo si es **escribible** (probar escritura), si no `./uploads`. Tests.

### [ ] F4 — `scp` sube el archivo pero el cliente queda colgado
- **Pasos:** `scp -P <port> foto.jpg localhost:alice/x.jpg`.
- **Esperado:** subida y salida limpia.
- **Actual:** el archivo **sí** se sube y procesa (512×384) pero el cliente no cierra (exit 124 por timeout).
- **Evidencia:** log cliente `channel 0: free: client-session` sin `Connection closed`; log servidor `Image processed ...`. El `ssh` a la TUI sí cierra limpio.
- **Sospecha:** cierre del canal en el subsistema SFTP (`subsystem_request`/`channel_close`) o artefacto del harness (`setsid`+askpass).
- **Plan:** reproducir con cliente real; si es del servidor, cerrar explícitamente el canal al terminar el subsistema.

### [x] F5 — El comando SCP mostrado ignora `AGORA_PUBLIC_HOST` / `AGORA_PUBLIC_SSH_PORT`
- **Esperado (docs):** usar esas variables.
- **Actual:** la TUI muestra siempre `scp -P 2222 archivo.jpg localhost:<user>/archivo.jpg`.
- **Impacto:** en despliegue remoto el comando es incorrecto.
- **Plan:** construir el comando con `AGORA_PUBLIC_HOST` y `AGORA_PUBLIC_SSH_PORT` (y también en las instrucciones de descarga/export).

---

## Bajos

### [x] F6 — Modo Radio pega autor/contenido/fecha sin separación
- **Actual:** `@sysadminHoy empece...`, `...experiencia[hace 14 h]`.
- **Plan:** separadores/espaciado en `render_radio`.

### [x] F7 — Registro duplicado muestra `'@alice'`
- **Actual:** `El usuario '@alice' ya existe`.
- **Plan:** quitar el `@` extra en el texto.

### [x] F8 — Re-ejecutar `--seed` no reinicia AUTOINCREMENT
- **Actual:** tras 2 seeds, `meta.next_id` salta a 500 (ids más grandes). **No colisiona** (verificado).
- **Plan:** resetear `sqlite_sequence` al inicio del seed.

### [x] F9 — Post vacío + `Enter` no da feedback
- **Actual:** no-op silencioso.
- **Plan:** mostrar "el post no puede estar vacío".

---

## Informativo

### [ ] F10 — Clippy: 112 warnings, 0 errores
- No bloqueante; limpieza opcional.

---

## Verificado y funcionando ✅

CLI (seed/export/invites/condense), mesh de shards + condense, auth SSH (correcta/incorrecta/vacía),
TUI (login, timeline, crear/editar/borrar post, comentarios, búsqueda, perfil, seguir, editar perfil,
mensajes, notificaciones, radio, trending, export), entradas adversas (5001 chars, unicode/emoji,
`<b>`, `';--` sin SQLi, rate limit con baneo, registro duplicado), SCP (válido, no-imagen,
path traversal, cross-user), Docker (salud, env, sesión).

---

## Cambios aplicados (verificados)

- **F1:** `is_valid_image_url` ahora exige extensión `jpg/jpeg/png/gif/webp` (ignora query/fragment) vía `has_image_extension`. Verificado: `.txt` rechazado, `.jpg` aceptado. +3 tests.
- **F2:** `is_safe_url` resuelve DNS y rechaza si alguna IP resuelta es privada/loopback/link-local/unicast-local; `sync_download` revalida y valida cada salto de redirección. Tests con IPs literales.
- **F3:** `upload_dir()` solo usa `/data/uploads` si es **escribible** (prueba de escritura); si no, `./uploads`. Verificado sin `AGORA_UPLOAD_DIR` en host con `/data` no escribible.
- **F5:** los comandos `scp` mostrados (Ctrl+P, export, visor de imágenes) usan `AGORA_PUBLIC_HOST` y `AGORA_PUBLIC_SSH_PORT`. Verificado: `scp -P 2200 midominio.example:alice/...`.
- **F6:** el modo Radio usa varias `Line` (ratatui no interpreta `\n` dentro de un Span). Verificado: autor/contenido/fecha en líneas separadas.
- **F7:** se quitó el `@` extra del mensaje de duplicado. Verificado: `El usuario 'alice' ya existe`.
- **F8:** el seed resetea `sqlite_sequence`. Verificado: tras 2 seeds, `next_id=250`, `alice id=1`.
- **F9:** post vacío muestra "El post no puede estar vacío". Verificado.

Tests: **47/47 OK**. Pendiente: F4 (confirmar con cliente SCP real) y F10 (warnings de estilo).
