//! Shared types and helpers for the emergency route surfaces.
//!
//! These items are split out of the per-surface handler modules (`protocols`,
//! `contacts`, `incidents`, `broadcasts`, `drills`, `statistics`) so each
//! surface can `use super::shared::*` without duplicating the request/query
//! DTOs, their `From` conversions, or the manager-gate helper.
//!
//! # RLS routing (PAP-80)
//!
//! Every handler acquires an [`RlsConnection`](api_core::extractors::RlsConnection),
//! which validates the caller's JWT + org membership and opens a pooled
//! connection with the RLS context (`app.current_org_id` / user GUCs) bound to
//! it. All `emergency_*` tables run under `FORCE ROW LEVEL SECURITY` (migration
//! `00179`), so the connection is the only thing that scopes a query to the
//! caller's tenant — the repository holds no pool of its own.
//!
//! The authoritative organization is therefore `rls.tenant_id()` (the
//! membership-validated org from the session), **not** any client-supplied
//! `organization_id`. The org-keyed repo queries combine that value with the
//! RLS context, so the explicit `organization_id = $N` SQL filter and the
//! policy can never disagree; a foreign-tenant id resolves to `None` → `404`.
//! Membership is enforced by the extractor, so the previous `verify_org_access`
//! helper is no longer needed; manager-gated actions still check `rls.role()`.
//!
//! **IMPORTANT**: every path calls `rls.release().await` before returning so the
//! RLS context is cleared and the connection returns clean to the pool.

use common::TenantRole;
use db::models::{
    CreateEmergencyBroadcast, CreateEmergencyContact, CreateEmergencyDrill,
    CreateEmergencyIncident, CreateEmergencyProtocol, EmergencyBroadcastQuery,
    EmergencyContactQuery, EmergencyDrillQuery, EmergencyIncidentQuery, EmergencyProtocolQuery,
    UpdateEmergencyContact, UpdateEmergencyDrill, UpdateEmergencyIncident, UpdateEmergencyProtocol,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

// ============================================
// Authorization helper (issue #827 / PAP-80)
// ============================================
//
// Org membership is enforced up-front by the `RlsConnection` extractor (it
// rejects non-members before the handler body runs), and RLS scopes every
// query to `rls.tenant_id()`. The only remaining handler-level check is the
// manager gate on mass-notification / incident-lifecycle actions, which mirrors
// the original `verify_org_manager` role set: org_admin / manager plus the
// platform-level admin roles. `TechnicalManager` is intentionally excluded to
// preserve the pre-conversion authorization surface.

/// True if `role` may perform manager-gated emergency actions (broadcast +
/// incident create/acknowledge — issue #827).
pub(super) fn is_emergency_manager(role: TenantRole) -> bool {
    matches!(
        role,
        TenantRole::SuperAdmin
            | TenantRole::PlatformAdmin
            | TenantRole::OrgAdmin
            | TenantRole::Manager
    )
}

// ============================================
// Query Parameter Types
// ============================================

/// Organization query parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Uuid,
}

/// Create protocol request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateProtocolRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateEmergencyProtocol,
}

/// Create contact request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateContactRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateEmergencyContact,
}

/// Create incident request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateIncidentRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateEmergencyIncident,
}

/// Create broadcast request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateBroadcastRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateEmergencyBroadcast,
}

/// Create drill request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateDrillRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateEmergencyDrill,
}

/// Update protocol request wrapper.
#[derive(Debug, Deserialize)]
pub struct UpdateProtocolRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateEmergencyProtocol,
}

/// Update contact request wrapper.
#[derive(Debug, Deserialize)]
pub struct UpdateContactRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateEmergencyContact,
}

/// Update incident request wrapper.
#[derive(Debug, Deserialize)]
pub struct UpdateIncidentRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateEmergencyIncident,
}

/// Update drill request wrapper.
#[derive(Debug, Deserialize)]
pub struct UpdateDrillRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateEmergencyDrill,
}

/// Protocol list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ProtocolListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub protocol_type: Option<String>,
    pub is_active: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&ProtocolListQuery> for EmergencyProtocolQuery {
    fn from(q: &ProtocolListQuery) -> Self {
        EmergencyProtocolQuery {
            building_id: q.building_id,
            protocol_type: q.protocol_type.clone(),
            is_active: q.is_active,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Contact list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ContactListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub contact_type: Option<String>,
    pub is_active: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&ContactListQuery> for EmergencyContactQuery {
    fn from(q: &ContactListQuery) -> Self {
        EmergencyContactQuery {
            building_id: q.building_id,
            contact_type: q.contact_type.clone(),
            is_active: q.is_active,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Incident list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct IncidentListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub incident_type: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub active_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&IncidentListQuery> for EmergencyIncidentQuery {
    fn from(q: &IncidentListQuery) -> Self {
        EmergencyIncidentQuery {
            building_id: q.building_id,
            incident_type: q.incident_type.clone(),
            severity: q.severity.clone(),
            status: q.status.clone(),
            active_only: q.active_only,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Broadcast list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct BroadcastListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub incident_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&BroadcastListQuery> for EmergencyBroadcastQuery {
    fn from(q: &BroadcastListQuery) -> Self {
        EmergencyBroadcastQuery {
            building_id: q.building_id,
            incident_id: q.incident_id,
            is_active: q.is_active,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Drill list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct DrillListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub drill_type: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&DrillListQuery> for EmergencyDrillQuery {
    fn from(q: &DrillListQuery) -> Self {
        EmergencyDrillQuery {
            building_id: q.building_id,
            drill_type: q.drill_type.clone(),
            status: q.status.clone(),
            limit: q.limit,
            offset: q.offset,
        }
    }
}
