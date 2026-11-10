/**
 * Community Types
 *
 * camelCase mirrors of the api-server Community models
 * (`backend/crates/db/src/models/community.rs`, served by
 * `backend/servers/api-server/src/routes/community.rs`, mounted at
 * `/api/v1/community`). The server speaks snake_case; `api.ts` maps every
 * payload in both directions, so nothing outside this module ever sees a
 * snake_case key.
 *
 * Only the shapes the server actually serves are modelled here. There is no
 * server endpoint for: a cross-group feed, group members, editing a group /
 * post / event / item, deleting a post / event / item, listing comments,
 * attendees or inquiries, or community stats — so none of those exist in this
 * client either.
 */

// ============================================
// Groups
// ============================================

/** Postgres enum `community_group_type`. */
export type CommunityGroupType = 'interest' | 'floor' | 'building' | 'neighborhood' | 'official';

/** Postgres enum `community_group_visibility`. */
export type CommunityGroupVisibility = 'public' | 'private' | 'secret';

/** Postgres enum `community_member_role`. */
export type CommunityMemberRole = 'member' | 'moderator' | 'admin' | 'owner';

export interface CommunityGroup {
  id: string;
  buildingId: string;
  name: string;
  slug: string;
  description: string | null;
  groupType: CommunityGroupType;
  visibility: CommunityGroupVisibility;
  coverImageUrl: string | null;
  icon: string | null;
  color: string | null;
  rules: string | null;
  maxMembers: number | null;
  autoJoinNewResidents: boolean;
  requiresApproval: boolean;
  isOfficial: boolean;
  memberCount: number;
  postCount: number;
  createdBy: string | null;
  createdAt: string;
  updatedAt: string;
}

/**
 * A group as returned by the per-building list, flattened: the server nests
 * the group under `group` next to the caller's membership fields.
 */
export interface CommunityGroupWithMembership extends CommunityGroup {
  isMember: boolean;
  memberRole: CommunityMemberRole | null;
  joinedAt: string | null;
}

export interface CreateCommunityGroupRequest {
  name: string;
  description?: string;
  groupType?: CommunityGroupType;
  visibility?: CommunityGroupVisibility;
  icon?: string;
  color?: string;
  rules?: string;
  maxMembers?: number;
  autoJoinNewResidents?: boolean;
  requiresApproval?: boolean;
}

// ============================================
// Posts, reactions, comments
// ============================================

/** Postgres enum `community_post_type`. */
export type CommunityPostType =
  | 'text'
  | 'photo'
  | 'poll'
  | 'event'
  | 'marketplace'
  | 'question'
  | 'announcement';

export interface CommunityPost {
  id: string;
  groupId: string;
  authorId: string;
  postType: CommunityPostType;
  title: string | null;
  content: string;
  mediaUrls: string[];
  isPinned: boolean;
  isLocked: boolean;
  isAnonymous: boolean;
  viewCount: number;
  likeCount: number;
  commentCount: number;
  shareCount: number;
  createdAt: string;
  updatedAt: string;
  editedAt: string | null;
}

export interface CreateCommunityPostRequest {
  postType?: CommunityPostType;
  title?: string;
  content: string;
  mediaUrls?: string[];
  isAnonymous?: boolean;
}

/** Values accepted by `community_post_reactions.reaction_type`. */
export type CommunityReactionType = 'like' | 'love' | 'helpful' | 'funny';

export interface CommunityComment {
  id: string;
  postId: string;
  parentId: string | null;
  authorId: string;
  content: string;
  isAnonymous: boolean;
  likeCount: number;
  createdAt: string;
  updatedAt: string;
  editedAt: string | null;
}

export interface CreateCommunityCommentRequest {
  content: string;
  parentId?: string;
  isAnonymous?: boolean;
}

/** `limit` / `offset` paging accepted by the list endpoints (server default limit 20). */
export interface CommunityPageParams {
  limit?: number;
  offset?: number;
}

// ============================================
// Events
// ============================================

/** Postgres enum `community_event_type`. */
export type CommunityEventType =
  | 'social'
  | 'meeting'
  | 'workshop'
  | 'volunteer'
  | 'sports'
  | 'cultural'
  | 'other';

/** Postgres enum `event_rsvp_status`. */
export type CommunityRsvpStatus = 'going' | 'maybe' | 'not_going' | 'waitlist';

export interface CommunityEvent {
  id: string;
  groupId: string | null;
  buildingId: string;
  organizerId: string;
  title: string;
  description: string | null;
  eventType: CommunityEventType;
  location: string | null;
  locationDetails: string | null;
  isVirtual: boolean;
  virtualLink: string | null;
  startTime: string;
  endTime: string;
  allDay: boolean;
  coverImageUrl: string | null;
  maxAttendees: number | null;
  requiresRsvp: boolean;
  rsvpDeadline: string | null;
  /** Decimal, serialized by the server as a string. */
  costPerPerson: string | null;
  costCurrency: string | null;
  isPublic: boolean;
  /** `scheduled` | `cancelled` | `completed`. */
  status: string;
  attendeeCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface CreateCommunityEventRequest {
  groupId?: string;
  title: string;
  description?: string;
  eventType?: CommunityEventType;
  location?: string;
  locationDetails?: string;
  isVirtual?: boolean;
  virtualLink?: string;
  /** ISO-8601 instant. */
  startTime: string;
  /** ISO-8601 instant. */
  endTime: string;
  allDay?: boolean;
  maxAttendees?: number;
  requiresRsvp?: boolean;
  rsvpDeadline?: string;
  /** Decimal as a string, e.g. `"5.00"`. */
  costPerPerson?: string;
  isPublic?: boolean;
}

export interface CommunityEventRsvp {
  id: string;
  eventId: string;
  userId: string;
  status: CommunityRsvpStatus;
  guests: number;
  note: string | null;
  reminderSent: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CommunityRsvpRequest {
  status: CommunityRsvpStatus;
  guests?: number;
  note?: string;
}

// ============================================
// Marketplace
// ============================================

/** Postgres enum `marketplace_category`. */
export type MarketplaceCategory =
  | 'furniture'
  | 'electronics'
  | 'clothing'
  | 'sports'
  | 'books'
  | 'toys'
  | 'home_garden'
  | 'services'
  | 'free'
  | 'wanted'
  | 'other';

/** Postgres enum `item_condition`. */
export type MarketplaceItemCondition = 'new' | 'like_new' | 'good' | 'fair' | 'for_parts';

export interface MarketplaceItem {
  id: string;
  buildingId: string;
  sellerId: string;
  title: string;
  description: string | null;
  category: MarketplaceCategory;
  condition: MarketplaceItemCondition;
  /** Decimal, serialized by the server as a string. */
  price: string | null;
  currency: string | null;
  isFree: boolean;
  isNegotiable: boolean;
  isTradeAccepted: boolean;
  photoUrls: string[];
  location: string | null;
  pickupDetails: string | null;
  viewCount: number;
  inquiryCount: number;
  /** `active` | `sold` | `reserved` | `expired` | `removed`. */
  status: string;
  soldTo: string | null;
  soldAt: string | null;
  expiresAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateMarketplaceItemRequest {
  title: string;
  description?: string;
  category: MarketplaceCategory;
  condition: MarketplaceItemCondition;
  /** Decimal as a string, e.g. `"25.00"`. */
  price?: string;
  isFree?: boolean;
  isNegotiable?: boolean;
  isTradeAccepted?: boolean;
  photoUrls?: string[];
  location?: string;
  pickupDetails?: string;
}

export interface ListMarketplaceItemsParams extends CommunityPageParams {
  category?: MarketplaceCategory;
}

export interface MarketplaceInquiry {
  id: string;
  itemId: string;
  buyerId: string;
  message: string;
  /** Decimal, serialized by the server as a string. */
  offerPrice: string | null;
  status: string;
  createdAt: string;
}

export interface CreateMarketplaceInquiryRequest {
  message: string;
  /** Decimal as a string. */
  offerPrice?: string;
}
