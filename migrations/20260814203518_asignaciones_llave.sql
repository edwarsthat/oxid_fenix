-- Add migration script here
CREATE TABLE asignaciones_llave (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    llave_id UUID NOT NULL REFERENCES llaves_nfc (id),
    empleado_id UUID NOT NULL REFERENCES personal (id),

    asignada_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    devuelta_en TIMESTAMPTZ,
    motivo_devolucion VARCHAR(30),

    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT asignaciones_llave_fechas_check
        CHECK (devuelta_en IS NULL OR devuelta_en >= asignada_en),

    CONSTRAINT asignaciones_llave_motivo_check
        CHECK ((devuelta_en IS NULL) = (motivo_devolucion IS NULL)),

    CONSTRAINT asignaciones_llave_motivo_valido_check
        CHECK (motivo_devolucion IS NULL OR motivo_devolucion IN
            ('devolucion', 'perdida', 'dannada', 'retiro', 'reemplazo'))
);

CREATE UNIQUE INDEX ux_asignaciones_llave_activa
    ON asignaciones_llave (llave_id) WHERE devuelta_en IS NULL;
CREATE UNIQUE INDEX ux_asignaciones_empleado_activa
    ON asignaciones_llave (empleado_id) WHERE devuelta_en IS NULL;

CREATE INDEX idx_asignaciones_llave ON asignaciones_llave (llave_id);
CREATE INDEX idx_asignaciones_empleado ON asignaciones_llave (empleado_id);
