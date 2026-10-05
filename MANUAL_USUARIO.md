# Manual de Usuario — AGORA

Guía completa para usar la red social desde tu terminal.

AGORA es una red social privada y minimalista que se usa desde cualquier
terminal vía SSH. Publicá posts con hashtags y menciones, seguí a otras
personas, chateá por mensajes directos, subí imágenes y exportá tus datos — todo
sin navegador, sin cookies y sin rastreo. Tu usuario se crea al primer registro
y solo necesitás username y contraseña (más un código de invitación si el
servidor lo exige).

Este manual documenta la **TUI nativa** (Ratatui), la interfaz que sirve el
servidor a cada conexión SSH.

---

## Índice

1. [Conectarse](#1-conectarse)
2. [Registro y Login](#2-registro-y-login)
3. [Timeline](#3-timeline)
4. [Crear publicaciones](#4-crear-publicaciones)
5. [Hashtags](#5-hashtags)
6. [Imágenes](#6-imágenes)
7. [Detalle de publicación](#7-detalle-de-publicación)
8. [Comentarios y respuestas](#8-comentarios-y-respuestas)
9. [Perfiles](#9-perfiles)
10. [Seguir y dejar de seguir](#10-seguir-y-dejar-de-seguir)
11. [Mensajes directos](#11-mensajes-directos)
12. [Notificaciones](#12-notificaciones)
13. [Búsqueda](#13-búsqueda)
14. [Editar perfil](#14-editar-perfil)
15. [Modo Radio](#15-modo-radio)
16. [Exportar datos](#16-exportar-datos)
17. [Eliminar cuenta](#17-eliminar-cuenta)
18. [Atajos rápidos](#18-atajos-rápidos)

---

## 1. Conectarse

Desde cualquier terminal:

```bash
ssh agora.social -t
# o si el puerto no es el 22:
ssh agora.social -p 2222 -t
# vía Tor:
torsocks ssh agora.onion -t
```

No importa el username que uses para SSH. La autenticación real se hace dentro de la TUI.

---

## 2. Registro y Login

### Registrarse por primera vez

```
Formato: usuario:contraseña:nombre
Ejemplo:  jeseth:m1cl4v3:Jeseth G.
```

- Usuario: mínimo 1 carácter, sin espacios, único
- Contraseña: mínimo 4 caracteres
- Nombre: el que se mostrará en tu perfil

Si el servidor tiene el registro en modo invitación
(`REGISTRATION_MODE=invite`, el default en el despliegue Docker), agregá el
código de invitación al final:

```
Formato: usuario:contraseña:nombre:invitación
Ejemplo:  jeseth:m1cl4v3:Jeseth G.:AB12CD34
```

Cada código es de un solo uso. Pedile uno a quien administre el servidor.

### Iniciar sesión

```
Formato: usuario:contraseña
Ejemplo:  jeseth:m1cl4v3
```

**Teclas en pantalla de login/registro:**
| Tecla | Acción |
|---|---|
| `Tab` | Alternar entre Login y Registro |
| `Enter` | Enviar |
| `Esc` | Salir |
| `Ctrl+Q` | Salir |

---

## 3. Timeline

Tu feed principal. Muestra las publicaciones de las personas que seguís y las tuyas propias, en orden cronológico inverso (más recientes primero).

**Teclas del timeline:**

| Tecla | Acción |
|---|---|
| `j` / `↓` | Siguiente publicación |
| `k` / `↑` | Publicación anterior |
| `Enter` | Ver detalle de la publicación |
| `n` | Crear nueva publicación |
| `Ctrl+P` | Subir imagen (modo recepción SCP) |
| `Ctrl+U` | Adjuntar imagen desde URL |
| `/` | Buscar publicaciones |
| `#` | Ver trending hashtags |
| `R` | Modo Radio (ticker automático) |
| `s` | Buscar usuarios |
| `p` | Ver tu perfil |
| `m` | Mensajes directos |
| `Ctrl+N` | Notificaciones |
| `i` | Ver imagen adjunta (si la publicación tiene) |
| `d` | Ver instrucciones para descargar imagen |
| `Ctrl+F` | Página siguiente |
| `Ctrl+B` | Página anterior |
| `Ctrl+Q` | Salir |

**Indicadores visuales:**
- 📷 al lado de una publicación = tiene imagen adjunta
- `#hashtag` en color resaltado
- `@usuario` en color verde
- `[hace 5 min]` = timestamp relativo

---

## 4. Crear publicaciones

Presioná `n` desde el timeline para crear una publicación.

**Teclas:**

| Tecla | Acción |
|---|---|
| Escribir texto | Redactar contenido |
| `Ctrl+P` | Adjuntar imagen desde archivo local (SCP) |
| `Ctrl+U` | Adjuntar imagen desde URL |
| `Enter` | Publicar |
| `Esc` | Cancelar |

**Reglas:**
- Máximo 5000 caracteres
- Los hashtags (`#tema`) se extraen automáticamente y se indexan
- Las menciones (`@usuario`) generan notificación al mencionado
- Límite: 5 publicaciones por minuto

### Adjuntar imagen desde URL (`Ctrl+U`)

1. Presioná `Ctrl+U`
2. Pegá la URL de la imagen (debe terminar en .jpg, .jpeg, .png, .gif o .webp)
3. Presioná `Enter`
4. La URL se guarda en la publicación (la imagen se descarga solo al verla)

### Adjuntar imagen local (`Ctrl+P`)

Ver sección [Subir imágenes](#61-subir-imágenes-locales-scp).

---

## 5. Hashtags

Los hashtags funcionan como etiquetas dinámicas. Cualquier palabra que empiece con `#` en una publicación se convierte automáticamente en hashtag.

### Ver trending

Presioná `#` desde el timeline para ver los hashtags más usados, ordenados por frecuencia.

| Tecla | Acción |
|---|---|
| `j`/`k` | Navegar hashtags |
| `Enter` | Ver publicaciones con ese hashtag |
| `b`/`Esc` | Volver al timeline |

### Ver publicaciones de un hashtag

Desde trending, seleccioná uno con `Enter`. Verás todas las publicaciones etiquetadas con ese hashtag.

| Tecla | Acción |
|---|---|
| `j`/`k` | Navegar publicaciones |
| `Enter` | Ver detalle de publicación |
| `b`/`Esc` | Volver al timeline |
| `#` | Volver a trending |

---

## 6. Imágenes

### 6.1 Subir imágenes locales (SCP)

Este es el método para subir imágenes desde tu computadora al servidor.

1. En la pantalla de crear publicación, presioná `Ctrl+P`
2. La TUI entra en **modo recepción** y te muestra el comando SCP exacto:

   ```
   scp -P 2222 archivo.jpg localhost:jeseth/archivo.jpg
   ```

3. **Desde otra terminal**, copiá y pegá ese comando, cambiando `archivo.jpg` por tu archivo real
4. Autenticate con la contraseña SSH
5. La imagen se sube, y la TUI la detecta automáticamente en ~2 segundos
6. El archivo nuevo aparece **resaltado en verde** al tope de la lista
7. Seleccionalo con las flechas y presioná `Enter` para adjuntarlo

**Teclas del modo recepción:**

| Tecla | Acción |
|---|---|
| `↑`/`↓` | Navegar imágenes |
| `Enter` | Seleccionar imagen para adjuntar |
| `d` | **Borrar** imagen seleccionada |
| `r` | Refrescar lista manualmente |
| `Esc` | Cancelar (salir del modo recepción) |

**Qué pasa con la imagen al subir:**
- Se valida que sea una imagen real (PNG, JPG, GIF, WebP)
- Si es mayor a 512px, se **redimensiona** a 512px
- Se **convierte a JPEG** (eliminando toda metadata: ubicación, cámara, fecha)
- Si no es una imagen válida, se **borra automáticamente**
- Archivos mayores a 10 MB se rechazan

### 6.2 Ver imágenes

En cualquier parte donde veas el ícono 📷, presioná `i` para ver la imagen.

- La terminal se limpia y muestra la imagen con `chafa` (o `kitten`/`viu` si están instalados)
- Barra inferior: `d: descargar | Enter/q/Esc: volver`

| Tecla | Acción |
|---|---|
| `d` | Ver instrucciones de descarga |
| `Enter` / `q` / `Esc` | Volver a la TUI |

### 6.3 Descargar imágenes

Al presionar `d` mientras ves una imagen:

```
📥 Descargar: sprite.png — 36.3 KB
scp -P 2222 localhost:sprite.png .
```

Copiá ese comando en otra terminal para descargar la imagen a tu computadora.

### 6.4 Borrar imágenes

Desde el modo recepción (`Ctrl+P`):
1. Seleccioná la imagen con las flechas
2. Presioná `d`
3. La imagen se borra del disco y se desvincula de cualquier publicación que la usara

---

## 7. Detalle de publicación

Al presionar `Enter` sobre una publicación, ves su vista completa con comentarios.

**Teclas:**

| Tecla | Acción |
|---|---|
| `c` | Escribir comentario |
| `r` | Responder a un comentario específico |
| `i` | Ver imagen adjunta |
| `↑`/`↓` o `j`/`k` | Navegar comentarios |
| `e` | Editar publicación (solo si sos el autor) |
| `D` (mayúscula) | Eliminar publicación (solo si sos el autor) |
| `d` | Eliminar comentario seleccionado (solo si sos el autor) |
| `b` / `Esc` | Volver al timeline |

### Eliminar publicación (`D`)

1. Presioná `D` (mayúscula)
2. Aparece una confirmación en rojo: `¿Eliminar este post y todos sus comentarios? (y/n)`
3. `y` → elimina | `n` → cancela | `Esc` → cancela

Se borra la publicación **y todos sus comentarios**.

### Editar publicación (`e`)

1. Presioná `e`
2. Editá el texto
3. `Enter` → guardar | `Esc` → cancelar

---

## 8. Comentarios y respuestas

### Comentar en una publicación

1. Abrí la publicación con `Enter`
2. Presioná `c`
3. Escribí tu comentario
4. `Enter` → enviar | `Esc` → cancelar

### Responder a un comentario

1. Navegá los comentarios con `↑`/`↓`
2. Seleccioná el comentario al que querés responder
3. Presioná `r`
4. Escribí tu respuesta
5. `Enter` → enviar

Las respuestas aparecen indentadas con `└─` debajo del comentario padre, creando hilos de conversación visuales.

### Eliminar comentario

Solo podés eliminar tus propios comentarios. Seleccionalo y presioná `d`.

---

## 9. Perfiles

### Ver tu perfil

Presioná `p` desde el timeline.

### Ver el perfil de otro usuario

1. Presioná `s` (buscar usuarios)
2. Escribí el nombre o username
3. Seleccionalo y presioná `Enter`

### Información que muestra un perfil

- Nombre, username, bio
- Cantidad de seguidores y seguidos
- Lista de publicaciones del usuario
- Si lo estás siguiendo o no

**Teclas del perfil:**

| Tecla | Acción |
|---|---|
| `↑`/`↓` | Navegar publicaciones del perfil |
| `Enter` | Ver detalle de publicación |
| `f` | Seguir / dejar de seguir |
| `w` | Ver lista de seguidores |
| `g` | Ver lista de seguidos |
| `e` | Editar perfil (solo si es tuyo) |
| `E` | Exportar datos (solo si es tuyo) |
| `m` | Enviar mensaje directo |
| `x` | Borrar cuenta (solo si es tuya) |
| `b` / `Esc` | Volver |

---

## 10. Seguir y dejar de seguir

Seguir a alguien hace que sus publicaciones aparezcan en tu timeline.

- **Seguir**: andá al perfil del usuario y presioná `f`
- **Dejar de seguir**: presioná `f` de nuevo
- Cuando seguís a alguien, esa persona recibe una notificación

---

## 11. Mensajes directos

### Ver conversaciones

Presioná `m` desde el timeline. Verás la lista de personas con las que has intercambiado mensajes.

| Tecla | Acción |
|---|---|
| `↑`/`↓` | Navegar conversaciones |
| `Enter` | Abrir conversación |
| `b` / `Esc` | Volver al timeline |

### Enviar mensaje

Hay dos formas de iniciar un chat:

1. Desde el perfil de alguien → presioná `m`
2. Desde la lista de conversaciones → si ya existe, seleccionala

**En el chat:**

| Tecla | Acción |
|---|---|
| Escribir | Redactar mensaje |
| `Enter` | Enviar |
| `Esc` | Volver a conversaciones |

### Sobre los mensajes

- Son **privados**: solo vos y el destinatario los ven
- Se marcan como leídos al abrir la conversación
- Se **eliminan automáticamente** después de 90 días
- Límite: 10 mensajes por minuto
- El indicador ✉ en la barra de estado muestra mensajes no leídos

---

## 12. Notificaciones

Presioná `Ctrl+N` desde el timeline para ver tus notificaciones.

**Tipos de notificación:**
- 👤 **Follow**: `@usuario te ha seguido` → al seleccionarla, vas a su perfil
- 💬 **Mention**: `@usuario te mencionó` → al seleccionarla, vas a la publicación

| Tecla | Acción |
|---|---|
| `↑`/`↓` | Navegar notificaciones |
| `Enter` | Ir al perfil/publicación |
| `b` / `Esc` | Volver al timeline |

- El ícono 🔔 en la barra de estado muestra notificaciones no leídas
- Las notificaciones se eliminan automáticamente después de 90 días

---

## 13. Búsqueda

### Buscar publicaciones (`/`)

Presioná `/` desde el timeline.

**Tres modos de búsqueda** (cambiás con `Tab`):

| Modo | Busca por | Ejemplo |
|---|---|---|
| `@usuario` | Username del autor | `jeseth` |
| `tema` | Contenido de la publicación | `rust` |
| `fecha` | Fecha de creación | `2026-05` |

**Teclas:**

| Tecla | Acción |
|---|---|
| `Tab` | Cambiar modo de búsqueda |
| `Enter` | Buscar / seleccionar resultado |
| `↑`/`↓` | Navegar resultados |
| `Ctrl+F`/`Ctrl+B` | Página siguiente/anterior |
| `Esc` | Volver al timeline |

### Buscar usuarios (`s`)

Presioná `s` desde el timeline.

- Escribí parte del nombre o username
- Presioná `Enter` para buscar
- Seleccioná un resultado y presioná `Enter` para ver su perfil

---

## 14. Editar perfil

Desde tu perfil (`p`), presioná `e`.

**Campos editables:**

| Campo | Descripción |
|---|---|
| **Nombre** | Tu nombre visible (requerido) |
| **Bio** | Texto descriptivo sobre vos |
| **Zona horaria** | UTC offset para timestamps correctos (ej: -6, +1, +3) |

**Teclas:**

| Tecla | Acción |
|---|---|
| `Tab` | Cambiar entre campos |
| `↑`/`↓` | Ajustar zona horaria (en pasos de 30 min) |
| `Enter` | Guardar cambios |
| `Esc` | Cancelar |

---

## 15. Modo Radio

Un ticker automático que rota por los hashtags más populares a velocidad de lectura humana (~8 segundos por hashtag).

Presioná `R` desde el timeline para activarlo.

**Qué muestra:**
- Hashtag actual y cantidad de publicaciones
- La última publicación con ese hashtag (autor, contenido, timestamp)
- Spinner animado que indica que está rotando

**Teclas:**

| Tecla | Acción |
|---|---|
| `r` | Pausar / reanudar rotación |
| `n` | Saltar al siguiente hashtag |
| `Enter` | Abrir la publicación en detalle |
| `b` / `Esc` | Volver al timeline |

Cuando está pausado, el header muestra `[PAUSADO]`.

---

## 16. Exportar datos

Podés descargar un archivo JSON con todo tu historial: publicaciones, comentarios, mensajes, seguidores y seguidos.

### Desde la TUI

1. Andá a tu perfil (`p`)
2. Presioná `E` (mayúscula)
3. El archivo se genera y te muestra instrucciones para descargarlo con SCP:

   ```
   scp -P 2222 localhost:export_jeseth_20260520.json .
   ```

### Desde línea de comandos (admin)

```bash
agora --export --user jeseth --format json
```

El archivo incluye:
- Datos de perfil
- Todas tus publicaciones
- Todos tus comentarios
- Todos tus mensajes (enviados y recibidos)
- Lista de seguidores y seguidos

---

## 17. Eliminar cuenta

1. Andá a tu perfil (`p`)
2. Presioná `x`
3. Escribí tu contraseña para confirmar
4. `Enter` → elimina permanentemente

**Qué se borra:**
- Tu usuario y perfil
- Todas tus publicaciones
- Todos tus comentarios
- Tus mensajes (enviados y recibidos)
- Tus follows (seguidores y seguidos)
- Tus notificaciones

**Esta acción es irreversible.**

---

## 18. Atajos rápidos

### Navegación universal

| Tecla | Acción |
|---|---|
| `j` / `↓` | Siguiente elemento |
| `k` / `↑` | Elemento anterior |
| `Enter` | Seleccionar / confirmar |
| `b` / `Esc` | Volver atrás |
| `Ctrl+Q` | Salir de la aplicación |
| `Tab` | Cambiar entre opciones |

### Timeline

| Tecla | Acción |
|---|---|
| `n` | Nuevo post |
| `Ctrl+P` | Subir imagen |
| `Ctrl+U` | Adjuntar URL |
| `/` | Buscar posts |
| `#` | Trending hashtags |
| `R` | Modo Radio |
| `s` | Buscar usuarios |
| `p` | Tu perfil |
| `m` | Mensajes |
| `Ctrl+N` | Notificaciones |
| `i` | Ver imagen |
| `Ctrl+F`/`Ctrl+B` | Páginas |

### Detalle de publicación

| Tecla | Acción |
|---|---|
| `c` | Comentar |
| `r` | Responder |
| `e` | Editar (dueño) |
| `D` | Eliminar (dueño, confirma) |
| `d` | Eliminar comentario (dueño) |

### Chat

| Tecla | Acción |
|---|---|
| `Enter` | Enviar mensaje |
| `Esc` | Volver a conversaciones |

### Modo recepción de imágenes

| Tecla | Acción |
|---|---|
| `↑`/`↓` | Elegir imagen |
| `Enter` | Adjuntar |
| `d` | Borrar |
| `r` | Refrescar |

---

## Límites del sistema

| Acción | Límite |
|---|---|
| Publicaciones | 5 por minuto |
| Comentarios | 10 por minuto |
| Mensajes | 10 por minuto |
| Registros | 3 por minuto (global) |
| Caracteres por post | 5000 |
| Imagen (SCP) | 10 MB máximo |
| Imagen (redimensionada) | 512px máximo |
| Formato de imagen | PNG, JPG, GIF, WebP |
| Conexiones SSH | 10 por minuto por IP |

---

## Consejos

- Usá `#tema` para etiquetar tus publicaciones. Cuantos más hashtags uses, más poblado estará el Modo Radio
- Mencioná a otros con `@usuario` para que reciban notificación
- Las imágenes que subís se limpian automáticamente de metadata (ubicación, cámara, etc.)
- Tus mensajes y notificaciones se borran solos a los 90 días
- Si no usás tu cuenta por 2 años, se elimina automáticamente
- Presioná `Ctrl+Q` para salir de forma segura (nunca cierres la terminal directamente)
