-- Recepción de materia prima: un lote que llega del predio a la planta.
--
-- Esta tabla es la cabeza de la trazabilidad. Todo lo que pase después
-- (proceso, rendimiento, despacho, liquidación al proveedor) se cuelga de
-- una fila de acá, así que una fila nunca se borra: se anula.
--
-- Un ingreso = un lote = UNA materia prima. Si un camión trae plátano y
-- yuca son dos ingresos con el mismo numero_tiquete_bascula; mezclarlos en
-- una fila rompería el cálculo de rendimiento por materia prima.
--
-- El nombre lleva 'materia_prima' porque 'ingresos' a secas es una palabra
-- que van a querer también los insumos, los empaques y los repuestos: en el
-- namespace plano de Postgres la palabra genérica no puede ser de una sola
-- tabla.
--
-- La tabla guarda solo lo que ENTRÓ. El saldo en patio no vive acá: sale de
-- restarle a peso_ingreso los consumos registrados en el libro de
-- movimientos. Por eso ninguna columna de esta tabla cambia después del
-- registro, salvo `cerrado_en` (que la pone el trigger del libro) y la
-- anulación.

CREATE SEQUENCE ingresos_materia_prima_codigo_seq;

-- El código va en función y no en un DEFAULT inline por dos razones.
--
-- La primera es que LPAD no sirve acá: LPAD TRUNCA cuando el número no cabe
-- en el ancho pedido, así que LPAD(100000::text, 5, '0') devuelve '10000' y
-- choca con el código del ingreso número 10.000. Con REPEAT se rellena
-- hasta cinco dígitos y de ahí en adelante el número simplemente crece.
--
-- La segunda es que ese REPEAT necesita el valor de la secuencia dos veces
-- (para medirlo y para escribirlo). Inline habría que llamar nextval() y
-- currval() dentro del mismo '||', y el orden de evaluación de una
-- expresión no está garantizado. Acá nextval() se llama una sola vez y
-- queda en una variable.
CREATE FUNCTION ingresos_materia_prima_codigo_nuevo() RETURNS VARCHAR AS $$
DECLARE
    consecutivo TEXT := nextval('ingresos_materia_prima_codigo_seq')::text;
BEGIN
    RETURN 'LT-' || to_char(NOW(), 'YYYYMM') || '-'
           || REPEAT('0', GREATEST(0, 5 - length(consecutivo)))
           || consecutivo;
END;
$$ LANGUAGE plpgsql VOLATILE;

CREATE TABLE ingresos_materia_prima (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- 'LT-202608-00001'. Es el número que se escribe en la estiba y en el
    -- tiquete de báscula, por eso no basta el UUID.
    --
    -- Año y mes en ese orden (YYYYMM), no al revés: así el orden alfabético
    -- del código es el cronológico y un ORDER BY codigo queda bien sin
    -- pensarlo. Con MMYYYY, agosto de 2025 se mezclaría con agosto de 2026.
    --
    -- El consecutivo NO se reinicia con el mes: LT-202609-00450 sigue
    -- después de LT-202608-00449. El periodo es para leer, no para contar.
    -- Reiniciarlo pediría una tarea mensual que algún día 1 no va a correr,
    -- y el costo sería un código duplicado en producción.
    --
    -- Si algún día se necesita que cada mes arranque en 1, eso NO se hace
    -- reiniciando la secuencia sino con una tabla de consecutivos por
    -- (ámbito, periodo) e incremento atómico con ON CONFLICT.
    codigo VARCHAR(20) NOT NULL UNIQUE DEFAULT ingresos_materia_prima_codigo_nuevo(),

    -- === Origen ===
    -- El proveedor sale del predio (predios.proveedor_id), no se repite acá.
    predio_id UUID NOT NULL REFERENCES predios(id) ON DELETE RESTRICT,

    materia_prima_id UUID NOT NULL REFERENCES materias_primas(id) ON DELETE RESTRICT,

    -- === Transporte ===
    -- Normalizada en mayúsculas y sin guiones desde Rust. El CHECK es
    -- deliberadamente laxo (5 a 7 alfanuméricos) porque acá entran placas de
    -- camión (ABC123), de remolque (R12345) y de moto (ABC12D); una regex
    -- estricta terminaría bloqueando un ingreso real a las 5am.
    placa VARCHAR(10) NOT NULL,
    numero_remision VARCHAR(50),
    -- Sin UNIQUE: un mismo tiquete cubre los dos ingresos de un camión que
    -- trajo dos materias primas.
    numero_tiquete_bascula VARCHAR(50),

    -- === Tiempos ===
    -- fecha_ingreso va aparte del timestamp porque los reportes agrupan por
    -- día y un DATE indexado evita andar casteando en cada consulta.
    fecha_ingreso DATE NOT NULL DEFAULT CURRENT_DATE,
    llegada_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    inicio_descargue_en TIMESTAMPTZ,
    fin_descargue_en TIMESTAMPTZ,

    -- === Pesos (kg) ===
    -- NUMERIC y no float: acá se redondea plata contra el proveedor y las
    -- diferencias de binario se vuelven una discusión en la báscula.
    peso_ingreso NUMERIC(12,2) NOT NULL,

    -- Lo que se rechazó en portería y se devolvió en el mismo camión. Es
    -- informativo: nunca entró, así que ya está fuera de peso_ingreso y no
    -- toca el saldo. El porqué de la devolución va en observaciones.
    peso_devuelto NUMERIC(12,2) NOT NULL DEFAULT 0,

    -- === Estado ===
    -- NULL = el lote todavía tiene saldo en patio.
    --
    -- No es la verdad, es un atajo: el saldo real es peso_ingreso menos la
    -- suma de los consumos del libro de movimientos. Esta marca existe para
    -- que la consulta de patio no tenga que calcular el saldo de todos los
    -- lotes de la historia solo para descartarlos — con un millón de filas,
    -- leer 300 en vez del millón es la diferencia entre 1 ms y varios
    -- segundos. Si algún día la marca y la resta no coinciden, manda la
    -- resta.
    --
    -- La escribe el trigger de movimientos_materia_prima cuando el saldo
    -- llega a cero, nunca la aplicación a mano. Y como un lote casi nunca
    -- se consume al 100% exacto (siempre quedan kilos de merma en el
    -- fondo), el cierre normal va a venir de un movimiento de tipo 'merma'
    -- que lleva el saldo a cero, no de que el último consumo cuadre justo.
    cerrado_en TIMESTAMPTZ,

    -- === Auditoría ===
    observaciones VARCHAR(500),
    registrado_por UUID NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,

    -- Un ingreso no se borra: borrarlo dejaría producto terminado sin
    -- origen. Anular deja el rastro de quién y por qué.
    anulado_en TIMESTAMPTZ,
    anulado_por UUID REFERENCES usuarios(id) ON DELETE RESTRICT,
    motivo_anulacion VARCHAR(300),

    version INTEGER NOT NULL DEFAULT 1,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- === Transporte ===
    CONSTRAINT ingresos_materia_prima_placa_check
        CHECK (placa ~ '^[A-Z0-9]{5,7}$'),

    -- === Pesos coherentes ===
    CONSTRAINT ingresos_materia_prima_peso_ingreso_check CHECK (peso_ingreso > 0),
    CONSTRAINT ingresos_materia_prima_peso_devuelto_check CHECK (peso_devuelto >= 0),
    -- === Tiempos ordenados ===
    CONSTRAINT ingresos_materia_prima_inicio_descargue_check
        CHECK (inicio_descargue_en IS NULL OR inicio_descargue_en >= llegada_en),
    CONSTRAINT ingresos_materia_prima_fin_descargue_check
        CHECK (fin_descargue_en IS NULL
               OR (inicio_descargue_en IS NOT NULL
                   AND fin_descargue_en >= inicio_descargue_en)),

    -- === Anulación ===
    -- Va completa o no va: quién, cuándo y por qué. Media anulación deja una
    -- fila que nadie puede auditar después.
    CONSTRAINT ingresos_materia_prima_anulacion_completa_check
        CHECK (num_nonnulls(anulado_en, anulado_por, motivo_anulacion) IN (0, 3))
);

ALTER SEQUENCE ingresos_materia_prima_codigo_seq OWNED BY ingresos_materia_prima.codigo;

-- Los índices van con el prefijo abreviado 'idx_ingresos_mp_': el nombre
-- completo de la tabla dejaría cosas como
-- idx_ingresos_materia_prima_materia_prima_id. Los CHECK sí llevan el
-- nombre entero porque son los que salen en los mensajes de error, y ahí
-- la claridad vale más que la brevedad.
--
-- La consulta de todos los días: los ingresos de hoy, o del mes para el
-- reporte. DESC porque siempre se mira lo más reciente primero.
CREATE INDEX idx_ingresos_mp_fecha ON ingresos_materia_prima (fecha_ingreso DESC);

-- Historial por origen: "qué me ha entregado este predio", que es lo que se
-- revisa antes de liquidarle al proveedor. Por proveedor se llega uniendo
-- contra predios, que ya tiene su propio idx_predios_proveedor.
CREATE INDEX idx_ingresos_mp_predio ON ingresos_materia_prima (predio_id);
CREATE INDEX idx_ingresos_mp_materia_prima ON ingresos_materia_prima (materia_prima_id);

-- El índice de la pantalla de patio, y el que hace que el inventario no se
-- degrade con los años: al ser parcial solo contiene los lotes abiertos, y
-- un lote SALE del índice cuando se cierra. O sea que se queda del tamaño
-- del patio (decenas, cientos) por más que la tabla llegue a millones.
--
-- Sirve para las dos consultas de esa pantalla: kilos por materia prima
-- (condición sobre la primera columna) y el orden FEFO por llegada. Con
-- tan pocas filas, ordenar sale gratis aunque no lo resuelva el índice.
CREATE INDEX idx_ingresos_mp_patio
    ON ingresos_materia_prima (materia_prima_id, llegada_en)
    WHERE cerrado_en IS NULL AND anulado_en IS NULL;

-- Para cruzar contra el tiquete físico cuando algo no cuadra en báscula.
CREATE INDEX idx_ingresos_mp_tiquete ON ingresos_materia_prima (numero_tiquete_bascula)
    WHERE numero_tiquete_bascula IS NOT NULL;

CREATE TRIGGER trg_ingresos_materia_prima_actualizado_en
BEFORE UPDATE ON ingresos_materia_prima
FOR EACH ROW EXECUTE FUNCTION set_actualizado_en_version();
