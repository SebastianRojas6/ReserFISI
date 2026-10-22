# ReserFISI ✳

Una web full stack en **Rust + Topcoat + PostgreSQL** para reservar los espacios de la **Facultad de Ingeniería de Sistemas e Informática (FISI) de la Universidad Nacional Mayor de San Marcos (UNMSM)**. Renderizado en servidor, formularios funcionales sin JavaScript y mejoras progresivas sin Node, WASM ni compilación de frontend.

## Qué incluye

- Cancha de fútbol, Aula Magna y Auditorio, con filtros por categoría y fecha.
- Agenda diaria con disponibilidad real, en hora de Lima (UTC−5).
- Registro de estudiantes contra un padrón local administrado por la facultad.
- Contraseñas Argon2id y sesiones persistentes de 7 días; tokens aleatorios almacenados como SHA-256, cookies `HttpOnly` / `SameSite=Lax` y protección CSRF en operaciones autenticadas.
- Confirmación inmediata, historial personal y cancelación antes del inicio.
- Protección de cruces **en PostgreSQL**, también con solicitudes concurrentes: un espacio y un estudiante no pueden tener reservas activas superpuestas. Los intervalos son `[inicio, fin)`, por lo que se permiten reservas consecutivas.
- Diseño responsive editorial: `#F2FDFF`, `#1B497A`, `#FFE20A`, titulares serif, ilustraciones SVG originales, navegación por teclado y respeto por movimiento reducido.
- Fotografías reales de la FISI en la portada y logo a color en cabecera, pie y formularios. Los recursos se sirven localmente desde Rust.

### Identidad e imágenes

`assets/fisi.jpg` y `assets/fisi2.jpg` muestran la fachada y los jardines de la facultad. `assets/logo-fisi.png` conserva los colores del logo y se utiliza también como favicon; `assets/logo-fisi.svg` se conserva como recurso vectorial monocromático. Las imágenes están incluidas en el binario mediante `include_bytes!`: al reemplazarlas, vuelve a compilar y reinicia el servidor.

## Requisitos

- Rust **1.95 o superior**. Se fija **Topcoat 0.6.2**, compatible con esa versión mínima y con el Rust 1.97 instalado en este proyecto.
- PostgreSQL local **14 o superior**, con la extensión `btree_gist` disponible.
- `psql` para crear la base y cargar el padrón.

## Ejecutar con PostgreSQL instalado localmente

Con el servicio PostgreSQL iniciado, entra como administrador:

```sh
sudo -u postgres psql
```

```sql
CREATE ROLE reserfisi LOGIN PASSWORD 'reserfisi_local';
CREATE DATABASE reserfisi OWNER reserfisi;
\connect reserfisi
CREATE EXTENSION IF NOT EXISTS btree_gist;
\quit
```

Copia `.env.example` a `.env` y ajusta `DATABASE_URL` con tu usuario, contraseña, host, puerto y base:

```sh
cp .env.example .env
cargo run
```

Abre **http://127.0.0.1:3000**. Las migraciones se aplican automáticamente al iniciar; los tres espacios se crean una sola vez. El servidor necesita conectarse a PostgreSQL para arrancar.

### PostgreSQL local con contenedor (opcional)

El archivo `compose.yaml` ofrece una base local con volumen persistente:

```sh
docker compose up -d db
cp .env.example .env
cargo run
```

También puedes usar Podman. En este entorno se ha iniciado una base de desarrollo con:

```sh
podman run --detach --name reserfisi-dev-db \
  -e POSTGRES_USER=reserfisi \
  -e POSTGRES_PASSWORD=reserfisi_local \
  -e POSTGRES_DB=reserfisi \
  -p 127.0.0.1:5432:5432 \
  docker.io/library/postgres:18-alpine
```

Para volver a iniciarla: `podman start reserfisi-dev-db`. Para detenerla: `podman stop reserfisi-dev-db`. Las credenciales de ejemplo son únicamente para desarrollo local.

## Cargar estudiantes FISI

La pertenencia a la facultad se determina por `student_roster`. Antes de registrar cuentas, importa el padrón oficial. Ejemplo de `students.csv` (no se versiona):

```csv
student_code,full_name,email
20260001,Lucía Torres,lucia.torres@fisi.example
20260002,Mateo Rojas,mateo.rojas@fisi.example
```

```sh
psql "postgres://reserfisi:reserfisi_local@localhost:5432/reserfisi" \
  -c "\copy student_roster (student_code, full_name, email) FROM 'students.csv' WITH (FORMAT csv, HEADER true)"
```

El código debe tener entre 6 y 12 dígitos y el correo debe estar en minúsculas. Para una prueba rápida, carga el padrón ficticio incluido **después de iniciar la aplicación por primera vez**:

```sh
psql "postgres://reserfisi:reserfisi_local@localhost:5432/reserfisi" -f fixtures/demo.sql
```

Abre **Crear cuenta** y utiliza `20260001` / `lucia.torres@fisi.example`, o `20260002` / `mateo.rojas@fisi.example`, con una contraseña elegida por ti de al menos 10 caracteres. No hay cuentas ni contraseñas precargadas.

El registro compara código y correo con el padrón; **no envía un correo de verificación ni se conecta a un sistema universitario de identidad**. El padrón y su acceso deben ser administrados por la facultad. Si se requiere comprobar la titularidad del correo, se debe integrar el correo institucional o el SSO de la universidad.

Para desactivar el acceso de un estudiante sin borrar su historial:

```sql
UPDATE student_roster SET active = false WHERE student_code = '20260001';
```

Las sesiones de ese estudiante dejan de autorizar operaciones inmediatamente. Si corresponde liberar sus reservas, la administración puede cancelarlas en PostgreSQL.

## Reglas de reserva

- De **08:00 a 22:00**, todos los días, en hora de Lima.
- Duración entre **30 minutos y 3 horas**, en bloques de 30 minutos.
- Inicio en el futuro y hasta **30 días** de anticipación.
- Participantes: al menos uno, hasta el aforo configurado para el espacio.
- Sin solapamientos del espacio ni del propio estudiante.
- Cancelación solo por el propietario y antes del inicio. Para cambiar de horario, cancela y crea una nueva reserva.

La agenda muestra bloques de una hora: “Reservado” significa que toda o parte de esa hora está ocupada. El formulario permite reservar medias horas, que el servidor valida al confirmar. Los aforos iniciales (22, 120 y 250), las ubicaciones y los equipamientos son configurables en la tabla `spaces`; deben ajustarse a los datos reales de la facultad.

## Configuración

| Variable | Uso |
|---|---|
| `DATABASE_URL` | Conexión PostgreSQL; obligatoria. |
| `HOST` | Interfaz HTTP; por defecto `127.0.0.1`. |
| `PORT` | Puerto HTTP; por defecto `3000`. |
| `COOKIE_SECURE` | `false` para HTTP local; `true` con HTTPS. |
| `RUST_LOG` | Nivel de logs, por ejemplo `reserfisi=info`. |

## Verificación

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
node --check assets/app.js
```

La prueba de integración crea una base aislada y comprueba dos reservas simultáneas, cruces del estudiante, horarios consecutivos, liberación por cancelación y aforo. Requiere una conexión PostgreSQL con permiso `CREATEDB` (el usuario del contenedor ya lo tiene):

```sh
DATABASE_URL=postgres://reserfisi:reserfisi_local@localhost:5432/reserfisi \
  cargo test database_prevents_races -- --include-ignored
```

`GET /health` verifica que el servidor y PostgreSQL están disponibles.

## Estructura

```text
src/main.rs       Servidor Topcoat, rutas, recursos y cabeceras
src/ui.rs         Páginas y componentes Rust / view!
src/auth.rs       Contraseñas, sesiones y CSRF
src/actions.rs    Formularios de registro, ingreso, reserva y cancelación
src/db.rs         Consultas, reglas de negocio y pruebas
migrations/       Esquema, restricciones GiST y espacios iniciales
assets/           CSS, JavaScript progresivo e ilustraciones SVG locales
fixtures/         Padrón ficticio opcional de desarrollo
```
