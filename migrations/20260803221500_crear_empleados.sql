-- Add migration script here
CREATE SEQUENCE personal_codigo_seq;

CREATE TABLE personal (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    codigo VARCHAR(20) NOT NULL UNIQUE DEFAULT ('OP-' || LPAD(nextval('personal_codigo_seq')::text, 4, '0')),
    tipo_documento VARCHAR(10) NOT NULL DEFAULT 'CC',
    documento VARCHAR(30) NOT NULL,

    nombre VARCHAR(100) NOT NULL,
    apellido VARCHAR(100) NOT NULL,
    fecha_nacimiento DATE,
    telefono VARCHAR(30),

    cargo_id UUID NOT NULL REFERENCES cargos_personal (id),
    fecha_ingreso DATE NOT NULL,
    fecha_retiro DATE,

    activo BOOLEAN NOT NULL DEFAULT TRUE,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_personal_documento UNIQUE (tipo_documento, documento),
    CONSTRAINT personal_fechas_check
        CHECK (fecha_retiro IS NULL OR fecha_retiro >= fecha_ingreso)
);

ALTER SEQUENCE personal_codigo_seq OWNED BY personal.codigo;

CREATE INDEX idx_personal_activo ON personal (activo);
CREATE INDEX idx_personal_cargo ON personal (cargo_id);

CREATE TRIGGER trg_personal_actualizado_en
BEFORE UPDATE ON personal
FOR EACH ROW EXECUTE FUNCTION set_actualizado_en();
