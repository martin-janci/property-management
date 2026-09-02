//! Notification types and event definitions (Story 84.4: Notification Trigger System).
//!
//! This module provides the foundation for the event-driven notification system,
//! supporting multi-channel delivery (push, email, in-app) with user preferences.
//!
//! # Epic 2B — Notification Delivery Pipeline
//! The `pipeline` sub-module adds:
//! - `TransportAdapter` traits for each channel (email / push / in-app)
//! - `DeliveryRecord` + `DeliveryStatus` for delivery tracking
//! - `RoutingDecision` for preference-based channel filtering
//! - `PipelineResult` for per-dispatch summaries

pub mod events;
pub mod pipeline;

pub use events::*;
pub use pipeline::{
    DeliveryRecord, DeliveryRequest, DeliveryStatus, EmailTransport, InAppTransport,
    PipelineResult, PushTransport, RoutingDecision, TransportResult,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Notification Channel
// ============================================================================

/// Available notification delivery channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    /// Push notification (FCM/APNs).
    Push,
    /// Email notification.
    Email,
    /// In-app notification (stored in database, delivered via WebSocket).
    InApp,
}

impl NotificationChannel {
    /// Get all available channels.
    pub const fn all() -> &'static [NotificationChannel] {
        &[
            NotificationChannel::Push,
            NotificationChannel::Email,
            NotificationChannel::InApp,
        ]
    }

    /// Get the string representation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            NotificationChannel::Push => "push",
            NotificationChannel::Email => "email",
            NotificationChannel::InApp => "in_app",
        }
    }
}

impl std::fmt::Display for NotificationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Notification Category
// ============================================================================

/// Categories of notifications for preference management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    /// Announcements from management.
    Announcements,
    /// Fault/issue status updates.
    Faults,
    /// Voting and polls.
    Votes,
    /// Direct messages.
    Messages,
    /// Community events and updates.
    Community,
    /// Financial notifications (invoices, payments).
    Financial,
    /// Document sharing notifications (Story 7A.5).
    Documents,
    /// System maintenance and updates.
    System,
}

impl NotificationCategory {
    /// Get all categories.
    pub const fn all() -> &'static [NotificationCategory] {
        &[
            NotificationCategory::Announcements,
            NotificationCategory::Faults,
            NotificationCategory::Votes,
            NotificationCategory::Messages,
            NotificationCategory::Community,
            NotificationCategory::Financial,
            NotificationCategory::Documents,
            NotificationCategory::System,
        ]
    }

    /// Get the string representation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            NotificationCategory::Announcements => "announcements",
            NotificationCategory::Faults => "faults",
            NotificationCategory::Votes => "votes",
            NotificationCategory::Messages => "messages",
            NotificationCategory::Community => "community",
            NotificationCategory::Financial => "financial",
            NotificationCategory::Documents => "documents",
            NotificationCategory::System => "system",
        }
    }

    /// Get default enabled channels for this category.
    pub const fn default_channels(&self) -> &'static [NotificationChannel] {
        match self {
            // Critical categories default to all channels
            NotificationCategory::Announcements => NotificationChannel::all(),
            NotificationCategory::Faults => NotificationChannel::all(),
            NotificationCategory::Votes => NotificationChannel::all(),
            // Messages default to push and in-app only
            NotificationCategory::Messages => {
                &[NotificationChannel::Push, NotificationChannel::InApp]
            }
            // Community defaults to in-app only
            NotificationCategory::Community => &[NotificationChannel::InApp],
            // Financial defaults to all
            NotificationCategory::Financial => NotificationChannel::all(),
            // Documents (share notifications) default to push and in-app
            NotificationCategory::Documents => {
                &[NotificationChannel::Push, NotificationChannel::InApp]
            }
            // System defaults to email and in-app
            NotificationCategory::System => {
                &[NotificationChannel::Email, NotificationChannel::InApp]
            }
        }
    }
}

impl std::fmt::Display for NotificationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Notification Priority
// ============================================================================

/// Priority level for notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPriority {
    /// Low priority - can be batched.
    Low,
    /// Normal priority - standard delivery.
    #[default]
    Normal,
    /// High priority - immediate delivery.
    High,
    /// Urgent - bypass quiet hours, immediate delivery.
    Urgent,
}

impl NotificationPriority {
    /// Get the string representation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            NotificationPriority::Low => "low",
            NotificationPriority::Normal => "normal",
            NotificationPriority::High => "high",
            NotificationPriority::Urgent => "urgent",
        }
    }
}

// ============================================================================
// Notification Payload
// ============================================================================

/// A notification ready for delivery.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Notification {
    /// Unique notification ID.
    pub id: Uuid,

    /// Target user ID.
    pub user_id: Uuid,

    /// Notification category.
    pub category: NotificationCategory,

    /// Short title.
    pub title: String,

    /// Notification body text.
    pub body: String,

    /// Priority level.
    pub priority: NotificationPriority,

    /// Optional deep link URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_url: Option<String>,

    /// Additional data payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,

    /// When the notification was created.
    pub created_at: DateTime<Utc>,

    /// When the notification was read (in-app only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_at: Option<DateTime<Utc>>,
}

impl Notification {
    /// Create a new notification.
    pub fn new(
        user_id: Uuid,
        category: NotificationCategory,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            category,
            title: title.into(),
            body: body.into(),
            priority: NotificationPriority::default(),
            action_url: None,
            data: None,
            created_at: Utc::now(),
            read_at: None,
        }
    }

    /// Set the priority.
    #[must_use]
    pub fn with_priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the action URL.
    #[must_use]
    pub fn with_action_url(mut self, url: impl Into<String>) -> Self {
        self.action_url = Some(url.into());
        self
    }

    /// Set additional data.
    #[must_use]
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Check if the notification is read.
    pub fn is_read(&self) -> bool {
        self.read_at.is_some()
    }
}

// ============================================================================
// Notification Preferences
// ============================================================================

/// User notification preferences for a category.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationPreference {
    /// User ID.
    pub user_id: Uuid,

    /// Category.
    pub category: NotificationCategory,

    /// Whether push notifications are enabled.
    pub push_enabled: bool,

    /// Whether email notifications are enabled.
    pub email_enabled: bool,

    /// Whether in-app notifications are enabled.
    pub in_app_enabled: bool,
}

impl NotificationPreference {
    /// Create default preferences for a category.
    pub fn default_for_category(user_id: Uuid, category: NotificationCategory) -> Self {
        let defaults = category.default_channels();
        Self {
            user_id,
            category,
            push_enabled: defaults.contains(&NotificationChannel::Push),
            email_enabled: defaults.contains(&NotificationChannel::Email),
            in_app_enabled: defaults.contains(&NotificationChannel::InApp),
        }
    }

    /// Check if a channel is enabled.
    pub fn is_channel_enabled(&self, channel: NotificationChannel) -> bool {
        match channel {
            NotificationChannel::Push => self.push_enabled,
            NotificationChannel::Email => self.email_enabled,
            NotificationChannel::InApp => self.in_app_enabled,
        }
    }

    /// Get enabled channels.
    pub fn enabled_channels(&self) -> Vec<NotificationChannel> {
        let mut channels = Vec::new();
        if self.push_enabled {
            channels.push(NotificationChannel::Push);
        }
        if self.email_enabled {
            channels.push(NotificationChannel::Email);
        }
        if self.in_app_enabled {
            channels.push(NotificationChannel::InApp);
        }
        channels
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Notification system errors.
#[derive(Debug, Error)]
pub enum NotificationError {
    /// Failed to resolve recipients.
    #[error("Failed to resolve recipients: {0}")]
    RecipientResolution(String),

    /// Failed to send push notification.
    #[error("Push notification failed: {0}")]
    PushFailed(String),

    /// Failed to send email.
    #[error("Email notification failed: {0}")]
    EmailFailed(String),

    /// Failed to store in-app notification.
    #[error("In-app notification failed: {0}")]
    InAppFailed(String),

    /// Database error.
    #[error("Database error: {0}")]
    Database(String),

    /// User preferences not found.
    #[error("Preferences not found for user {0}")]
    PreferencesNotFound(Uuid),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded for user {0}")]
    RateLimitExceeded(Uuid),

    /// Transport not configured (e.g. FCM_PROJECT_ID unset).
    /// Issue #484: the pipeline maps this to `skipped` rather than
    /// `sent` so callers get accurate delivery counts.
    #[error("Push transport not configured")]
    PushNotConfigured,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_channel_all() {
        let channels = NotificationChannel::all();
        assert_eq!(channels.len(), 3);
    }

    #[test]
    fn test_notification_category_all() {
        let categories = NotificationCategory::all();
        assert_eq!(categories.len(), 8);
    }

    #[test]
    fn test_notification_creation() {
        let user_id = Uuid::new_v4();
        let notification = Notification::new(
            user_id,
            NotificationCategory::Announcements,
            "Test Title",
            "Test body",
        );

        assert_eq!(notification.user_id, user_id);
        assert_eq!(notification.category, NotificationCategory::Announcements);
        assert_eq!(notification.title, "Test Title");
        assert!(!notification.is_read());
    }

    #[test]
    fn test_notification_builder() {
        let user_id = Uuid::new_v4();
        let notification = Notification::new(
            user_id,
            NotificationCategory::Faults,
            "Fault Update",
            "Your fault has been resolved",
        )
        .with_priority(NotificationPriority::High)
        .with_action_url("/faults/123");

        assert_eq!(notification.priority, NotificationPriority::High);
        assert_eq!(notification.action_url, Some("/faults/123".to_string()));
    }

    #[test]
    fn test_default_preferences() {
        let user_id = Uuid::new_v4();
        let pref = NotificationPreference::default_for_category(
            user_id,
            NotificationCategory::Announcements,
        );

        // Announcements should have all channels enabled by default
        assert!(pref.push_enabled);
        assert!(pref.email_enabled);
        assert!(pref.in_app_enabled);
    }

    #[test]
    fn test_enabled_channels() {
        let user_id = Uuid::new_v4();
        let pref = NotificationPreference {
            user_id,
            category: NotificationCategory::Faults,
            push_enabled: true,
            email_enabled: false,
            in_app_enabled: true,
        };

        let channels = pref.enabled_channels();
        assert_eq!(channels.len(), 2);
        assert!(channels.contains(&NotificationChannel::Push));
        assert!(channels.contains(&NotificationChannel::InApp));
        assert!(!channels.contains(&NotificationChannel::Email));
    }
}
