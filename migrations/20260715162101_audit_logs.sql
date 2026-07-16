-- Add migration script here
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type TEXT NOT NULL,      -- 'cargo', 'permiso', 'empleado', etc.
    entity_id UUID NOT NULL,
    action TEXT NOT NULL,           -- 'add', 'update', 'delete'
    actor_id UUID NOT NULL REFERENCES usuarios(id),  -- quién hizo el cambio (usuario)
    area TEXT,                      -- si quieres registrar el área/contexto
    changes JSONB,                  -- diff o snapshot de los datos
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);