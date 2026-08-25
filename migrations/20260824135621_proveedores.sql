-- Add migration script her
CREATE SEQUENCE proveedores_codigo_seq;

CREATE TABLE proveedores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    codigo VARCHAR(20) NOT NULL UNIQUE
        DEFAULT ('PR-' || LPAD(nextval('proveedores_codigo_seq')::text, 4, '0')),

    tipo_proveedor VARCHAR(30) NOT NULL,

    tipo_persona VARCHAR(10) NOT NULL DEFAULT 'natural',
    tipo_documento VARCHAR(10) NOT NULL DEFAULT 'CC',
    documento VARCHAR(30) NOT NULL,
    digito_verificacion CHAR(1),

    nombre VARCHAR(200) NOT NULL,
    razon_social VARCHAR(200),

    telefono VARCHAR(30),
    telefono_alterno VARCHAR(30),
    email VARCHAR(150),
    direccion VARCHAR(200),
    departamento VARCHAR(80),
    municipio VARCHAR(80),
    contacto_nombre VARCHAR(150),
    contacto_telefono VARCHAR(30),

    banco VARCHAR(80),
    tipo_cuenta VARCHAR(20),
    numero_cuenta VARCHAR(30),
    titular_cuenta VARCHAR(200),
    titular_documento VARCHAR(30),

    observaciones VARCHAR(500),
    activo BOOLEAN NOT NULL DEFAULT TRUE,
    version INTEGER NOT NULL DEFAULT 1,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_proveedores_documento UNIQUE (tipo_documento, documento),
    CONSTRAINT proveedores_tipo_proveedor_check
        CHECK (tipo_proveedor IN ('materia_prima', 'insumo', 'servicio')),
    CONSTRAINT proveedores_tipo_persona_check
        CHECK (tipo_persona IN ('natural', 'juridica')),
    CONSTRAINT proveedores_tipo_cuenta_check
        CHECK (tipo_cuenta IS NULL OR tipo_cuenta IN ('ahorros', 'corriente'))
);

ALTER SEQUENCE proveedores_codigo_seq OWNED BY proveedores.codigo;

CREATE INDEX idx_proveedores_activo ON proveedores (activo);
CREATE INDEX idx_proveedores_tipo ON proveedores (tipo_proveedor);

CREATE TRIGGER trg_proveedores_actualizado_en
BEFORE UPDATE ON proveedores
FOR EACH ROW EXECUTE FUNCTION set_actualizado_en_version();
