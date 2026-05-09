//! Feature guard middleware (Epic 110, Story 110.1).
//!
//! Provides middleware and extractors for protecting routes with feature flags.
//! Routes can be guarded to require specific features to be enabled for the
//! requesting user's context (user, organization, or role overrides).

use axum::{
    body::Body,
    extract::FromRequestParts,
    http::{request::Parts, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// Error response when a feature is disabled.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeatureDisabledError {
    /// Error code
    pub error: String,
    /// Feature key that is disabled
    pub feature_key: String,
    /// Optional message about how to enable the feature
    pub message: String,
    /// Optional upgrade path information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgrade_path: Option<String>,
}

impl FeatureDisabledError {
    /// Create a new feature disabled error.
    pub fn new(feature_key: &str) -> Self {
        Self {
            error: "FEATURE_DISABLED".to_string(),
            feature_key: feature_key.to_string(),
            message: format!(
                "The '{}' feature is not enabled for your account",
                feature_key
            ),
            upgrade_path: None,
        }
    }

    /// Add upgrade path information.
    pub fn with_upgrade_path(mut self, path: &str) -> Self {
        self.upgrade_path = Some(path.to_string());
        self
    }

    /// Add custom message.
    pub fn with_message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }
}

impl IntoResponse for FeatureDisabledError {
    fn into_response(self) -> Response {
        (StatusCode::FORBIDDEN, Json(self)).into_response()
    }
}

/// User context for feature resolution extracted from request.
///
/// This struct contains the IDs needed to resolve feature flags
/// according to the priority order: user > organization > role > global.
#[derive(Debug, Clone, Default)]
pub struct FeatureResolutionContext {
    /// User ID (highest priority for overrides)
    pub user_id: Option<Uuid>,
    /// Organization ID
    pub organization_id: Option<Uuid>,
    /// Role ID
    pub role_id: Option<Uuid>,
}

impl FeatureResolutionContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create context with user ID.
    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Create context with organization ID.
    pub fn with_organization(mut self, org_id: Uuid) -> Self {
        self.organization_id = Some(org_id);
        self
    }

    /// Create context with role ID.
    pub fn with_role(mut self, role_id: Uuid) -> Self {
        self.role_id = Some(role_id);
        self
    }
}

/// Resolved features for the current request context.
///
/// This extractor provides access to the resolved feature flags
/// for the current user's context. It can be used in handlers
/// to conditionally enable functionality.
#[derive(Debug, Clone, Default)]
pub struct FeatureContext {
    /// Map of feature key to enabled state
    pub features: HashMap<String, bool>,
    /// The resolution context used
    pub context: FeatureResolutionContext,
}

impl FeatureContext {
    /// Create a new empty feature context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create context with pre-resolved features.
    pub fn with_features(features: HashMap<String, bool>) -> Self {
        Self {
            features,
            context: FeatureResolutionContext::new(),
        }
    }

    /// Check if a specific feature is enabled.
    ///
    /// Returns `false` if the feature is not found or is disabled.
    pub fn is_enabled(&self, key: &str) -> bool {
        self.features.get(key).copied().unwrap_or(false)
    }

    /// Check if all specified features are enabled.
    pub fn all_enabled(&self, keys: &[&str]) -> bool {
        keys.iter().all(|key| self.is_enabled(key))
    }

    /// Check if any of the specified features are enabled.
    pub fn any_enabled(&self, keys: &[&str]) -> bool {
        keys.iter().any(|key| self.is_enabled(key))
    }

    /// Get a list of all enabled feature keys.
    pub fn enabled_features(&self) -> Vec<&str> {
        self.features
            .iter()
            .filter_map(|(k, v)| if *v { Some(k.as_str()) } else { None })
            .collect()
    }

    /// Get a list of all disabled feature keys.
    pub fn disabled_features(&self) -> Vec<&str> {
        self.features
            .iter()
            .filter_map(|(k, v)| if !*v { Some(k.as_str()) } else { None })
            .collect()
    }
}

impl<S> FromRequestParts<S> for FeatureContext
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get feature context from extensions (set by middleware)
        if let Some(ctx) = parts.extensions.get::<FeatureContext>() {
            return Ok(ctx.clone());
        }

        // If no context is set, return empty context
        // Features will be resolved as disabled by default
        Ok(FeatureContext::new())
    }
}

/// Layer configuration for feature guards.
///
/// This struct holds the configuration for a feature guard layer,
/// including the required feature key and optional upgrade information.
#[derive(Debug, Clone)]
pub struct FeatureGuardConfig {
    /// The feature key that must be enabled
    pub feature_key: &'static str,
    /// Optional upgrade path message
    pub upgrade_path: Option<&'static str>,
    /// Whether to log when access is denied
    pub log_denials: bool,
}

impl FeatureGuardConfig {
    /// Create a new feature guard configuration.
    pub const fn new(feature_key: &'static str) -> Self {
        Self {
            feature_key,
            upgrade_path: None,
            log_denials: true,
        }
    }

    /// Add upgrade path information.
    pub const fn with_upgrade_path(mut self, path: &'static str) -> Self {
        self.upgrade_path = Some(path);
        self
    }

    /// Disable denial logging.
    pub const fn without_logging(mut self) -> Self {
        self.log_denials = false;
        self
    }
}

/// Middleware function to check if a feature is enabled.
///
/// This middleware checks the `FeatureContext` in request extensions
/// and denies access if the required feature is not enabled.
///
/// # Example
///
/// ```ignore
/// use api_core::middleware::feature_guard::{require_feature, FeatureGuardConfig};
/// use axum::middleware;
///
/// let config = FeatureGuardConfig::new("advanced_analytics")
///     .with_upgrade_path("Upgrade to Pro plan for advanced analytics");
///
/// let protected_routes = Router::new()
///     .route("/analytics/advanced", get(advanced_analytics_handler))
///     .layer(middleware::from_fn(move |req, next| {
///         let cfg = config.clone();
///         async move { require_feature(cfg, req, next).await }
///     }));
/// ```
pub async fn require_feature(
    config: FeatureGuardConfig,
    request: Request<Body>,
    next: Next,
) -> Result<Response, FeatureDisabledError> {
    // Get feature context from request extensions
    let feature_context = request
        .extensions()
        .get::<FeatureContext>()
        .cloned()
        .unwrap_or_default();

    if feature_context.is_enabled(config.feature_key) {
        tracing::debug!(
            feature = config.feature_key,
            "Feature guard: access granted"
        );
        Ok(next.run(request).await)
    } else {
        if config.log_denials {
            tracing::warn!(
                feature = config.feature_key,
                user_id = ?feature_context.context.user_id,
                org_id = ?feature_context.context.organization_id,
                "Feature guard: access denied - feature not enabled"
            );
        }

        let mut error = FeatureDisabledError::new(config.feature_key);
        if let Some(upgrade_path) = config.upgrade_path {
            error = error.with_upgrade_path(upgrade_path);
        }

        Err(error)
    }
}

/// Common feature guard configurations for frequently used features.
pub mod feature_guards {
    use super::FeatureGuardConfig;

    /// Guard for AI-powered features.
    pub const AI_FEATURES: FeatureGuardConfig = FeatureGuardConfig::new("ai_suggestions")
        .with_upgrade_path("Upgrade to enable AI-powered suggestions");

    /// Guard for advanced analytics.
    pub const ADVANCED_ANALYTICS: FeatureGuardConfig =
        FeatureGuardConfig::new("advanced_analytics")
            .with_upgrade_path("Upgrade to Pro plan for advanced analytics");

    /// Guard for beta features.
    pub const BETA_FEATURES: FeatureGuardConfig =
        FeatureGuardConfig::new("beta_features").with_upgrade_path("Join our beta program");

    /// Guard for dark mode.
    pub const DARK_MODE: FeatureGuardConfig = FeatureGuardConfig::new("dark_mode");

    /// Guard for new dashboard.
    pub const NEW_DASHBOARD: FeatureGuardConfig = FeatureGuardConfig::new("new_dashboard");
}

/// Helper to create a feature guard middleware closure.
///
/// This is a convenience function for creating feature guard middleware
/// with a simple API.
///
/// # Example
///
/// ```ignore
/// use api_core::middleware::feature_guard::feature_guard;
/// use axum::middleware;
///
/// let protected_routes = Router::new()
///     .route("/ai/suggest", post(ai_suggest_handler))
///     .layer(middleware::from_fn(feature_guard("ai_suggestions")));
/// ```
#[allow(clippy::type_complexity)]
pub fn feature_guard(
    feature_key: &'static str,
) -> impl Fn(
    Request<Body>,
    Next,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, FeatureDisabledError>> + Send>,
> + Clone
       + Send
       + 'static {
    move |request: Request<Body>, next: Next| {
        let config = FeatureGuardConfig::new(feature_key);
        Box::pin(async move { require_feature(config, request, next).await })
    }
}

/// Helper to create a feature guard with upgrade path.
///
/// # Example
///
/// ```ignore
/// use api_core::middleware::feature_guard::feature_guard_with_upgrade;
/// use axum::middleware;
///
/// let protected_routes = Router::new()
///     .route("/ai/suggest", post(ai_suggest_handler))
///     .layer(middleware::from_fn(feature_guard_with_upgrade(
///         "ai_suggestions",
///         "Upgrade to Pro for AI features"
///     )));
/// ```
#[allow(clippy::type_complexity)]
pub fn feature_guard_with_upgrade(
    feature_key: &'static str,
    upgrade_path: &'static str,
) -> impl Fn(
    Request<Body>,
    Next,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, FeatureDisabledError>> + Send>,
> + Clone
       + Send
       + 'static {
    move |request: Request<Body>, next: Next| {
        let config = FeatureGuardConfig::new(feature_key).with_upgrade_path(upgrade_path);
        Box::pin(async move { require_feature(config, request, next).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_context_is_enabled() {
        let mut features = HashMap::new();
        features.insert("enabled_feature".to_string(), true);
        features.insert("disabled_feature".to_string(), false);

        let ctx = FeatureContext::with_features(features);

        assert!(ctx.is_enabled("enabled_feature"));
        assert!(!ctx.is_enabled("disabled_feature"));
        assert!(!ctx.is_enabled("nonexistent_feature"));
    }

    #[test]
    fn test_feature_context_all_enabled() {
        let mut features = HashMap::new();
        features.insert("a".to_string(), true);
        features.insert("b".to_string(), true);
        features.insert("c".to_string(), false);

        let ctx = FeatureContext::with_features(features);

        assert!(ctx.all_enabled(&["a", "b"]));
        assert!(!ctx.all_enabled(&["a", "c"]));
        assert!(!ctx.all_enabled(&["a", "nonexistent"]));
    }

    #[test]
    fn test_feature_context_any_enabled() {
        let mut features = HashMap::new();
        features.insert("a".to_string(), true);
        features.insert("b".to_string(), false);

        let ctx = FeatureContext::with_features(features);

        assert!(ctx.any_enabled(&["a", "b"]));
        assert!(ctx.any_enabled(&["a", "nonexistent"]));
        assert!(!ctx.any_enabled(&["b", "nonexistent"]));
    }

    #[test]
    fn test_feature_disabled_error() {
        let error = FeatureDisabledError::new("test_feature")
            .with_upgrade_path("Upgrade to Pro")
            .with_message("Custom message");

        assert_eq!(error.error, "FEATURE_DISABLED");
        assert_eq!(error.feature_key, "test_feature");
        assert_eq!(error.message, "Custom message");
        assert_eq!(error.upgrade_path, Some("Upgrade to Pro".to_string()));
    }

    #[test]
    fn test_feature_resolution_context_builder() {
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let ctx = FeatureResolutionContext::new()
            .with_user(user_id)
            .with_organization(org_id)
            .with_role(role_id);

        assert_eq!(ctx.user_id, Some(user_id));
        assert_eq!(ctx.organization_id, Some(org_id));
        assert_eq!(ctx.role_id, Some(role_id));
    }

    #[test]
    fn test_feature_guard_config() {
        let config = FeatureGuardConfig::new("test")
            .with_upgrade_path("Upgrade now")
            .without_logging();

        assert_eq!(config.feature_key, "test");
        assert_eq!(config.upgrade_path, Some("Upgrade now"));
        assert!(!config.log_denials);
    }
}
