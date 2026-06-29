//! Organization dashboard aggregate.

use super::SensorRepository;
use crate::models::{SensorAlert, SensorTypeCount};
use sqlx::PgConnection;
use uuid::Uuid;

impl SensorRepository {
    /// Get dashboard data for an organization.
    ///
    /// Multi-statement (six aggregate reads), so it takes a connection and
    /// reborrows it for each query.
    pub async fn get_dashboard(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        building_id: Option<Uuid>,
    ) -> Result<crate::models::SensorDashboard, sqlx::Error> {
        // Get sensor counts
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sensors WHERE organization_id = $1 AND ($2::uuid IS NULL OR building_id = $2)",
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&mut *conn)
        .await?;

        let (active,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sensors WHERE organization_id = $1 AND status = 'active' AND ($2::uuid IS NULL OR building_id = $2)",
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&mut *conn)
        .await?;

        let (offline,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sensors WHERE organization_id = $1 AND status = 'offline' AND ($2::uuid IS NULL OR building_id = $2)",
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&mut *conn)
        .await?;

        // Count unresolved alerts
        let (unresolved_alerts,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM sensor_alerts a
            JOIN sensors s ON s.id = a.sensor_id
            WHERE s.organization_id = $1 AND a.resolved_at IS NULL
                AND ($2::uuid IS NULL OR s.building_id = $2)
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&mut *conn)
        .await?;

        // Get sensors by type
        let sensors_by_type: Vec<SensorTypeCount> = sqlx::query_as(
            r#"
            SELECT sensor_type, COUNT(*) as count
            FROM sensors
            WHERE organization_id = $1 AND ($2::uuid IS NULL OR building_id = $2)
            GROUP BY sensor_type
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(&mut *conn)
        .await?;

        // Get recent alerts
        let recent_alerts: Vec<SensorAlert> = sqlx::query_as(
            r#"
            SELECT a.* FROM sensor_alerts a
            JOIN sensors s ON s.id = a.sensor_id
            WHERE s.organization_id = $1 AND ($2::uuid IS NULL OR s.building_id = $2)
            ORDER BY a.triggered_at DESC
            LIMIT 10
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(&mut *conn)
        .await?;

        Ok(crate::models::SensorDashboard {
            total_sensors: total,
            active_sensors: active,
            offline_sensors: offline,
            unresolved_alerts,
            sensors_by_type,
            recent_alerts,
        })
    }
}
