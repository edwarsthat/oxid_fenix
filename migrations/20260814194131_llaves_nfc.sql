-- Add migration script here
CREATE SEQUENCE llaves_nfc_codigo_seq;

CREATE TABLE llaves_nfc (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    uid VARCHAR(32) NOT NULL UNIQUE,
    codigo VARCHAR(20) NOT NULL UNIQUE
        DEFAULT ('LL-' || LPAD(nextval('llaves_nfc_codigo_seq')::text, 4, '0')),

    estado VARCHAR(20) NOT NULL DEFAULT 'inventario',
    descripcion VARCHAR(200),

    version INTEGER NOT NULL DEFAULT 1,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT llaves_nfc_estado_check
        CHECK (estado IN ('inventario', 'perdida', 'dannada', 'baja'))
);

ALTER SEQUENCE llaves_nfc_codigo_seq OWNED By llaves_nfc.codigo;

CREATE INDEX idx_llaves_nfc_estado ON llaves_nfc (estado);

CREATE TRIGGER trg_llaves_nfc_actualizado_en
BEFORE UPDATE ON llaves_nfc
FOR EACH ROW EXECUTE FUNCTION set_actualizado_en_version();
