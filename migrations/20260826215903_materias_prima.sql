-- Catálogo de materias primas: datos casi constantes (pocas filas).
-- La app solo lee; los cambios se hacen editando el seed y corriendo
-- `cargo run --bin seeds`, no con UPDATE sueltos en psql.
CREATE TABLE materias_primas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Código manual y legible ('PLA', 'YUC'): son pocos y se siembran desde
    -- el seed, así que no hay secuencia como en proveedores o llaves_nfc.
    codigo VARCHAR(20) NOT NULL UNIQUE,
    nombre VARCHAR(100) NOT NULL,

    -- Lo único que de verdad opera distinto entre plátano y yuca hoy.
    -- NULL en horas_maximas_espera = sin límite (plátano aguanta días,
    -- la yuca se deteriora en ~48h).
    horas_maximas_espera INTEGER,
    rendimiento_esperado_pct NUMERIC(5,2),
    -- El rango vive en la tabla, no hardcodeado en Rust: es lo que se ajusta
    -- por temporada para detectar un proceso fuera de rango.
    rendimiento_min_pct NUMERIC(5,2),
    rendimiento_max_pct NUMERIC(5,2),

    activo BOOLEAN NOT NULL DEFAULT TRUE,
    version INTEGER NOT NULL DEFAULT 1,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_materias_primas_nombre UNIQUE (nombre),

    -- Los CHECK son la única red: al editarse a mano no pasa por validación
    -- de Rust. NUMERIC(5,2) por sí solo aceptaría hasta 999.99.
    CONSTRAINT materias_primas_horas_espera_check
        CHECK (horas_maximas_espera IS NULL OR horas_maximas_espera > 0),
    CONSTRAINT materias_primas_rendimiento_check
        CHECK (rendimiento_esperado_pct IS NULL
               OR rendimiento_esperado_pct BETWEEN 0 AND 100),
    CONSTRAINT materias_primas_rendimiento_min_check
        CHECK (rendimiento_min_pct IS NULL
               OR rendimiento_min_pct BETWEEN 0 AND 100),
    CONSTRAINT materias_primas_rendimiento_max_check
        CHECK (rendimiento_max_pct IS NULL
               OR rendimiento_max_pct BETWEEN 0 AND 100),
    CONSTRAINT materias_primas_rendimiento_rango_check
        CHECK (rendimiento_min_pct IS NULL OR rendimiento_max_pct IS NULL
               OR rendimiento_min_pct <= rendimiento_max_pct)
);

-- Sin índice en activo a propósito: la tabla es de pocas filas y Postgres
-- hace seq scan igual, a diferencia de proveedores o personal.

CREATE TRIGGER trg_materias_primas_actualizado_en
BEFORE UPDATE ON materias_primas
FOR EACH ROW EXECUTE FUNCTION set_actualizado_en_version();
