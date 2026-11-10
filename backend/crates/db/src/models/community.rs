//! Community & Social models (Epic 37).
//!
//! Models for community groups, posts, events, and marketplace.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================
// Community Groups (Story 37.1)
// ============================================

/// Community group entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct CommunityGroup {
    pub id: Uuid,
    pub building_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub group_type: String,
    pub visibility: String,
    pub cover_image_url: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub rules: Option<String>,
    pub max_members: Option<i32>,
    pub auto_join_new_residents: bool,
    pub requires_approval: bool,
    pub is_official: bool,
    pub member_count: i32,
    pub post_count: i32,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create community group request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCommunityGroup {
    pub name: String,
    pub description: Option<String>,
    pub group_type: Option<String>,
    pub visibility: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub rules: Option<String>,
    pub max_members: Option<i32>,
    pub auto_join_new_residents: Option<bool>,
    pub requires_approval: Option<bool>,
}

/// Update community group request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateCommunityGroup {
    pub name: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<String>,
    pub cover_image_url: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub rules: Option<String>,
    pub max_members: Option<i32>,
    pub auto_join_new_residents: Option<bool>,
    pub requires_approval: Option<bool>,
}

/// Community group member.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct CommunityGroupMember {
    pub id: Uuid,
    pub group_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub notification_settings: serde_json::Value,
    pub joined_at: DateTime<Utc>,
    pub invited_by: Option<Uuid>,
}

/// Join group request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JoinGroupRequest {
    pub message: Option<String>,
}

/// Community group with user membership info.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CommunityGroupWithMembership {
    pub group: CommunityGroup,
    pub is_member: bool,
    pub member_role: Option<String>,
    pub joined_at: Option<DateTime<Utc>>,
}

/// Community group detail as seen by the calling user: the group plus the
/// caller's membership (`is_member`, `my_role`). The group fields are
/// flattened, so the payload is a superset of [`CommunityGroup`].
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CommunityGroupDetail {
    #[serde(flatten)]
    pub group: CommunityGroup,
    pub is_member: bool,
    pub my_role: Option<String>,
}

/// Outcome of a leave-group request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaveGroupOutcome {
    /// The membership row was removed.
    Left,
    /// The caller was not a member (idempotent no-op).
    NotMember,
    /// The caller is the group's only owner; leaving would orphan the group.
    LastOwner,
}

// ============================================
// Allowed enum values
// ============================================
//
// The `group_type`, `visibility`, `post_type`, `event_type`, RSVP `status`,
// marketplace `category` and `condition` columns are Postgres enums (migration
// 00064). Handlers validate client strings against these lists so a bad value
// is a 400 rather than a Postgres `invalid input value for enum` 500.

/// `community_group_type` values.
pub const COMMUNITY_GROUP_TYPES: &[&str] =
    &["interest", "floor", "building", "neighborhood", "official"];
/// `community_group_visibility` values.
pub const COMMUNITY_GROUP_VISIBILITIES: &[&str] = &["public", "private", "secret"];
/// `community_post_type` values.
pub const COMMUNITY_POST_TYPES: &[&str] = &[
    "text",
    "photo",
    "poll",
    "event",
    "marketplace",
    "question",
    "announcement",
];
/// Accepted `community_post_reactions.reaction_type` values.
pub const COMMUNITY_REACTION_TYPES: &[&str] = &["like", "love", "helpful", "funny"];
/// `community_event_type` values.
pub const COMMUNITY_EVENT_TYPES: &[&str] = &[
    "social",
    "meeting",
    "workshop",
    "volunteer",
    "sports",
    "cultural",
    "other",
];
/// `event_rsvp_status` values.
pub const EVENT_RSVP_STATUSES: &[&str] = &["going", "maybe", "not_going", "waitlist"];
/// `marketplace_category` values.
pub const MARKETPLACE_CATEGORIES: &[&str] = &[
    "furniture",
    "electronics",
    "clothing",
    "sports",
    "books",
    "toys",
    "home_garden",
    "services",
    "free",
    "wanted",
    "other",
];
/// `item_condition` values.
pub const MARKETPLACE_CONDITIONS: &[&str] = &["new", "like_new", "good", "fair", "for_parts"];

// ============================================
// Community Posts (Story 37.2)
// ============================================

/// Community post entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct CommunityPost {
    pub id: Uuid,
    pub group_id: Uuid,
    pub author_id: Uuid,
    pub post_type: String,
    pub title: Option<String>,
    pub content: String,
    pub media_urls: Option<Vec<String>>,
    pub poll_options: Option<serde_json::Value>,
    pub poll_ends_at: Option<DateTime<Utc>>,
    pub poll_multiple_choice: Option<bool>,
    pub is_pinned: bool,
    pub is_locked: bool,
    pub is_anonymous: bool,
    pub view_count: i32,
    pub like_count: i32,
    pub comment_count: i32,
    pub share_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

/// Create community post request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCommunityPost {
    pub post_type: Option<String>,
    pub title: Option<String>,
    pub content: String,
    pub media_urls: Option<Vec<String>>,
    pub poll_options: Option<Vec<PollOption>>,
    pub poll_ends_at: Option<DateTime<Utc>>,
    pub poll_multiple_choice: Option<bool>,
    pub is_anonymous: Option<bool>,
}

/// Poll option for creating polls.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PollOption {
    pub text: String,
}

/// Update community post request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateCommunityPost {
    pub title: Option<String>,
    pub content: Option<String>,
    pub media_urls: Option<Vec<String>>,
    pub is_pinned: Option<bool>,
    pub is_locked: Option<bool>,
}

/// Post with author info.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CommunityPostWithAuthor {
    pub id: Uuid,
    pub group_id: Uuid,
    pub post_type: String,
    pub title: Option<String>,
    pub content: String,
    pub media_urls: Option<Vec<String>>,
    pub author_id: Uuid,
    pub author_name: String,
    pub author_avatar_url: Option<String>,
    pub is_anonymous: bool,
    pub is_pinned: bool,
    pub view_count: i32,
    pub like_count: i32,
    pub comment_count: i32,
    pub has_liked: bool,
    pub created_at: DateTime<Utc>,
}

/// Community comment entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct CommunityComment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub author_id: Uuid,
    pub content: String,
    pub is_anonymous: bool,
    pub like_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

/// Create comment request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCommunityComment {
    pub content: String,
    pub parent_id: Option<Uuid>,
    pub is_anonymous: Option<bool>,
}

// ============================================
// Community Events (Story 37.3)
// ============================================

/// Community event entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct CommunityEvent {
    pub id: Uuid,
    pub group_id: Option<Uuid>,
    pub building_id: Uuid,
    pub organizer_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub event_type: String,
    pub location: Option<String>,
    pub location_details: Option<String>,
    pub is_virtual: bool,
    pub virtual_link: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub all_day: bool,
    pub recurring_rule: Option<String>,
    pub cover_image_url: Option<String>,
    pub max_attendees: Option<i32>,
    pub requires_rsvp: bool,
    pub rsvp_deadline: Option<DateTime<Utc>>,
    pub cost_per_person: Option<Decimal>,
    pub cost_currency: Option<String>,
    pub is_public: bool,
    pub status: String,
    pub attendee_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create community event request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCommunityEvent {
    pub group_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub event_type: Option<String>,
    pub location: Option<String>,
    pub location_details: Option<String>,
    pub is_virtual: Option<bool>,
    pub virtual_link: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub all_day: Option<bool>,
    pub max_attendees: Option<i32>,
    pub requires_rsvp: Option<bool>,
    pub rsvp_deadline: Option<DateTime<Utc>>,
    pub cost_per_person: Option<Decimal>,
    pub is_public: Option<bool>,
}

/// Update community event request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateCommunityEvent {
    pub title: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub location_details: Option<String>,
    pub is_virtual: Option<bool>,
    pub virtual_link: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub cover_image_url: Option<String>,
    pub max_attendees: Option<i32>,
    pub status: Option<String>,
}

/// Event RSVP entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct CommunityEventRsvp {
    pub id: Uuid,
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub guests: i32,
    pub note: Option<String>,
    pub reminder_sent: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// RSVP request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EventRsvpRequest {
    pub status: String,
    pub guests: Option<i32>,
    pub note: Option<String>,
}

/// Event with RSVP status.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CommunityEventWithRsvp {
    pub event: CommunityEvent,
    pub my_rsvp: Option<CommunityEventRsvp>,
    pub organizer_name: String,
    pub organizer_avatar_url: Option<String>,
}

// ============================================
// Marketplace (Story 37.4)
// ============================================

/// Marketplace item entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct MarketplaceItem {
    pub id: Uuid,
    pub building_id: Uuid,
    pub seller_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub condition: String,
    pub price: Option<Decimal>,
    pub currency: Option<String>,
    pub is_free: bool,
    pub is_negotiable: bool,
    pub is_trade_accepted: bool,
    pub photo_urls: Option<Vec<String>>,
    pub location: Option<String>,
    pub pickup_details: Option<String>,
    pub view_count: i32,
    pub inquiry_count: i32,
    pub status: String,
    pub sold_to: Option<Uuid>,
    pub sold_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create marketplace item request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateMarketplaceItem {
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub condition: String,
    pub price: Option<Decimal>,
    pub is_free: Option<bool>,
    pub is_negotiable: Option<bool>,
    pub is_trade_accepted: Option<bool>,
    pub photo_urls: Option<Vec<String>>,
    pub location: Option<String>,
    pub pickup_details: Option<String>,
}

/// Update marketplace item request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateMarketplaceItem {
    pub title: Option<String>,
    pub description: Option<String>,
    pub price: Option<Decimal>,
    pub is_negotiable: Option<bool>,
    pub photo_urls: Option<Vec<String>>,
    pub location: Option<String>,
    pub pickup_details: Option<String>,
    pub status: Option<String>,
}

/// Marketplace inquiry entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct MarketplaceInquiry {
    pub id: Uuid,
    pub item_id: Uuid,
    pub buyer_id: Uuid,
    pub message: String,
    pub offer_price: Option<Decimal>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Create inquiry request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateMarketplaceInquiry {
    pub message: String,
    pub offer_price: Option<Decimal>,
}

/// Item with seller info.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MarketplaceItemWithSeller {
    pub item: MarketplaceItem,
    pub seller_name: String,
    pub seller_avatar_url: Option<String>,
    pub is_favorited: bool,
}
