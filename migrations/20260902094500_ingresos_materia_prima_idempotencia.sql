-- Clave de idempotencia del alta de un ingreso.
--
-- Va en migración aparte y no dentro de 20260831173159 porque esa ya puede
-- estar corrida en algún lado: editarla le cambiaría el checksum y el
-- `migrate` de ese servidor abortaría en vez de aplicar nada. Si la tabla
-- todavía no existe en producción, esta migración se puede plegar sobre la
-- otra sin consecuencias.
--
-- El problema que resuelve: el de báscula registra desde una tablet con
-- señal intermitente. Si el INSERT se hace y la respuesta se pierde, el
-- cliente reintenta y sin esta columna entran DOS lotes con el mismo
-- camión. Y un lote duplicado no es un renglón de más: es peso que nunca
-- llegó sumando al saldo de patio y a la liquidación del proveedor.
--
-- La clave la genera el cliente una vez por ingreso (al abrir el
-- formulario, no al enviarlo) y la repite en cada reintento. El alta hace
-- ON CONFLICT DO NOTHING y, cuando no inserta, devuelve el lote que ya
-- existía: el reintento le sale al usuario como un éxito, no como un error.
--
-- UUID y no texto libre para que dos terminales no puedan mandar la misma
-- clave por casualidad ("1", "ingreso-hoy") y una termine tapando el
-- ingreso de la otra.

-- El DEFAULT es solo para poblar las filas que ya estuvieran en la tabla —
-- se registraron antes de que existiera la idempotencia, así que a cada una
-- le toca su propia clave — y se quita enseguida: de acá en adelante la
-- clave la pone el cliente o el INSERT falla, que es justo lo que se busca.
ALTER TABLE ingresos_materia_prima
    ADD COLUMN clave_idempotencia UUID NOT NULL DEFAULT gen_random_uuid();

ALTER TABLE ingresos_materia_prima
    ALTER COLUMN clave_idempotencia DROP DEFAULT;

-- El UNIQUE es la garantía de verdad: el ON CONFLICT del alta se apoya en
-- este índice, y sin él dos reintentos en paralelo (la tablet reintentando
-- mientras la primera petición todavía va en camino) se colarían los dos.
ALTER TABLE ingresos_materia_prima
    ADD CONSTRAINT ingresos_materia_prima_clave_idempotencia_key
        UNIQUE (clave_idempotencia);
