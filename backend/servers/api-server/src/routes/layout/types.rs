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
