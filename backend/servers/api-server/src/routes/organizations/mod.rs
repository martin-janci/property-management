//! Organization routes (UC-27, Epic 2A) — split by surface.
//!
//! | Surface  | Module     | Handlers |
//! |----------|------------|----------|
//! | core     | `core`     | CRUD (create, list, get, update, delete) |
//! | members  | `members`  | Member management (list, add, update, remove) |
//! | roles    | `roles`    | Role management (list, create, get, update, delete) |
//! | settings | `settings` | Settings, branding, data export, feature preferences |

pub mod core;
pub mod members;
pub mod roles;
pub mod settings;

// Re-export all public types so `main.rs` (OpenApi derive) can reference them
// via `routes::organizations::*` without change.
pub use core::{
    // handler fns (needed for utoipa __path_* re-export below)
    __path_create_organization,
    __path_delete_organization,
    __path_get_organization,
    __path_list_my_organizations,
    __path_list_organizations,
    __path_update_organization,
    // types
    CreateOrganizationRequest,
    DeleteOrganizationResponse,
    ListOrganizationsResponse,
    OrganizationResponse,
    UpdateOrganizationRequest,
};
pub use members::{
    __path_add_organization_member, __path_list_organization_members,
    __path_remove_organization_member, __path_update_organization_member, AddMemberRequest,
    AddMemberResponse, ListMembersResponse, MemberResponse, RemoveMemberResponse,
    UpdateMemberRequest, UpdateMemberResponse,
};
pub use roles::{
    __path_create_organization_role, __path_delete_organization_role, __path_get_organization_role,
    __path_list_organization_roles, __path_update_organization_role, CreateRoleRequest,
    CreateRoleResponse, DeleteRoleResponse, GetRoleResponse, ListRolesResponse, RoleResponse,
    UpdateRoleRequest, UpdateRoleResponse,
};
pub use settings::{
    __path_export_organization_data, __path_get_organization_branding,
    __path_get_organization_settings, __path_update_organization_branding,
    __path_update_organization_settings, ExportMember, ExportQuery, ExportRole,
    OrganizationBrandingResponse, OrganizationExportResponse, OrganizationSettingsResponse,
    UpdateOrganizationBrandingRequest, UpdateOrganizationSettingsRequest,
};

use axum::Router;

use crate::state::AppState;

/// Assemble the organizations router from all surface sub-modules.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(core::router())
        .merge(members::router())
        .merge(roles::router())
        .merge(settings::router())
}
