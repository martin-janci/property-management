//! Small org-ownership checks for ids that arrive in a request body or path
//! and are written through the plain pool (no RLS context).

use db::DbPool;
use uuid::Uuid;

/// Whether `building_id` belongs to `org_id`.
pub async fn building_in_org(db: &DbPool, building_id: Uuid, org_id: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM buildings WHERE id = $1 AND organization_id = $2)",
    )
    .bind(building_id)
    .bind(org_id)
    .fetch_one(db)
    .await
}

/// Whether `unit_id` belongs to a building of `org_id`.
pub async fn unit_in_org(db: &DbPool, unit_id: Uuid, org_id: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        r#"SELECT EXISTS(
               SELECT 1 FROM units u JOIN buildings b ON b.id = u.building_id
               WHERE u.id = $1 AND b.organization_id = $2)"#,
    )
    .bind(unit_id)
    .bind(org_id)
    .fetch_one(db)
    .await
}
