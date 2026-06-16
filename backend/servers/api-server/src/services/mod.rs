//! Business logic services.

pub mod actions;
pub mod accounting;
pub mod auth;
pub mod auth_policy;
pub mod document_generation;
pub mod email;
pub mod feature_service;
pub mod jwt;
pub mod notification;
pub mod notification_pipeline;
pub mod oauth;
pub mod push_fanout;
pub mod quiet_hours;
pub mod quiet_hours_drain;
pub mod scheduler;
pub mod syndication;
pub mod totp;
pub mod voice_commands;
pub mod workflow_executor;

pub use accounting::AccountingService;
pub use auth::AuthService;
pub use auth_policy::{AuthPolicyEnforcer, AuthPolicyError};
#[allow(unused_imports)]
pub use document_generation::DocumentGenerationService;
pub use email::EmailService;
pub use feature_service::FeatureService;
pub use jwt::JwtService;
#[allow(unused_imports)]
pub use notification::{NotificationService, NotificationServiceConfig};
#[allow(unused_imports)]
pub use notification_pipeline::{
    NotificationPipeline, PipelineConfig, PreferenceRouter, SmtpEmailAdapter,
};
pub use oauth::{OAuthService, OAuthServiceError};
#[allow(unused_imports)]
pub use push_fanout::{FcmConfig, FcmHttpAdapter, PushFanoutConfig, PushFanoutWorker};
#[allow(unused_imports)]
pub use quiet_hours_drain::{QuietHoursDrainConfig, QuietHoursDrainWorker};
pub use scheduler::{Scheduler, SchedulerConfig};
pub use syndication::SyndicationService;
pub use totp::TotpService;
pub use voice_commands::VoiceCommandProcessor;
pub use workflow_executor::{WorkflowEvent, WorkflowExecutor};
