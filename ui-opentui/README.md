# AGORA OpenTUI

Interfaz interactiva de AGORA construida con React + OpenTUI. Se conecta al
backend Rust mediante RPC local y trabaja con los usuarios, publicaciones,
mensajes y notificaciones reales de la base de datos.

## Ejecutar

Desde la raíz del proyecto:

```bash
cargo build
cd ui-opentui
npm install
DATABASE_URL=postgres://usuario:clave@localhost/social npm start
```

Si el binario de AGORA no está en `target/debug/agora`, se puede indicar con
`AGORA_BACKEND_BIN=/ruta/al/binario`.

## Navegación

- `1`–`5`: Inicio, Explorar, Mensajes, Alertas y Perfil.
- `j`/`k` o flechas: mover el foco.
- `n`: nueva publicación desde Inicio.
- `/`: buscar publicaciones.
- `Enter`: abrir la conversación seleccionada.
- `m`: responder en una conversación abierta.
- `p`: abrir el perfil del autor seleccionado.
- `c`: comentar una publicación abierta.
- `r`: responder un comentario; fuera de un hilo abre AGORA Radio.
- Dentro de Publicar, `Ctrl+P`: adjuntar mediante SCP.
- Dentro de Publicar, `Ctrl+U`: adjuntar mediante URL.
- `?`: ayuda contextual.
- `Esc`: cerrar ventana.
- `q`: salir.

La interfaz adapta su composición al ancho de la terminal y emplea un acento
distinto por área: cian para Inicio, violeta para Explorar, azul para Mensajes,
ámbar para Alertas y verde para Perfil. Las tarjetas alternan seis tonos para
conservar jerarquía visual sin sacrificar legibilidad.

## Imágenes

Pulsa `N` y después `Ctrl+P` para generar un comando SCP listo para copiar. El usuario SSH del
comando es un token de un solo uso que expira en cinco minutos; no hay que
escribir el usuario de AGORA ni una ruta remota. La ventana detecta el archivo,
lo valida, elimina sus metadatos, lo redimensiona y lo adjunta automáticamente.

Configura `AGORA_PUBLIC_HOST` con el dominio o IP que deben usar los clientes y
`SSH_PORT` con el puerto público anunciado en el comando.
