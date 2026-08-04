-- Add migration script here
CREATE TABLE cargos_personal (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    nombre VARCHAR(80) NOT NULL UNIQUE,
    tipo_contrato VARCHAR(50),
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    activo BOOLEAN NOT NULL DEFAULT TRUE
);