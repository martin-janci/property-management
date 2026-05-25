// Epic 8A-3: Mobile OS push notification registration
// DB model and request/response types for device push tokens (FCM/APNs).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Push platform discriminant stored in the `platform` column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PushPlatform {
    /// Firebase Cloud Messaging — Android devices.
    Fcm,
    /// Apple Push Notification service — iOS devices.
    Apns,
}

impl PushPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            PushPlatform::Fcm => "fcm",
            PushPlatform::Apns => "apns",
        }
    }
}

impl TryFrom<&str> for PushPlatform {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "fcm" => Ok(PushPlatform::Fcm),
            "apns" => Ok(PushPlatform::Apns),
            other => Err(format!("unknown push platform: {other}")),
        }
    }
}

/// Raw DB row returned by `SELECT ... FROM device_push_tokens`.
#[derive(Debug, Clone, FromRow)]
pub struct DevicePushToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub platform: String,
    pub app_id: Option<String>,
    pub device_name: Option<String>,
    pub last_seen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl DevicePushToken {
    /// Parse the stored platform string into the typed enum.
    pub fn push_platform(&self) -> PushPlatform {
        PushPlatform::try_from(self.platform.as_str()).unwrap_or(PushPlatform::Fcm)
    }
}

/// Request body for `POST /api/v1/users/me/push-tokens`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterPushTokenRequest {
    /// The native FCM registration token or APNs device token.
    pub token: String,
    /// Push platform (`fcm` or `apns`).
    pub platform: PushPlatform,
    /// Optional bundle / package ID (e.g. `three.two.bit.ppt.management`).
    pub app_id: Option<String>,
    /// Human-readable device name for the token inventory in the admin UI.
    pub device_name: Option<String>,
}

/// Response body returned after successful token registration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PushTokenResponse {
    pub id: Uuid,
    pub platform: PushPlatform,
    pub app_id: Option<String>,
    pub device_name: Option<String>,
    pub last_seen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl From<DevicePushToken> for PushTokenResponse {
    fn from(row: DevicePushToken) -> Self {
        Self {
            id: row.id,
            platform: row.push_platform(),
            app_id: row.app_id,
            device_name: row.device_name,
            last_seen_at: row.last_seen_at,
            created_at: row.created_at,
        }
    }
}
