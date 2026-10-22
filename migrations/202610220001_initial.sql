CREATE EXTENSION IF NOT EXISTS btree_gist;

-- La administración importa aquí el padrón oficial de estudiantes FISI.
CREATE TABLE student_roster (
    student_code text PRIMARY KEY CHECK (student_code ~ '^[0-9]{6,12}$'),
    full_name text NOT NULL CHECK (length(full_name) BETWEEN 3 AND 120),
    email text NOT NULL UNIQUE CHECK (email = lower(email)),
    active boolean NOT NULL DEFAULT true
);

CREATE TABLE students (
    id uuid PRIMARY KEY,
    student_code text NOT NULL UNIQUE REFERENCES student_roster(student_code),
    password_hash text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    token_hash text PRIMARY KEY,
    student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
    csrf_token text NOT NULL,
    expires_at timestamptz NOT NULL
);
CREATE INDEX sessions_expiry_idx ON sessions(expires_at);

CREATE TABLE spaces (
    id smallint PRIMARY KEY,
    name text NOT NULL UNIQUE,
    category text NOT NULL,
    description text NOT NULL,
    capacity integer NOT NULL CHECK (capacity > 0),
    location text NOT NULL,
    amenity text NOT NULL,
    illustration text NOT NULL
);

INSERT INTO spaces VALUES
    (1, 'Cancha de fútbol', 'Deporte', 'Un encuentro, un equipo, una buena tarde. El espacio para compartir dentro y fuera de la cancha.', 22, 'Zona deportiva · FISI', 'Al aire libre', 'cancha'),
    (2, 'Aula Magna', 'Académico', 'Las grandes ideas necesitan un lugar. Reúne a tu comunidad para aprender y crear juntos.', 120, 'Pabellón principal · FISI', 'Proyector y pizarra', 'aula'),
    (3, 'Auditorio', 'Cultural', 'Un escenario para lo que tienes que contar. Charlas, presentaciones y encuentros que dejan huella.', 250, 'Pabellón principal · FISI', 'Sonido y proyección', 'auditorio');

CREATE TABLE reservations (
    id uuid PRIMARY KEY,
    student_id uuid NOT NULL REFERENCES students(id),
    space_id smallint NOT NULL REFERENCES spaces(id),
    starts_at timestamptz NOT NULL,
    ends_at timestamptz NOT NULL,
    purpose text NOT NULL CHECK (length(purpose) BETWEEN 5 AND 180),
    attendees integer NOT NULL CHECK (attendees > 0),
    cancelled_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (ends_at > starts_at AND ends_at - starts_at <= interval '3 hours'),
    CHECK ((starts_at AT TIME ZONE 'America/Lima')::date = (ends_at AT TIME ZONE 'America/Lima')::date),
    CHECK ((starts_at AT TIME ZONE 'America/Lima')::time >= time '08:00'
        AND (ends_at AT TIME ZONE 'America/Lima')::time <= time '22:00'),
    CHECK (extract(minute FROM starts_at AT TIME ZONE 'America/Lima') IN (0, 30)
        AND extract(minute FROM ends_at AT TIME ZONE 'America/Lima') IN (0, 30)
        AND extract(second FROM starts_at) = 0 AND extract(second FROM ends_at) = 0),
    -- [inicio, fin): una reserva puede comenzar justo cuando termina otra.
    CONSTRAINT no_space_overlap EXCLUDE USING gist (
        space_id WITH =, tstzrange(starts_at, ends_at, '[)') WITH &&
    ) WHERE (cancelled_at IS NULL),
    CONSTRAINT no_student_overlap EXCLUDE USING gist (
        student_id WITH =, tstzrange(starts_at, ends_at, '[)') WITH &&
    ) WHERE (cancelled_at IS NULL)
);
CREATE INDEX reservations_student_idx ON reservations(student_id, starts_at DESC);
