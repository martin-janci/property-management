use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ScreenQuery {
    pub screen: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutDraftRequest {
    pub screen: String,
    /// Must deserialize as layout_core::ScreenConfig.
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutRailsRequest {
    pub screen: String,
    /// Must deserialize as layout_core::Rails.
    pub rails: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutManifestRequest {
    /// "web" | "mobile"
    pub platform: String,
    /// Must deserialize as layout_core::RegistryManifest with matching platform.
    pub manifest: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ValidationErrorsResponse {
    pub errors: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PublishRequest {
    pub screen: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RollbackRequest {
    pub screen: String,
    pub version: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct KillRequest {
    pub screen: String,
    pub section_type: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutTenantOverrideRequest {
    pub screen: String,
    /// Must deserialize as layout_core::TenantOverride and pass rails validation.
    pub override_config: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResolvedQuery {
    /// "web" | "mobile" (default "web")
    pub platform: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PreviewResolveRequest {
    /// Must deserialize as layout_core::ScreenConfig.
    pub config: serde_json::Value,
    /// "web" | "mobile"
    pub platform: String,
}
