-- Add migration script here

CREATE SEQUENCE predios_codigo_seq;

CREATE TABLE predios (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    codigo VARCHAR(20) NOT NULL UNIQUE
        DEFAULT ('PD-' || LPAD(nextval('predios_codigo_seq')::text, 4, '0')),

    proveedor_id UUID NOT NULL REFERENCES proveedores(id) ON DELETE RESTRICT,
    nombre VARCHAR(200) NOT NULL,

    departamento VARCHAR(80) NOT NULL,
    municipio VARCHAR(80) NOT NULL,
    vereda VARCHAR(120),
    referencia_ubicacion VARCHAR(300),

    -- Grados decimales, como los pide el ICA. Opcionales: hay predios que se
    -- cargan sin haber ido a tomar el punto. NUMERIC(9,6) da ~11 cm de
    -- resolución, de sobra para ubicar un lote.
    latitud NUMERIC(9,6),
    longitud NUMERIC(9,6),

    responsable_nombre VARCHAR(150),
    responsable_documento VARCHAR(30),
    responsable_telefono VARCHAR(30),

    observaciones VARCHAR(500),
    activo BOOLEAN NOT NULL DEFAULT TRUE,
    version INTEGER NOT NULL DEFAULT 1,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- No obligan a cargar la coordenada, pero si va, va completa y dentro de
    -- rango: media coordenada no ubica nada y una latitud de 200 es un dedazo.
    CONSTRAINT predios_coordenadas_completas_check
        CHECK ((latitud IS NULL) = (longitud IS NULL)),
    CONSTRAINT predios_latitud_check
        CHECK (latitud IS NULL OR latitud BETWEEN -90 AND 90),
    CONSTRAINT predios_longitud_check
        CHECK (longitud IS NULL OR longitud BETWEEN -180 AND 180)
);

ALTER SEQUENCE predios_codigo_seq OWNED BY predios.codigo;

-- Dos fincas con el mismo nombre bajo el mismo proveedor son la misma finca
-- cargada dos veces. Por lower() porque "La Esperanza" y "LA ESPERANZA" es el
-- mismo predio escrito por dos personas distintas.
CREATE UNIQUE INDEX uq_predios_proveedor_nombre ON predios (proveedor_id, lower(nombre));

-- El acceso de todos los días: los predios de un proveedor al momento de
-- registrar una recepción.
CREATE INDEX idx_predios_proveedor ON predios (proveedor_id);
CREATE INDEX idx_predios_activo ON predios (activo);

CREATE TRIGGER trg_predios_actualizado_en
BEFORE UPDATE ON predios
FOR EACH ROW EXECUTE FUNCTION set_actualizado_en_version();
