-- Add migration script here
CREATE TABLE cargos_permisos (
    cargo_id UUID NOT NULL REFERENCES cargos(id) ON DELETE CASCADE,
    permiso_id UUID NOT NULL REFERENCES permisos(id) ON DELETE CASCADE,
    PRIMARY KEY (cargo_id, permiso_id)
);