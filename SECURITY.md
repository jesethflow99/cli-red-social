# Seguridad

## Reportar una vulnerabilidad

No publiques vulnerabilidades explotables en un issue público. Contacta de forma
privada al responsable del proyecto e incluye una descripción, pasos para
reproducirla, impacto estimado y, si existe, una mitigación propuesta.

## Secretos y datos locales

- Nunca confirmes `.env`, claves privadas SSH, exportaciones, bases de datos ni
  imágenes subidas.
- Genera las claves de host en cada despliegue con `./setup-keys.sh`.
- Usa una contraseña SSH fuerte; la base de datos es SQLite embebido (un
  archivo local, sin credenciales propias).
- Si una clave o contraseña llegó a Git, rotarla es obligatorio; borrarla del
  último commit no elimina las copias del historial.

## Despliegue

Docker Compose exige `SSH_PASSWORD`. Usa `.env.example` como plantilla y limita
el acceso administrativo mediante firewall. Para equipos privados, usa
`REGISTRATION_MODE=invite` o `closed`. Respalda el archivo `agora.db` (o el mesh
condensado) con `./scripts/backup.sh`.

## Datos retenidos accidentalmente en Git

Antes de publicar este repositorio, elimina del historial las claves privadas,
`social.db` y `uploads/` con una herramienta como `git filter-repo`. Este proceso
reescribe commits y debe coordinarse con cualquier persona que ya haya clonado el
repositorio. Después, rota todas las claves y credenciales afectadas.
