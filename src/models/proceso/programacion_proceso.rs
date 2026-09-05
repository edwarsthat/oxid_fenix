use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::{ValidacionError, Validar, texto_opcional, uuid_requerido};

#[derive(Debug, FromRow, Serialize)]
pub struct ProgramacionProceso {
    pub id: Uuid,
    pub lote_id: Uuid,

    // SMALLINT en la tabla, que el driver decodifica como i16. Con i32 el
    // SELECT no falla al compilar sino al leer la fila, ya en producción.
    pub linea: i16,

    // El intervalo. `fin_en` en NULL —y con él `cerrado_por`— es el caso
    // normal, no el raro: es la programación que está corriendo ahora, la que
    // le da el lote a cada pesada.
    pub inicio_en: DateTime<Utc>,
    pub fin_en: Option<DateTime<Utc>>,

    pub programado_por: Uuid,
    pub cerrado_por: Option<Uuid>,

    pub observaciones: Option<String>,

    pub version: i32,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
}

/// Lo que devuelve el alta: la programación, y si de verdad se creó en esta
/// llamada.
///
/// El `bool` es el mismo de `AltaIngreso` y está por la misma razón, aunque
/// esta tabla no tenga clave de idempotencia. Una programación es un estado
/// ("la línea está en el lote A"), no un evento: programar dos veces el mismo
/// lote es el mismo hecho dicho dos veces, así que el reintento devuelve la
/// programación que ya estaba abierta. Sin distinguirlo, cada reintento
/// escribiría otro renglón en la auditoría y volvería a emitir el evento.
#[derive(Debug)]
pub struct AltaProgramacionProceso {
    pub programacion: ProgramacionProceso,
    pub creado: bool,
}

/// Lo que manda el coordinador de turno: qué lote se monta en la línea.
///
/// `lote_id` llega como texto igual que en `ingresos` y `predios`: si se
/// declarara como `Uuid`, un valor mal escrito lo rechazaría serde con un
/// "invalid value" en inglés antes de que `validar()` alcance a decir cuál
/// campo es y qué se esperaba.
///
/// No trae `inicio_en` ni `linea`: las dos tienen DEFAULT en la tabla (`NOW()`
/// y `1`), y la hora la pone el reloj del servidor de base y no el del que
/// programa. `programado_por` tampoco viaja acá: sale de la sesión
/// (`ctx.user_id`), no de lo que diga el cliente.
///
/// Cerrar la programación anterior tampoco es un campo del payload: lo hace el
/// servicio en la misma transacción del alta, porque el invariante de "una
/// sola abierta" es de la tabla entera y no de esta fila.
#[derive(Debug, Deserialize)]
pub struct ProgramacionProcesoAddPayload {
    pub lote_id: Option<String>,
    pub observaciones: Option<String>,
}

/// Lo que devuelve validar() y consume el servicio: si tenés un
/// `ProgramacionProcesoNuevo`, los datos ya están normalizados y son válidos.
///
/// Las dos reglas que faltan —que el lote exista, y que no esté anulado ni ya
/// cerrado— no se pueden verificar acá porque necesitan mirar otra fila. Van
/// en el servicio, dentro de la transacción del alta.
#[derive(Debug)]
pub struct ProgramacionProcesoNuevo {
    pub lote_id: Uuid,
    pub observaciones: Option<String>,
}

impl Validar for ProgramacionProcesoAddPayload {
    type Datos = ProgramacionProcesoNuevo;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        let lote_id = uuid_requerido(self.lote_id, "lote_id")?;

        // El largo es el de la migración: se corta acá para que Postgres no
        // devuelva un 22001 (value too long) como error 500.
        let observaciones = texto_opcional(self.observaciones.as_deref(), "observaciones", 500)?;

        Ok(ProgramacionProcesoNuevo {
            lote_id,
            observaciones,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_payload() -> ProgramacionProcesoAddPayload {
        ProgramacionProcesoAddPayload {
            lote_id: Some(Uuid::new_v4().to_string()),
            observaciones: None,
        }
    }

    #[test]
    fn validar_normaliza_y_devuelve_los_datos() {
        let lote_id = Uuid::new_v4();
        let payload = ProgramacionProcesoAddPayload {
            lote_id: Some(format!("  {lote_id}  ")),
            observaciones: Some("  se monta despues del almuerzo  ".into()),
        };

        let nuevo = payload.validar().expect("deberia ser valido");

        assert_eq!(nuevo.lote_id, lote_id);
        assert_eq!(
            nuevo.observaciones.as_deref(),
            Some("se monta despues del almuerzo")
        );
    }

    #[test]
    fn los_opcionales_en_blanco_quedan_en_none() {
        let payload = ProgramacionProcesoAddPayload {
            observaciones: Some("   ".into()),
            ..add_payload()
        };

        let nuevo = payload.validar().expect("deberia ser valido");

        assert_eq!(nuevo.observaciones, None);
    }

    /// Sin lote no hay qué programar: el error lo damos nosotros en español y
    /// diciendo cuál campo falta, no serde con un "missing field".
    #[test]
    fn validar_rechaza_el_lote_ausente_o_invalido() {
        let payload = ProgramacionProcesoAddPayload {
            lote_id: None,
            ..add_payload()
        };

        assert_eq!(payload.validar().unwrap_err().mensaje(), "falta el lote_id");

        let payload = ProgramacionProcesoAddPayload {
            lote_id: Some("no-es-un-uuid".into()),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el lote_id no es un UUID válido"
        );
    }

    /// El máximo es el de la columna VARCHAR(500): cortarlo acá es lo que
    /// convierte un 22001 del driver en un 400 que dice qué corregir.
    #[test]
    fn validar_rechaza_observaciones_muy_largas() {
        let payload = ProgramacionProcesoAddPayload {
            observaciones: Some("a".repeat(501)),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el campo observaciones no puede superar 500 caracteres"
        );
    }
}
