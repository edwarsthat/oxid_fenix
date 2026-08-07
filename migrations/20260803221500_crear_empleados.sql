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
    version INTEGER NOT NULL DEFAULT 1,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_personal_documento UNIQUE (tipo_documento, documento),
    CONSTRAINT personal_fechas_check
        CHECK (fecha_retiro IS NULL OR fecha_retiro >= fecha_ingreso)
);

ALTER SEQUENCE personal_codigo_seq OWNED BY personal.codigo;

CREATE INDEX idx_personal_activo ON personal (activo);
CREATE INDEX idx_personal_cargo ON personal (cargo_id);

-- Función propia: set_actualizado_en() la comparten usuarios y cargos, que no
-- tienen columna version. Acá el trigger sube la version en cada UPDATE, así
-- ninguna operación puede olvidarse de hacerlo.
CREATE OR REPLACE FUNCTION set_actualizado_en_version()
RETURNS TRIGGER AS $$
BEGIN
    NEW.actualizado_en = NOW();
    NEW.version = OLD.version + 1;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_personal_actualizado_en
BEFORE UPDATE ON personal
FOR EACH ROW EXECUTE FUNCTION set_actualizado_en_version();
