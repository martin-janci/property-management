//! Emergency repository for Epic 23.
//!
//! Handles all database operations for emergency protocols, contacts, incidents, and broadcasts.
//!
//! # RLS Integration (PAP-80 / PAP-67)
//!
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on every emergency table — `emergency_protocols`,
//! `emergency_contacts`, `emergency_incidents`, `emergency_incident_attachments`,
//! `emergency_incident_updates`, `emergency_broadcasts`,
//! `emergency_broadcast_acknowledgments`, `emergency_drills` (8 tables). Under
//! `FORCE` the api-server's owner connection is no longer exempt, so a query
//! issued on a connection WITHOUT `app.current_org_id` set collapses to deny-all
//! (own-org reads return empty, writes fail the policy `WITH CHECK`).
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
//! pool**, so there is no way to issue a query that bypasses RLS. This mirrors
//! the `work_order.rs` / `budget.rs` / `sensor.rs` precedent.
//!
//! The explicit `organization_id = $N` SQL filters on the parent tables are
//! retained as defense-in-depth; handlers pass `rls.tenant_id()` (the org the
//! caller was validated against), so the SQL filter and the RLS context can
//! never disagree. The child tables (`*_attachments`, `*_updates`,
//! `*_acknowledgments`) are scoped by RLS through an `EXISTS` join on the
//! parent's `organization_id`.

use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::models::{
    AcknowledgeBroadcast, AddIncidentAttachment, CompleteDrill, CreateEmergencyBroadcast,
    CreateEmergencyContact, CreateEmergencyDrill, CreateEmergencyIncident, CreateEmergencyProtocol,
    CreateIncidentUpdate, EmergencyBroadcast, EmergencyBroadcastAcknowledgment,
    EmergencyBroadcastQuery, EmergencyContact, EmergencyContactQuery, EmergencyDrill,
    EmergencyDrillQuery, EmergencyIncident, EmergencyIncidentAttachment, EmergencyIncidentQuery,
    EmergencyIncidentUpdate, EmergencyProtocol, EmergencyProtocolQuery, EmergencyStatistics,
    IncidentSeveritySummary, IncidentTypeSummary, UpdateEmergencyContact, UpdateEmergencyDrill,
    UpdateEmergencyIncident, UpdateEmergencyProtocol,
};
use crate::DbPool;

/// Repository for emergency management operations.
///
/// Stateless: every method receives an RLS-context-bearing executor (the
/// per-request connection from the `RlsConnection` extractor). The repo holds
/// **no pool**, so it cannot issue an un-scoped query that would collapse to
/// deny-all under the `FORCE ROW LEVEL SECURITY` policies (migration `00179`).
#[derive(Clone)]
pub struct EmergencyRepository;

impl EmergencyRepository {
    /// Create a new EmergencyRepository.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it — every query runs on a context-set connection supplied by the
    /// handler's `RlsConnection`.
    pub fn new(_pool: DbPool) -> Self {
        Self
    }

    // ============================================
    // Emergency Protocol Operations
    // ============================================

    /// Create a new emergency protocol.
    pub async fn create_protocol<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        created_by: Uuid,
        data: CreateEmergencyProtocol,
    ) -> Result<EmergencyProtocol, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyProtocol>(
            r#"
            INSERT INTO emergency_protocols (
                organization_id, building_id, name, protocol_type, description,
                steps, contacts, evacuation_info, attachments, is_active, priority, created_by
            ) VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, '[]'), $8, $9, COALESCE($10, TRUE), COALESCE($11, 0), $12)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(data.building_id)
        .bind(&data.name)
        .bind(&data.protocol_type)
        .bind(&data.description)
        .bind(&data.steps)
        .bind(&data.contacts)
        .bind(&data.evacuation_info)
        .bind(&data.attachments)
        .bind(data.is_active)
        .bind(data.priority)
        .bind(created_by)
        .fetch_one(executor)
        .await
    }

    /// Find emergency protocol by ID.
    pub async fn find_protocol_by_id<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        protocol_id: Uuid,
    ) -> Result<Option<EmergencyProtocol>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyProtocol>(
            "SELECT * FROM emergency_protocols WHERE id = $1 AND organization_id = $2",
        )
        .bind(protocol_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
    }

    /// List emergency protocols with filters.
    pub async fn list_protocols<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        query: EmergencyProtocolQuery,
    ) -> Result<Vec<EmergencyProtocol>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, EmergencyProtocol>(
            r#"
            SELECT * FROM emergency_protocols
            WHERE organization_id = $1
              AND ($2::UUID IS NULL OR building_id = $2)
              AND ($3::VARCHAR IS NULL OR protocol_type = $3)
              AND ($4::BOOLEAN IS NULL OR is_active = $4)
            ORDER BY priority DESC, name ASC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(organization_id)
        .bind(query.building_id)
        .bind(&query.protocol_type)
        .bind(query.is_active)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update an emergency protocol.
    pub async fn update_protocol<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        protocol_id: Uuid,
        data: UpdateEmergencyProtocol,
    ) -> Result<Option<EmergencyProtocol>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyProtocol>(
            r#"
            UPDATE emergency_protocols SET
                building_id = COALESCE($3, building_id),
                name = COALESCE($4, name),
                protocol_type = COALESCE($5, protocol_type),
                description = COALESCE($6, description),
                steps = COALESCE($7, steps),
                contacts = COALESCE($8, contacts),
                evacuation_info = COALESCE($9, evacuation_info),
                attachments = COALESCE($10, attachments),
                is_active = COALESCE($11, is_active),
                priority = COALESCE($12, priority),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(protocol_id)
        .bind(organization_id)
        .bind(data.building_id)
        .bind(&data.name)
        .bind(&data.protocol_type)
        .bind(&data.description)
        .bind(&data.steps)
        .bind(&data.contacts)
        .bind(&data.evacuation_info)
        .bind(&data.attachments)
        .bind(data.is_active)
        .bind(data.priority)
        .fetch_optional(executor)
        .await
    }

    /// Delete an emergency protocol.
    pub async fn delete_protocol<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        protocol_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("DELETE FROM emergency_protocols WHERE id = $1 AND organization_id = $2")
                .bind(protocol_id)
                .bind(organization_id)
                .execute(executor)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    // ============================================
    // Emergency Contact Operations
    // ============================================

    /// Create a new emergency contact.
    pub async fn create_contact<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        data: CreateEmergencyContact,
    ) -> Result<EmergencyContact, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyContact>(
            r#"
            INSERT INTO emergency_contacts (
                organization_id, building_id, name, role, phone, phone_secondary,
                email, address, notes, priority_order, contact_type, available_hours
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, COALESCE($10, 0), $11, $12)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(data.building_id)
        .bind(&data.name)
        .bind(&data.role)
        .bind(&data.phone)
        .bind(&data.phone_secondary)
        .bind(&data.email)
        .bind(&data.address)
        .bind(&data.notes)
        .bind(data.priority_order)
        .bind(&data.contact_type)
        .bind(&data.available_hours)
        .fetch_one(executor)
        .await
    }

    /// Find emergency contact by ID.
    pub async fn find_contact_by_id<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        contact_id: Uuid,
    ) -> Result<Option<EmergencyContact>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyContact>(
            "SELECT * FROM emergency_contacts WHERE id = $1 AND organization_id = $2",
        )
        .bind(contact_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
    }

    /// List emergency contacts with filters.
    pub async fn list_contacts<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        query: EmergencyContactQuery,
    ) -> Result<Vec<EmergencyContact>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, EmergencyContact>(
            r#"
            SELECT * FROM emergency_contacts
            WHERE organization_id = $1
              AND ($2::UUID IS NULL OR building_id = $2)
              AND ($3::VARCHAR IS NULL OR contact_type = $3)
              AND ($4::BOOLEAN IS NULL OR is_active = $4)
            ORDER BY priority_order ASC, name ASC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(organization_id)
        .bind(query.building_id)
        .bind(&query.contact_type)
        .bind(query.is_active)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update an emergency contact.
    pub async fn update_contact<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        contact_id: Uuid,
        data: UpdateEmergencyContact,
    ) -> Result<Option<EmergencyContact>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyContact>(
            r#"
            UPDATE emergency_contacts SET
                building_id = COALESCE($3, building_id),
                name = COALESCE($4, name),
                role = COALESCE($5, role),
                phone = COALESCE($6, phone),
                phone_secondary = COALESCE($7, phone_secondary),
                email = COALESCE($8, email),
                address = COALESCE($9, address),
                notes = COALESCE($10, notes),
                priority_order = COALESCE($11, priority_order),
                contact_type = COALESCE($12, contact_type),
                is_active = COALESCE($13, is_active),
                available_hours = COALESCE($14, available_hours),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(contact_id)
        .bind(organization_id)
        .bind(data.building_id)
        .bind(&data.name)
        .bind(&data.role)
        .bind(&data.phone)
        .bind(&data.phone_secondary)
        .bind(&data.email)
        .bind(&data.address)
        .bind(&data.notes)
        .bind(data.priority_order)
        .bind(&data.contact_type)
        .bind(data.is_active)
        .bind(&data.available_hours)
        .fetch_optional(executor)
        .await
    }

    /// Delete an emergency contact.
    pub async fn delete_contact<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        contact_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("DELETE FROM emergency_contacts WHERE id = $1 AND organization_id = $2")
                .bind(contact_id)
                .bind(organization_id)
                .execute(executor)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    // ============================================
    // Emergency Incident Operations
    // ============================================

    /// Create a new emergency incident.
    pub async fn create_incident<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        reported_by: Uuid,
        data: CreateEmergencyIncident,
    ) -> Result<EmergencyIncident, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyIncident>(
            r#"
            INSERT INTO emergency_incidents (
                organization_id, building_id, unit_id, reported_by, incident_type,
                severity, title, description, location_details, latitude, longitude,
                protocol_id, metadata
            ) VALUES ($1, $2, $3, $4, $5, COALESCE($6, 'medium'), $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(data.building_id)
        .bind(data.unit_id)
        .bind(reported_by)
        .bind(&data.incident_type)
        .bind(&data.severity)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.location_details)
        .bind(data.latitude)
        .bind(data.longitude)
        .bind(data.protocol_id)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Find emergency incident by ID.
    pub async fn find_incident_by_id<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        incident_id: Uuid,
    ) -> Result<Option<EmergencyIncident>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyIncident>(
            "SELECT * FROM emergency_incidents WHERE id = $1 AND organization_id = $2",
        )
        .bind(incident_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
    }

    /// List emergency incidents with filters.
    pub async fn list_incidents<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        query: EmergencyIncidentQuery,
    ) -> Result<Vec<EmergencyIncident>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, EmergencyIncident>(
            r#"
            SELECT * FROM emergency_incidents
            WHERE organization_id = $1
              AND ($2::UUID IS NULL OR building_id = $2)
              AND ($3::VARCHAR IS NULL OR incident_type = $3)
              AND ($4::VARCHAR IS NULL OR severity = $4)
              AND ($5::VARCHAR IS NULL OR status = $5)
              AND ($6::BOOLEAN IS NULL OR ($6 = TRUE AND status NOT IN ('resolved', 'closed')))
            ORDER BY reported_at DESC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(organization_id)
        .bind(query.building_id)
        .bind(&query.incident_type)
        .bind(&query.severity)
        .bind(&query.status)
        .bind(query.active_only)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update an emergency incident.
    pub async fn update_incident<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        incident_id: Uuid,
        data: UpdateEmergencyIncident,
    ) -> Result<Option<EmergencyIncident>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyIncident>(
            r#"
            UPDATE emergency_incidents SET
                incident_type = COALESCE($3, incident_type),
                severity = COALESCE($4, severity),
                title = COALESCE($5, title),
                description = COALESCE($6, description),
                location_details = COALESCE($7, location_details),
                status = COALESCE($8, status),
                resolution = COALESCE($9, resolution),
                protocol_id = COALESCE($10, protocol_id),
                fault_id = COALESCE($11, fault_id),
                metadata = COALESCE($12, metadata),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(incident_id)
        .bind(organization_id)
        .bind(&data.incident_type)
        .bind(&data.severity)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.location_details)
        .bind(&data.status)
        .bind(&data.resolution)
        .bind(data.protocol_id)
        .bind(data.fault_id)
        .bind(&data.metadata)
        .fetch_optional(executor)
        .await
    }

    /// Acknowledge an incident.
    pub async fn acknowledge_incident<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        incident_id: Uuid,
    ) -> Result<Option<EmergencyIncident>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyIncident>(
            r#"
            UPDATE emergency_incidents SET
                status = 'acknowledged',
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2 AND status = 'reported'
            RETURNING *
            "#,
        )
        .bind(incident_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
    }

    /// Resolve an incident.
    /// Only incidents that are not already resolved or closed can be resolved.
    pub async fn resolve_incident<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        incident_id: Uuid,
        resolved_by: Uuid,
        resolution: &str,
    ) -> Result<Option<EmergencyIncident>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyIncident>(
            r#"
            UPDATE emergency_incidents SET
                status = 'resolved',
                resolution = $3,
                resolved_by = $4,
                resolved_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
              AND status NOT IN ('resolved', 'closed')
            RETURNING *
            "#,
        )
        .bind(incident_id)
        .bind(organization_id)
        .bind(resolution)
        .bind(resolved_by)
        .fetch_optional(executor)
        .await
    }

    /// Close an incident.
    pub async fn close_incident<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        incident_id: Uuid,
    ) -> Result<Option<EmergencyIncident>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyIncident>(
            r#"
            UPDATE emergency_incidents SET
                status = 'closed',
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2 AND status = 'resolved'
            RETURNING *
            "#,
        )
        .bind(incident_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
    }

    /// Add attachment to incident.
    pub async fn add_incident_attachment<'e, E>(
        &self,
        executor: E,
        incident_id: Uuid,
        uploaded_by: Uuid,
        data: AddIncidentAttachment,
    ) -> Result<EmergencyIncidentAttachment, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyIncidentAttachment>(
            r#"
            INSERT INTO emergency_incident_attachments (incident_id, document_id, attachment_type, description, uploaded_by)
            VALUES ($1, $2, COALESCE($3, 'photo'), $4, $5)
            RETURNING *
            "#,
        )
        .bind(incident_id)
        .bind(data.document_id)
        .bind(&data.attachment_type)
        .bind(&data.description)
        .bind(uploaded_by)
        .fetch_one(executor)
        .await
    }

    /// List incident attachments.
    pub async fn list_incident_attachments<'e, E>(
        &self,
        executor: E,
        incident_id: Uuid,
    ) -> Result<Vec<EmergencyIncidentAttachment>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyIncidentAttachment>(
            "SELECT * FROM emergency_incident_attachments WHERE incident_id = $1 ORDER BY created_at DESC",
        )
        .bind(incident_id)
        .fetch_all(executor)
        .await
    }

    /// Add update to incident timeline.
    pub async fn add_incident_update<'e, E>(
        &self,
        executor: E,
        incident_id: Uuid,
        updated_by: Uuid,
        data: CreateIncidentUpdate,
    ) -> Result<EmergencyIncidentUpdate, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyIncidentUpdate>(
            r#"
            INSERT INTO emergency_incident_updates (incident_id, update_type, previous_status, new_status, message, updated_by)
            VALUES ($1, COALESCE($2, 'note'), $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(incident_id)
        .bind(&data.update_type)
        .bind(&data.previous_status)
        .bind(&data.new_status)
        .bind(&data.message)
        .bind(updated_by)
        .fetch_one(executor)
        .await
    }

    /// List incident updates.
    pub async fn list_incident_updates<'e, E>(
        &self,
        executor: E,
        incident_id: Uuid,
    ) -> Result<Vec<EmergencyIncidentUpdate>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyIncidentUpdate>(
            "SELECT * FROM emergency_incident_updates WHERE incident_id = $1 ORDER BY created_at DESC",
        )
        .bind(incident_id)
        .fetch_all(executor)
        .await
    }

    // ============================================
    // Emergency Broadcast Operations
    // ============================================

    /// Create a new emergency broadcast.
    pub async fn create_broadcast<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        sent_by: Uuid,
        data: CreateEmergencyBroadcast,
    ) -> Result<EmergencyBroadcast, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyBroadcast>(
            r#"
            INSERT INTO emergency_broadcasts (
                organization_id, building_id, incident_id, title, message,
                severity, channels, sent_by, expires_at, metadata
            ) VALUES ($1, $2, $3, $4, $5, COALESCE($6, 'high'), COALESCE($7, '["push", "email"]'), $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(data.building_id)
        .bind(data.incident_id)
        .bind(&data.title)
        .bind(&data.message)
        .bind(&data.severity)
        .bind(&data.channels)
        .bind(sent_by)
        .bind(data.expires_at)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Find emergency broadcast by ID.
    pub async fn find_broadcast_by_id<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        broadcast_id: Uuid,
    ) -> Result<Option<EmergencyBroadcast>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyBroadcast>(
            "SELECT * FROM emergency_broadcasts WHERE id = $1 AND organization_id = $2",
        )
        .bind(broadcast_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
    }

    /// List emergency broadcasts with filters.
    pub async fn list_broadcasts<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        query: EmergencyBroadcastQuery,
    ) -> Result<Vec<EmergencyBroadcast>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, EmergencyBroadcast>(
            r#"
            SELECT * FROM emergency_broadcasts
            WHERE organization_id = $1
              AND ($2::UUID IS NULL OR building_id = $2)
              AND ($3::UUID IS NULL OR incident_id = $3)
              AND ($4::BOOLEAN IS NULL OR is_active = $4)
            ORDER BY sent_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(organization_id)
        .bind(query.building_id)
        .bind(query.incident_id)
        .bind(query.is_active)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update broadcast delivery counts.
    pub async fn update_broadcast_counts<'e, E>(
        &self,
        executor: E,
        broadcast_id: Uuid,
        recipient_count: i32,
        delivered_count: i32,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            UPDATE emergency_broadcasts SET
                recipient_count = $2,
                delivered_count = $3
            WHERE id = $1
            "#,
        )
        .bind(broadcast_id)
        .bind(recipient_count)
        .bind(delivered_count)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Deactivate a broadcast.
    pub async fn deactivate_broadcast<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        broadcast_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            "UPDATE emergency_broadcasts SET is_active = FALSE WHERE id = $1 AND organization_id = $2",
        )
        .bind(broadcast_id)
        .bind(organization_id)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Acknowledge a broadcast.
    pub async fn acknowledge_broadcast<'e, E>(
        &self,
        executor: E,
        broadcast_id: Uuid,
        user_id: Uuid,
        data: AcknowledgeBroadcast,
    ) -> Result<EmergencyBroadcastAcknowledgment, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyBroadcastAcknowledgment>(
            r#"
            INSERT INTO emergency_broadcast_acknowledgments (broadcast_id, user_id, status, message)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (broadcast_id, user_id)
            DO UPDATE SET status = $3, message = $4, acknowledged_at = NOW()
            RETURNING *
            "#,
        )
        .bind(broadcast_id)
        .bind(user_id)
        .bind(&data.status)
        .bind(&data.message)
        .fetch_one(executor)
        .await
    }

    /// List broadcast acknowledgments.
    pub async fn list_broadcast_acknowledgments<'e, E>(
        &self,
        executor: E,
        broadcast_id: Uuid,
    ) -> Result<Vec<EmergencyBroadcastAcknowledgment>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyBroadcastAcknowledgment>(
            "SELECT * FROM emergency_broadcast_acknowledgments WHERE broadcast_id = $1 ORDER BY acknowledged_at DESC",
        )
        .bind(broadcast_id)
        .fetch_all(executor)
        .await
    }

    /// Get acknowledgment count by status.
    pub async fn get_acknowledgment_counts<'e, E>(
        &self,
        executor: E,
        broadcast_id: Uuid,
    ) -> Result<(i64, i64, i64, i64), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query_as::<_, (i64, i64, i64, i64)>(
            r#"
            SELECT
                COUNT(*) AS total,
                COUNT(*) FILTER (WHERE status = 'safe') AS safe_count,
                COUNT(*) FILTER (WHERE status = 'need_help') AS need_help_count,
                COUNT(*) FILTER (WHERE status = 'evacuated') AS evacuated_count
            FROM emergency_broadcast_acknowledgments
            WHERE broadcast_id = $1
            "#,
        )
        .bind(broadcast_id)
        .fetch_one(executor)
        .await?;

        Ok(result)
    }

    // ============================================
    // Emergency Drill Operations
    // ============================================

    /// Create a new emergency drill.
    pub async fn create_drill<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        created_by: Uuid,
        data: CreateEmergencyDrill,
    ) -> Result<EmergencyDrill, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyDrill>(
            r#"
            INSERT INTO emergency_drills (
                organization_id, building_id, protocol_id, drill_type, title,
                description, scheduled_at, participants_expected, created_by
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(data.building_id)
        .bind(data.protocol_id)
        .bind(&data.drill_type)
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.scheduled_at)
        .bind(data.participants_expected)
        .bind(created_by)
        .fetch_one(executor)
        .await
    }

    /// Find emergency drill by ID.
    pub async fn find_drill_by_id<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        drill_id: Uuid,
    ) -> Result<Option<EmergencyDrill>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyDrill>(
            "SELECT * FROM emergency_drills WHERE id = $1 AND organization_id = $2",
        )
        .bind(drill_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
    }

    /// List emergency drills with filters.
    pub async fn list_drills<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        query: EmergencyDrillQuery,
    ) -> Result<Vec<EmergencyDrill>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, EmergencyDrill>(
            r#"
            SELECT * FROM emergency_drills
            WHERE organization_id = $1
              AND ($2::UUID IS NULL OR building_id = $2)
              AND ($3::VARCHAR IS NULL OR drill_type = $3)
              AND ($4::VARCHAR IS NULL OR status = $4)
            ORDER BY scheduled_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(organization_id)
        .bind(query.building_id)
        .bind(&query.drill_type)
        .bind(&query.status)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update an emergency drill.
    pub async fn update_drill<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        drill_id: Uuid,
        data: UpdateEmergencyDrill,
    ) -> Result<Option<EmergencyDrill>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyDrill>(
            r#"
            UPDATE emergency_drills SET
                protocol_id = COALESCE($3, protocol_id),
                drill_type = COALESCE($4, drill_type),
                title = COALESCE($5, title),
                description = COALESCE($6, description),
                scheduled_at = COALESCE($7, scheduled_at),
                status = COALESCE($8, status),
                participants_expected = COALESCE($9, participants_expected),
                participants_actual = COALESCE($10, participants_actual),
                duration_minutes = COALESCE($11, duration_minutes),
                notes = COALESCE($12, notes),
                issues_found = COALESCE($13, issues_found),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(drill_id)
        .bind(organization_id)
        .bind(data.protocol_id)
        .bind(&data.drill_type)
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.scheduled_at)
        .bind(&data.status)
        .bind(data.participants_expected)
        .bind(data.participants_actual)
        .bind(data.duration_minutes)
        .bind(&data.notes)
        .bind(&data.issues_found)
        .fetch_optional(executor)
        .await
    }

    /// Start a drill (change status to in_progress).
    pub async fn start_drill<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        drill_id: Uuid,
    ) -> Result<Option<EmergencyDrill>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyDrill>(
            r#"
            UPDATE emergency_drills SET
                status = 'in_progress',
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2 AND status = 'scheduled'
            RETURNING *
            "#,
        )
        .bind(drill_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
    }

    /// Complete a drill.
    pub async fn complete_drill<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        drill_id: Uuid,
        data: CompleteDrill,
    ) -> Result<Option<EmergencyDrill>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyDrill>(
            r#"
            UPDATE emergency_drills SET
                status = 'completed',
                completed_at = NOW(),
                participants_actual = $3,
                duration_minutes = $4,
                notes = COALESCE($5, notes),
                issues_found = COALESCE($6, issues_found),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2 AND status = 'in_progress'
            RETURNING *
            "#,
        )
        .bind(drill_id)
        .bind(organization_id)
        .bind(data.participants_actual)
        .bind(data.duration_minutes)
        .bind(&data.notes)
        .bind(&data.issues_found)
        .fetch_optional(executor)
        .await
    }

    /// Cancel a drill.
    pub async fn cancel_drill<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        drill_id: Uuid,
    ) -> Result<Option<EmergencyDrill>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyDrill>(
            r#"
            UPDATE emergency_drills SET
                status = 'cancelled',
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2 AND status IN ('scheduled', 'in_progress')
            RETURNING *
            "#,
        )
        .bind(drill_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
    }

    /// Delete an emergency drill.
    pub async fn delete_drill<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        drill_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            "DELETE FROM emergency_drills WHERE id = $1 AND organization_id = $2 AND status = 'scheduled'",
        )
        .bind(drill_id)
        .bind(organization_id)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ============================================
    // Statistics and Reporting
    // ============================================

    /// Get emergency statistics for organization.
    pub async fn get_statistics<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<EmergencyStatistics, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyStatistics>(
            r#"
            SELECT
                (SELECT COUNT(*) FROM emergency_protocols WHERE organization_id = $1) AS total_protocols,
                (SELECT COUNT(*) FROM emergency_protocols WHERE organization_id = $1 AND is_active = TRUE) AS active_protocols,
                (SELECT COUNT(*) FROM emergency_contacts WHERE organization_id = $1 AND is_active = TRUE) AS total_contacts,
                (SELECT COUNT(*) FROM emergency_incidents WHERE organization_id = $1) AS total_incidents,
                (SELECT COUNT(*) FROM emergency_incidents WHERE organization_id = $1 AND status NOT IN ('resolved', 'closed')) AS active_incidents,
                (SELECT COUNT(*) FROM emergency_incidents WHERE organization_id = $1 AND status = 'resolved') AS resolved_incidents,
                (SELECT COUNT(*) FROM emergency_broadcasts WHERE organization_id = $1) AS total_broadcasts,
                (SELECT COUNT(*) FROM emergency_broadcasts WHERE organization_id = $1 AND is_active = TRUE) AS active_broadcasts,
                (SELECT COUNT(*) FROM emergency_drills WHERE organization_id = $1) AS total_drills,
                (SELECT COUNT(*) FROM emergency_drills WHERE organization_id = $1 AND status = 'completed') AS completed_drills
            "#,
        )
        .bind(organization_id)
        .fetch_one(executor)
        .await
    }

    /// Get incident summary by type.
    pub async fn get_incident_summary_by_type<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<Vec<IncidentTypeSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, IncidentTypeSummary>(
            r#"
            SELECT
                incident_type,
                COUNT(*) AS count,
                COUNT(*) FILTER (WHERE status NOT IN ('resolved', 'closed')) AS active_count
            FROM emergency_incidents
            WHERE organization_id = $1
            GROUP BY incident_type
            ORDER BY count DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(executor)
        .await
    }

    /// Get incident summary by severity.
    pub async fn get_incident_summary_by_severity<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<Vec<IncidentSeveritySummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, IncidentSeveritySummary>(
            r#"
            SELECT
                severity,
                COUNT(*) AS count
            FROM emergency_incidents
            WHERE organization_id = $1
            GROUP BY severity
            ORDER BY
                CASE severity
                    WHEN 'critical' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'medium' THEN 3
                    WHEN 'low' THEN 4
                    ELSE 5
                END
            "#,
        )
        .bind(organization_id)
        .fetch_all(executor)
        .await
    }

    /// Get active incidents (for dashboard).
    pub async fn get_active_incidents<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<Vec<EmergencyIncident>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyIncident>(
            r#"
            SELECT * FROM emergency_incidents
            WHERE organization_id = $1
              AND status NOT IN ('resolved', 'closed')
            ORDER BY
                CASE severity
                    WHEN 'critical' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'medium' THEN 3
                    WHEN 'low' THEN 4
                    ELSE 5
                END,
                reported_at DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(executor)
        .await
    }

    /// Get upcoming drills.
    pub async fn get_upcoming_drills<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        days_ahead: i32,
    ) -> Result<Vec<EmergencyDrill>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, EmergencyDrill>(
            r#"
            SELECT * FROM emergency_drills
            WHERE organization_id = $1
              AND status = 'scheduled'
              AND scheduled_at >= NOW()
              AND scheduled_at <= NOW() + make_interval(days => $2)
            ORDER BY scheduled_at ASC
            "#,
        )
        .bind(organization_id)
        .bind(days_ahead)
        .fetch_all(executor)
        .await
    }
}
