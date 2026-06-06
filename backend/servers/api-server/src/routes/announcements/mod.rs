//! Announcement routes (Epic 6: Announcements & Communication, Epic 92: Intelligent Document Generation).
//!
//! Split by surface so each concern lives in its own module:
//!
//! | Surface     | Module       | Handlers |
//! |-------------|--------------|----------|
//! | shared      | `shared`     | Request/response DTOs, constants, helpers (sanitize, target validation, recipient resolution) |
//! | crud        | `crud`       | create, list, list_published, get, update, delete |
//! | lifecycle   | `lifecycle`  | publish, schedule, archive, pin |
//! | engagement  | `engagement` | attachments (list/add/delete), read, acknowledge, acknowledgments |
//! | comments    | `comments`   | list, create, delete (Story 6.3) |
//! | stats       | `stats`      | statistics, unread-count |
//! | ai_draft    | `ai_draft`   | AI draft generation (Epic 92.4) |
//!
//! Public DTO types are re-exported here so `routes::announcements::*`
//! references continue to resolve unchanged.

mod ai_draft;
mod comments;
mod crud;
mod engagement;
mod lifecycle;
pub mod shared;
mod stats;

use crate::state::AppState;
use axum::Router;

// Re-export all public DTO types so external references via
// `routes::announcements::<Type>` keep working after the split.
pub use shared::{
    AcknowledgmentListQuery, AcknowledgmentListResponse, AcknowledgmentStatsResponse,
    AddAttachmentRequest, AnnouncementActionResponse, AnnouncementDetailResponse,
    AnnouncementDraft, AnnouncementListResponse, AttachmentsResponse, CommentResponse,
    CommentsResponse, CreateAnnouncementRequest, CreateAnnouncementResponse, CreateCommentRequest,
    DeleteCommentRequest, GenerateAiDraftRequest, GenerateAiDraftResponse, ListAnnouncementsQuery,
    ListCommentsQuery, PinAnnouncementRequest, ScheduleAnnouncementRequest, StatisticsResponse,
    UnreadCountResponse, UpdateAnnouncementRequest,
};

/// Create announcements router.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(crud::router())
        .merge(lifecycle::router())
        .merge(engagement::router())
        .merge(comments::router())
        .merge(stats::router())
        .merge(ai_draft::router())
}
