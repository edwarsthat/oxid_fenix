-- Libro de movimientos de materia prima: todo lo que SALE de un lote (y lo
-- que se le devuelve por corrección). Es la contracara de
-- ingresos_materia_prima, que solo guarda lo que entró.
--
-- El saldo de un lote es peso_ingreso + SUM(peso_efecto) de esta tabla. Ese
-- número es INFORMATIVO: no bloquea nada y puede quedar negativo. Con comida
-- no hay pesos exactos —el rechazo de un lote sale unas veces más y otras
-- menos que lo estimado— así que un lote no se cierra solo al llegar a cero:
-- lo cierra el encargado de proceso cuando decide que ese lote se acabó.
--
-- OJO: el comentario de `cerrado_en` en 20260831173159 dice que la marca la
-- escribe un trigger de esta tabla. Eso quedó viejo con esta decisión —
-- `cerrado_en` la escribe la aplicación cuando el encargado finaliza el lote,
-- y acá no hay trigger de saldo. Esa migración no se edita porque ya corrió
-- en el VPS y cambiarle el checksum abortaría el `migrate` de ese servidor.
--
-- Una fila NO se borra ni se anula: un libro donde las líneas desaparecen no
-- se puede auditar. El error de registro se corrige con un movimiento de
-- ajuste en sentido contrario, que deja las dos filas visibles.

-- La FK compuesta de más abajo necesita un índice único sobre el par
-- (id, lote_id) en el destino. `id` ya es PK, así que este UNIQUE no prohíbe
-- nada nuevo: existe solo para que Postgres acepte referenciar ese par.
--
-- Va acá y no dentro de 20260904144303 por la misma razón que la idempotencia
-- de ingresos fue en migración aparte: esa migración ya puede estar corrida, y
-- editarla le cambiaría el checksum abortando el `migrate` de ese servidor.
ALTER TABLE programaciones_proceso
    ADD CONSTRAINT programaciones_proceso_id_lote_key UNIQUE (id, lote_id);

CREATE TABLE movimientos_materia_prima (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Mismo patrón y misma razón que en ingresos: la tablet de báscula tiene
    -- señal intermitente, y una pesada duplicada no es un renglón de más —
    -- es peso que nunca salió restando del saldo del lote. Acá pesa todavía
    -- más que en ingresos, porque una pesada se repite decenas de veces por
    -- turno y no hay forma de distinguir el reintento de la estiba siguiente
    -- mirando los datos: 300 kg del mismo operario en el mismo lote es un
    -- valor perfectamente normal dos veces seguidas.
    --
    -- La genera el cliente por pesada (al capturar el peso, no al enviarlo).
    clave_idempotencia UUID NOT NULL UNIQUE,

    -- === Qué lote, y por qué se cree que es ese ===
    -- El par dato-resuelto/origen. lote_id es la respuesta; programacion_id
    -- es quién la dio. Cuando aparezcan pesadas en el lote equivocado —y van
    -- a aparecer— esto es lo único que permite reconstruir por qué el sistema
    -- creía lo que creía, en vez de mirar una fila que solo dice "lote A".
    lote_id UUID NOT NULL REFERENCES ingresos_materia_prima(id) ON DELETE RESTRICT,

    -- Sin REFERENCES propio: lo cubre la FK compuesta del final, que además
    -- obliga a que la programación sea la de ESTE lote. Con dos FKs sueltas
    -- nada impide una fila que diga lote A con la programación de un turno del
    -- lote B, y esa es exactamente la fila que este par promete poder
    -- reconstruir.
    programacion_id UUID,

    -- === Qué clase de salida ===
    -- VARCHAR con CHECK y no un ENUM nativo, como el resto del esquema:
    -- agregar un tipo es un ALTER del CHECK y no un ALTER TYPE que en
    -- Postgres no se puede revertir dentro de una transacción.
    --
    -- 'ajuste' viene partido en dos y no como un tipo con signo, porque el
    -- signo lo determina el tipo (ver peso_efecto). Un solo 'ajuste' obligaría
    -- a una columna de sentido aparte, y a que cada consulta de saldo la
    -- recuerde.
    tipo VARCHAR(24) NOT NULL,

    -- === Kilos ===
    -- Siempre positivo: el signo no es un dato que el cliente pueda mandar
    -- mal, se deriva del tipo. NUMERIC por la misma razón que en ingresos —
    -- esto termina en la liquidación del proveedor.
    peso NUMERIC(12,2) NOT NULL,

    -- El signo, calculado una sola vez acá y no en cada consulta. Es lo que
    -- deja el saldo como un SUM plano en vez de un CASE que hay que repetir
    -- (y algún día olvidar) en el reporte y en la pantalla de patio.
    --
    -- Los tipos van listados uno por uno y NO hay ELSE, a propósito: si mañana
    -- se agrega un tipo al CHECK y se olvida acá, el CASE devuelve NULL, choca
    -- con el NOT NULL de esta columna y el INSERT falla de una. Con un
    -- `ELSE -peso` ese tipo nuevo entraría restando en silencio y el saldo
    -- quedaría mal para siempre — el error que nadie encuentra hasta que no
    -- cuadra la liquidación del proveedor.
    peso_efecto NUMERIC(12,2) NOT NULL GENERATED ALWAYS AS (
        CASE
            WHEN tipo IN ('consumo', 'merma', 'devolucion_proveedor',
                          'ajuste_salida') THEN -peso
            WHEN tipo = 'ajuste_entrada' THEN peso
        END
    ) STORED,

    -- === Quién ===
    -- Apunta a personal y no a usuarios: el que pesa en la línea es un
    -- empleado con llave, no alguien con sesión en el sistema.
    --
    -- Nullable, con un CHECK por tipo más abajo en vez de NOT NULL: hay
    -- movimientos que no pasan por la línea. Un ajuste que hace el supervisor
    -- corrigiendo un dedazo no tiene operario, y obligarlo llevaría a que
    -- alguien ponga cualquier nombre con tal de que el formulario pase — un
    -- operario inventado es peor que la columna vacía, porque el dato falso no
    -- se distingue del bueno cuando alguien venga a revisar esa fila.
    --
    -- No confundir con registrado_por, que es la otra mitad y sí va siempre:
    -- una cosa es quién pesó y otra quién lo metió al sistema.
    operario_id UUID REFERENCES personal(id) ON DELETE RESTRICT,

    -- Quién lo metió al sistema, que no es lo mismo que quién pesó. En un
    -- consumo los dos son la misma persona en la práctica; en un ajuste el
    -- operario es el de la línea y quien decidió mover los kilos es el
    -- supervisor, y esa es justo la fila que alguien va a venir a revisar.
    --
    -- Sale de la sesión (`ctx.user_id`), nunca del payload. Va acá y no solo
    -- en audit_logs porque es parte de la línea del libro: leer el libro no
    -- debería obligar a un JOIN contra la auditoría.
    registrado_por UUID NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,

    -- Obligatorio en ajuste y devolución: son los que mueven kilos sin que
    -- haya pasado nada físico en la línea, y sin el porqué escrito no hay
    -- forma de revisarlos después.
    motivo VARCHAR(300),
    observaciones VARCHAR(500),

    -- Qué movimiento corrige este ajuste. El encabezado dice que un error no
    -- se borra sino que se compensa con un ajuste en sentido contrario; sin
    -- esta columna las dos filas quedan sueltas y no hay cómo saber cuál anula
    -- a cuál, ni ver que una misma pesada se corrigió dos veces.
    --
    -- La FK apunta a esta misma tabla: Postgres lo permite dentro del CREATE.
    corrige_movimiento_id UUID REFERENCES movimientos_materia_prima(id) ON DELETE RESTRICT,

    version INTEGER NOT NULL DEFAULT 1,
    -- `creado_en` es también la hora del movimiento: no hay un registrado_en
    -- aparte porque la pesada se registra cuando ocurre.
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT movimientos_materia_prima_tipo_check
        CHECK (tipo IN ('consumo', 'merma', 'devolucion_proveedor',
                        'ajuste_entrada', 'ajuste_salida')),

    CONSTRAINT movimientos_materia_prima_peso_check CHECK (peso > 0),

    -- La programación tiene que ser la de este lote, no la de cualquiera.
    -- MATCH SIMPLE (el default) no valida el par cuando programacion_id va en
    -- NULL, que es justo lo que se quiere para merma, devolución y ajustes.
    CONSTRAINT movimientos_materia_prima_programacion_fk
        FOREIGN KEY (programacion_id, lote_id)
        REFERENCES programaciones_proceso (id, lote_id) ON DELETE RESTRICT,

    -- Un consumo es una pesada de la línea: sin programación no se sabe de
    -- dónde salió el lote. Los otros tipos no pasan por la línea.
    CONSTRAINT movimientos_materia_prima_consumo_origen_check
        CHECK (tipo <> 'consumo' OR programacion_id IS NOT NULL),

    -- Y sin operario tampoco explica nada: una pesada de línea la hizo
    -- alguien. Va en un CHECK aparte del de arriba a propósito, para que el
    -- error de Postgres diga cuál de las dos falta y no solo que "algo" falta.
    --
    -- 'merma' queda por fuera a propósito: se pesa en línea muchas veces, pero
    -- también aparece revisando el patio después, y ahí no hay a quién anotar.
    -- Si en la planta la merma SIEMPRE la pesa un operario, este check debería
    -- decir tipo NOT IN ('consumo', 'merma').
    CONSTRAINT movimientos_materia_prima_operario_check
        CHECK (tipo <> 'consumo' OR operario_id IS NOT NULL),

    CONSTRAINT movimientos_materia_prima_motivo_check
        CHECK (tipo NOT IN ('ajuste_entrada', 'ajuste_salida', 'devolucion_proveedor')
               OR motivo IS NOT NULL),

    -- Solo un ajuste corrige algo: un consumo apuntando a otro movimiento no
    -- significa nada y sería un dato que después hay que aprender a ignorar.
    CONSTRAINT movimientos_materia_prima_correccion_check
        CHECK (corrige_movimiento_id IS NULL
               OR tipo IN ('ajuste_entrada', 'ajuste_salida'))
);

-- El índice del saldo, que es la consulta caliente: SUM(peso_efecto) de un
-- lote. El INCLUDE deja que salga del índice sin tocar la tabla.
CREATE INDEX idx_movimientos_mp_lote
    ON movimientos_materia_prima (lote_id) INCLUDE (peso_efecto);

-- El turno: qué se pesó en la línea mientras estuvo montado ese lote.
CREATE INDEX idx_movimientos_mp_programacion
    ON movimientos_materia_prima (programacion_id)
    WHERE programacion_id IS NOT NULL;

-- Las dos consultas del día: el libro cronológico, y la última pesada de un
-- operario — que es la que necesita la regla de tiempo mínimo entre
-- registros. Esa regla va en el servicio y no acá: es política, cambia sin
-- migración, y necesita mirar otra fila.
CREATE INDEX idx_movimientos_mp_fecha
    ON movimientos_materia_prima (creado_en DESC);
-- Parcial de nuevo ahora que operario_id puede ir en NULL: los ajustes sin
-- operario no aportan nada a "la última pesada de Juan" y solo engordarían
-- el índice.
CREATE INDEX idx_movimientos_mp_operario
    ON movimientos_materia_prima (operario_id, creado_en DESC)
    WHERE operario_id IS NOT NULL;

-- "¿esta pesada ya fue corregida?". Parcial porque la enorme mayoría de las
-- filas no corrige nada: el índice se queda del tamaño de los errores, no de
-- la tabla.
CREATE INDEX idx_movimientos_mp_correccion
    ON movimientos_materia_prima (corrige_movimiento_id)
    WHERE corrige_movimiento_id IS NOT NULL;

CREATE TRIGGER trg_movimientos_materia_prima_actualizado_en
BEFORE UPDATE ON movimientos_materia_prima
FOR EACH ROW EXECUTE FUNCTION set_actualizado_en_version();
