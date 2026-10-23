-- Add migration script here
CREATE SEQUENCE concentradores_codigo_seq;

CREATE TABLE concentradores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- 'CN-0001': lo que se escribe en la etiqueta pegada a la Pi.
    codigo VARCHAR(20) NOT NULL UNIQUE
        DEFAULT ('CN-' || LPAD(nextval('concentradores_codigo_seq')::text, 4, '0')),

    nombre VARCHAR(100) NOT NULL,       -- "Pi línea 1"
    ubicacion VARCHAR(200),             -- "Cuarto eléctrico, junto a la báscula"
    descripcion VARCHAR(500),

    -- SHA-256 de la API key, en hex. Con esto se identifica a la Pi en cada
    -- petición; la llave en claro no se guarda nunca.
    llave_hash CHAR(64) NOT NULL UNIQUE,
    -- Primeros caracteres de la llave, para reconocer en pantalla cuál tiene
    -- instalada cada Pi sin exponerla.
    llave_prefijo VARCHAR(8) NOT NULL,
    llave_emitida_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    estado VARCHAR(20) NOT NULL DEFAULT 'activo',

    version INTEGER NOT NULL DEFAULT 1,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT concentradores_estado_check
        CHECK (estado IN ('activo', 'baja'))
);

ALTER SEQUENCE concentradores_codigo_seq OWNED BY concentradores.codigo;

CREATE TRIGGER trg_concentradores_actualizado_en
BEFORE UPDATE ON concentradores
FOR EACH ROW EXECUTE FUNCTION set_actualizado_en_version();
