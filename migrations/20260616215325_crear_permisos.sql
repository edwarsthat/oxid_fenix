-- Add migration script here
CREATE TABLE permisos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    nombre VARCHAR(100) NOT NULL UNIQUE,
    descripcion VARCHAR(255),
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW()
);