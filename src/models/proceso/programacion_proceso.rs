use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::{
    Rango, ValidacionError, Validar, limitar, texto_opcional, uuid_requerido,
};

/// Filas por página del historial cuando el cliente no manda `limite`. Más
/// bajo que el default general: son filas anchas, con nombres y sumas, y la
/// pantalla muestra unas pocas a la vez.
const LIMITE_HISTORIAL_POR_DEFECTO: i64 = 50;

#[derive(Debug, FromRow, Serialize)]
pub struct ProgramacionProceso {
    pub id: Uuid,
    pub lote_id: Uuid,

    pub linea: i16,

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

/// Posición en el historial: el (inicio_en, id) de la última fila que recibió
/// el cliente. El `id` desempata dos programaciones del mismo instante, que
/// con una sola línea no debería pasar pero no cuesta nada cubrir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct CursorProgramacion {
    pub inicio_en: DateTime<Utc>,
    pub id: Uuid,
}

/// Lo que manda el cliente para el historial. Todo es opcional: sin nada trae
/// la primera página de lo más reciente.
///
/// `lote_id` va como `Uuid` y no como texto (a diferencia del alta) por lo
/// mismo que en los pesajes: es un filtro que el front saca de una lista, no
/// algo que alguien escribe a mano.
#[derive(Debug, Deserialize)]
pub struct ProgramacionesReadPayload {
    /// Instantes UTC que el front ya calculó para el día local, igual que en
    /// los pesajes: el servidor no corta por `::date`.
    pub rango: Option<Rango<DateTime<Utc>>>,
    pub lote_id: Option<Uuid>,
    pub limite: Option<i64>,
    pub cursor: Option<CursorProgramacion>,
}

impl Validar for ProgramacionesReadPayload {
    type Datos = ProgramacionesFiltros;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        // Sin tope de días: son pocas filas por día y el cursor ya corta la
        // página. Solo se revisa que el rango no venga al revés.
        let (desde, hasta) = match self.rango {
            Some(rango) => {
                let rango = rango.validar()?;
                (rango.desde(), rango.hasta())
            }
            None => (None, None),
        };

        let limite = limitar(Some(self.limite.unwrap_or(LIMITE_HISTORIAL_POR_DEFECTO)));

        Ok(ProgramacionesFiltros {
            desde,
            hasta,
            lote_id: self.lote_id,
            limite,
            cursor: self.cursor,
        })
    }
}

/// Lo que consume `get_historial_programaciones`.
///
/// El rango es opcional a diferencia del de los pesajes: acá hay unas pocas
/// filas por día y no hay totales que sin fechas no signifiquen nada, así que
/// el cursor solo alcanza para no traer la tabla entera.
#[derive(Debug)]
pub struct ProgramacionesFiltros {
    /// Inclusivo. Se compara contra el intervalo entero, no contra `inicio_en`.
    pub desde: Option<DateTime<Utc>>,
    /// Exclusivo.
    pub hasta: Option<DateTime<Utc>>,
    /// Todas las veces que se montó un lote: se monta, se para, se retoma.
    pub lote_id: Option<Uuid>,
    pub limite: i64,
    pub cursor: Option<CursorProgramacion>,
}

/// Una fila del historial: la programación más lo que hace falta para leerla
/// sin ir a otras pantallas (qué lote, de dónde, quién lo montó y lo cerró) y
/// lo que se pesó mientras estuvo montada.
#[derive(Debug, FromRow, Serialize)]
pub struct ProgramacionHistorial {
    pub id: Uuid,
    pub lote_id: Uuid,
    pub linea: i16,
    /// `fin_en` en `None` es lo que está en la línea ahora.
    pub inicio_en: DateTime<Utc>,
    pub fin_en: Option<DateTime<Utc>>,
    pub observaciones: Option<String>,

    pub lote_codigo: String,
    pub materia_prima_nombre: String,
    pub predio_nombre: String,

    pub programado_por_nombre: String,
    pub programado_por_apellido: String,
    pub cerrado_por_nombre: Option<String>,
    pub cerrado_por_apellido: Option<String>,

    /// Pesajes de báscula colgados de esta programación.
    pub cantidad_pesajes: i64,
    /// Brutos, igual que los totales del destajo: no restan los ajustes que
    /// corrigen estos pesajes.
    pub kg_pesados: f64,
}

/// Una página del historial. `siguiente_cursor` en `None` es la última página.
#[derive(Debug, Serialize)]
pub struct PaginaProgramaciones {
    pub programaciones: Vec<ProgramacionHistorial>,
    pub siguiente_cursor: Option<CursorProgramacion>,
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

    fn read_payload() -> ProgramacionesReadPayload {
        ProgramacionesReadPayload {
            rango: None,
            lote_id: None,
            limite: None,
            cursor: None,
        }
    }

    #[test]
    fn historial_sin_filtros_usa_el_limite_del_historial() {
        let filtros = read_payload().validar().expect("deberia ser valido");

        assert_eq!(filtros.desde, None);
        assert_eq!(filtros.hasta, None);
        assert_eq!(filtros.limite, LIMITE_HISTORIAL_POR_DEFECTO);
    }

    #[test]
    fn historial_acota_el_limite() {
        let payload = ProgramacionesReadPayload {
            limite: Some(0),
            ..read_payload()
        };
        assert_eq!(payload.validar().unwrap().limite, 1);

        let payload = ProgramacionesReadPayload {
            limite: Some(1_000_000),
            ..read_payload()
        };
        assert_eq!(
            payload.validar().unwrap().limite,
            crate::models::validations::LIMITE_MAXIMO
        );
    }

    #[test]
    fn historial_rechaza_el_rango_al_reves() {
        let payload = ProgramacionesReadPayload {
            rango: Some(Rango {
                desde: Some("2026-11-02T05:00:00Z".parse().unwrap()),
                hasta: Some("2026-11-01T05:00:00Z".parse().unwrap()),
            }),
            ..read_payload()
        };

        assert!(payload.validar().is_err());
    }

    /// Una sola punta vale: "desde el lunes" sin fin es una consulta normal.
    #[test]
    fn historial_acepta_el_rango_con_una_sola_punta() {
        let desde: DateTime<Utc> = "2026-11-01T05:00:00Z".parse().unwrap();
        let payload = ProgramacionesReadPayload {
            rango: Some(Rango {
                desde: Some(desde),
                hasta: None,
            }),
            ..read_payload()
        };

        let filtros = payload.validar().expect("deberia ser valido");

        assert_eq!(filtros.desde, Some(desde));
        assert_eq!(filtros.hasta, None);
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
