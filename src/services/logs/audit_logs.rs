use uuid::Uuid;

use crate::{models::audit_logs::AuditLog, services::error::ServiceError};


pub async fn create_audit_log<'e, E>(
    executor: E,
    entity_type: &str,
    entity_id: Uuid,
    action: &str,
    actor_id: Uuid,
    area: Option<&str>,
    changes: Option<serde_json::Value>,
) -> Result<AuditLog, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let log = sqlx::query_as!(
        AuditLog,
        r#"
        INSERT INTO audit_log (entity_type, entity_id, action, actor_id, area, changes)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, entity_type, entity_id, action, actor_id, area, changes, created_at
        "#,
        entity_type,
        entity_id,
        action,
        actor_id,
        area,
        changes
    )
    .fetch_one(executor)
    .await?;

    Ok(log)
}