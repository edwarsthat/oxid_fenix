-- Add migration script here

CREATE TABLE programaciones_proceso (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    lote_id UUID NOT NULL REFERENCES ingresos_materia_prima(id) ON DELETE RESTRICT,

    -- Hoy hay una sola línea, y por eso no hay catálogo de líneas: sería una
    -- tabla de una fila para sostener una constante. La columna sí va, y va
    -- ahora, porque es la que ancla el índice de "una sola abierta". El día
    -- que exista la segunda línea esto se vuelve un FK y el índice de abajo
    -- no se toca; sin la columna, ese día habría que reescribir el invariante
    -- con filas adentro.
    linea SMALLINT NOT NULL DEFAULT 1,

    -- El intervalo. fin_en en NULL = es lo que se está procesando ahora.
    -- De acá sale el lote_id de cada pesada: la programación cuyo intervalo
    -- contiene el instante en que la báscula mandó el peso.
    inicio_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    fin_en TIMESTAMPTZ,

    -- Un lote puede aparecer varias veces: se monta, se para por un daño, se
    -- retoma después. Por eso el UNIQUE no va sobre lote_id.
    programado_por UUID NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
    cerrado_por UUID REFERENCES usuarios(id) ON DELETE RESTRICT,

    observaciones VARCHAR(500),

    version INTEGER NOT NULL DEFAULT 1,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT programaciones_proceso_fechas_check
        CHECK (fin_en IS NULL OR fin_en >= inicio_en),

    -- Igual que la anulación del ingreso: el cierre va completo o no va.
    CONSTRAINT programaciones_proceso_cierre_completo_check
        CHECK (num_nonnulls(fin_en, cerrado_por) IN (0, 2))
);

-- El invariante: una sola programación abierta por línea. Es la red contra
-- el olvido del coordinador — programar sin cerrar la anterior dejaría dos
-- lotes candidatos para la misma pesada y ninguna forma de elegir.
CREATE UNIQUE INDEX ux_programaciones_proceso_abierta
    ON programaciones_proceso (linea) WHERE fin_en IS NULL;

CREATE INDEX idx_programaciones_proceso_lote ON programaciones_proceso (lote_id);
CREATE INDEX idx_programaciones_proceso_inicio ON programaciones_proceso (inicio_en DESC);

CREATE TRIGGER trg_programaciones_proceso_actualizado_en
BEFORE UPDATE ON programaciones_proceso
FOR EACH ROW EXECUTE FUNCTION set_actualizado_en_version();
