-- Add migration script here
-- Básculas de la línea: cada una es un ESP32 que pesa canastas y lee la
-- tarjeta del operario. No hablan con el servidor: le mandan los pesajes a un
-- concentrador (Raspberry Pi), que es el que se autentica y los reenvía.

CREATE SEQUENCE basculas_codigo_seq;

CREATE TABLE basculas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- 'BS-0001': lo que se escribe en la etiqueta pegada a la báscula.
    codigo VARCHAR(20) NOT NULL UNIQUE
        DEFAULT ('BS-' || LPAD(nextval('basculas_codigo_seq')::text, 4, '0')),

    -- La MAC del ESP32 ('70:4b:ca:99:3f:30'): es el device_id que trae cada
    -- pesaje, y con esto se sabe de qué báscula vino. En minúsculas y con ':'
    -- porque así la manda el firmware. El CHECK es estricto a propósito: las
    -- filas se cargan a mano, y una MAC mal escrita no daría error en ningún
    -- lado — simplemente ningún pesaje encontraría su báscula.
    device_id VARCHAR(17) NOT NULL UNIQUE,

    -- Por qué Pi llegan sus pesajes. RESTRICT: no se borra una Pi que todavía
    -- tiene básculas colgando; se da de baja.
    concentrador_id UUID NOT NULL REFERENCES concentradores(id) ON DELETE RESTRICT,

    -- En qué línea está la báscula. Es lo que conecta el pesaje con
    -- programaciones_proceso: la pesada va al lote programado en ESTA línea a
    -- la hora del pesaje. Mismo tipo y mismo DEFAULT que programaciones_proceso.linea.
    linea SMALLINT NOT NULL DEFAULT 1,

    nombre VARCHAR(100) NOT NULL,       -- "Báscula entrada línea 1"
    ubicacion VARCHAR(200),
    descripcion VARCHAR(500),

    estado VARCHAR(20) NOT NULL DEFAULT 'activo',

    version INTEGER NOT NULL DEFAULT 1,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT basculas_device_id_check
        CHECK (device_id ~ '^([0-9a-f]{2}:){5}[0-9a-f]{2}$'),

    CONSTRAINT basculas_estado_check
        CHECK (estado IN ('activo', 'baja'))
);

ALTER SEQUENCE basculas_codigo_seq OWNED BY basculas.codigo;

CREATE INDEX idx_basculas_concentrador ON basculas (concentrador_id);

CREATE TRIGGER trg_basculas_actualizado_en
BEFORE UPDATE ON basculas
FOR EACH ROW EXECUTE FUNCTION set_actualizado_en_version();
