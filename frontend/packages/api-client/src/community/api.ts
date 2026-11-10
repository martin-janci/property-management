/**
 * Community API Client
 *
 * Thin wrappers over the api-server Community routes
 * (`backend/servers/api-server/src/routes/community.rs`, nested at
 * `/api/v1/community` in `lib.rs`). Every path, method, query and body here
 * matches the server router 1:1:
 *
 *   GET    /buildings/{building_id}/groups        list groups (+ caller membership)
 *   POST   /buildings/{building_id}/groups        create group
 *   GET    /groups/{id}                           get group
 *   DELETE /groups/{id}                           delete group (owner / org manager)
 *   POST   /groups/{id}/join                      join group
 *   POST   /groups/{id}/leave                     leave group
 *   GET    /groups/{group_id}/posts               list posts (?limit&offset)
 *   POST   /groups/{group_id}/posts               create post
 *   POST   /posts/{id}/reactions                  add reaction
 *   POST   /posts/{id}/comments                   create comment
 *   GET    /buildings/{building_id}/events        list upcoming events (?limit)
 *   POST   /buildings/{building_id}/events        create event
 *   POST   /events/{id}/rsvp                      RSVP (upsert)
 *   GET    /buildings/{building_id}/marketplace   list items (?category&limit&offset)
 *   POST   /buildings/{building_id}/marketplace   create item
 *   GET    /marketplace/{id}                      get item
 *   POST   /marketplace/{id}/inquiries            contact seller
 *
 * Requests go through the shared `authenticatedFetch` client, so
 * Authorization / X-Tenant-ID, base-URL resolution and the MFA retry are
 * handled centrally. Payloads are mapped snake_case <-> camelCase here.
 */

import { ApiError, authenticatedFetch } from '../lib/fetch';
import type {
  CommunityComment,
  CommunityEvent,
  CommunityEventRsvp,
  CommunityGroup,
  CommunityGroupWithMembership,
  CommunityPageParams,
  CommunityPost,
  CommunityReactionType,
  CommunityRsvpRequest,
  CreateCommunityCommentRequest,
  CreateCommunityEventRequest,
  CreateCommunityGroupRequest,
  CreateCommunityPostRequest,
  CreateMarketplaceInquiryRequest,
  CreateMarketplaceItemRequest,
  ListMarketplaceItemsParams,
  MarketplaceInquiry,
  MarketplaceItem,
} from './types';

export const COMMUNITY_API_BASE = '/api/v1/community';

type Raw = Record<string, unknown>;

// ============================================
// Transport
// ============================================

async function request<T>(
  path: string,
  init: { method?: string; body?: unknown; signal?: AbortSignal } = {}
): Promise<T> {
  const headers: Record<string, string> = { Accept: 'application/json' };
  let body: string | undefined;
  if (init.body !== undefined) {
    headers['Content-Type'] = 'application/json';
    body = JSON.stringify(init.body);
  }
  const response = await authenticatedFetch(`${COMMUNITY_API_BASE}${path}`, {
    method: init.method ?? 'GET',
    headers,
    body,
    signal: init.signal,
  });

  if (!response.ok) {
    const err = (await response.json().catch(() => ({}))) as {
      code?: string;
      error?: string;
      message?: string;
    };
    throw new ApiError(
      response.status,
      err.message || err.code || err.error || `HTTP ${response.status}`,
      err.code ?? err.error
    );
  }

  if (response.status === 204) return undefined as T;
  const text = await response.text();
  return (text ? JSON.parse(text) : undefined) as T;
}

function qs(params: Record<string, string | number | undefined>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== '') search.append(key, String(value));
  }
  const s = search.toString();
  return s ? `?${s}` : '';
}

/** Drop `undefined` entries so optional fields are omitted, not sent as null. */
function compact(obj: Raw): Raw {
  return Object.fromEntries(Object.entries(obj).filter(([, v]) => v !== undefined));
}

const str = (v: unknown): string => (typeof v === 'string' ? v : v == null ? '' : String(v));
const strOrNull = (v: unknown): string | null =>
  v === null || v === undefined ? null : typeof v === 'string' ? v : String(v);
const num = (v: unknown): number => (typeof v === 'number' ? v : Number(v ?? 0) || 0);
const numOrNull = (v: unknown): number | null =>
  v === null || v === undefined ? null : typeof v === 'number' ? v : Number(v);
const bool = (v: unknown): boolean => v === true;
const strArr = (v: unknown): string[] =>
  Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
const arr = (v: unknown): Raw[] => (Array.isArray(v) ? (v as Raw[]) : []);

// ============================================
// Mappers (server snake_case -> client camelCase)
// ============================================

export function mapCommunityGroup(r: Raw): CommunityGroup {
  return {
    id: str(r.id),
    buildingId: str(r.building_id),
    name: str(r.name),
    slug: str(r.slug),
    description: strOrNull(r.description),
    groupType: str(r.group_type) as CommunityGroup['groupType'],
    visibility: str(r.visibility) as CommunityGroup['visibility'],
    coverImageUrl: strOrNull(r.cover_image_url),
    icon: strOrNull(r.icon),
    color: strOrNull(r.color),
    rules: strOrNull(r.rules),
    maxMembers: numOrNull(r.max_members),
    autoJoinNewResidents: bool(r.auto_join_new_residents),
    requiresApproval: bool(r.requires_approval),
    isOfficial: bool(r.is_official),
    memberCount: num(r.member_count),
    postCount: num(r.post_count),
    createdBy: strOrNull(r.created_by),
    createdAt: str(r.created_at),
    updatedAt: str(r.updated_at),
  };
}

export function mapCommunityGroupWithMembership(r: Raw): CommunityGroupWithMembership {
  return {
    ...mapCommunityGroup((r.group as Raw | undefined) ?? {}),
    isMember: bool(r.is_member),
    memberRole: strOrNull(r.member_role) as CommunityGroupWithMembership['memberRole'],
    joinedAt: strOrNull(r.joined_at),
  };
}

export function mapCommunityPost(r: Raw): CommunityPost {
  return {
    id: str(r.id),
    groupId: str(r.group_id),
    authorId: str(r.author_id),
    postType: str(r.post_type || 'text') as CommunityPost['postType'],
    title: strOrNull(r.title),
    content: str(r.content),
    mediaUrls: strArr(r.media_urls),
    isPinned: bool(r.is_pinned),
    isLocked: bool(r.is_locked),
    isAnonymous: bool(r.is_anonymous),
    viewCount: num(r.view_count),
    likeCount: num(r.like_count),
    commentCount: num(r.comment_count),
    shareCount: num(r.share_count),
    createdAt: str(r.created_at),
    updatedAt: str(r.updated_at),
    editedAt: strOrNull(r.edited_at),
  };
}

export function mapCommunityComment(r: Raw): CommunityComment {
  return {
    id: str(r.id),
    postId: str(r.post_id),
    parentId: strOrNull(r.parent_id),
    authorId: str(r.author_id),
    content: str(r.content),
    isAnonymous: bool(r.is_anonymous),
    likeCount: num(r.like_count),
    createdAt: str(r.created_at),
    updatedAt: str(r.updated_at),
    editedAt: strOrNull(r.edited_at),
  };
}

export function mapCommunityEvent(r: Raw): CommunityEvent {
  return {
    id: str(r.id),
    groupId: strOrNull(r.group_id),
    buildingId: str(r.building_id),
    organizerId: str(r.organizer_id),
    title: str(r.title),
    description: strOrNull(r.description),
    eventType: str(r.event_type || 'other') as CommunityEvent['eventType'],
    location: strOrNull(r.location),
    locationDetails: strOrNull(r.location_details),
    isVirtual: bool(r.is_virtual),
    virtualLink: strOrNull(r.virtual_link),
    startTime: str(r.start_time),
    endTime: str(r.end_time),
    allDay: bool(r.all_day),
    coverImageUrl: strOrNull(r.cover_image_url),
    maxAttendees: numOrNull(r.max_attendees),
    requiresRsvp: bool(r.requires_rsvp),
    rsvpDeadline: strOrNull(r.rsvp_deadline),
    costPerPerson: strOrNull(r.cost_per_person),
    costCurrency: strOrNull(r.cost_currency),
    isPublic: bool(r.is_public),
    status: str(r.status),
    attendeeCount: num(r.attendee_count),
    createdAt: str(r.created_at),
    updatedAt: str(r.updated_at),
  };
}

export function mapCommunityEventRsvp(r: Raw): CommunityEventRsvp {
  return {
    id: str(r.id),
    eventId: str(r.event_id),
    userId: str(r.user_id),
    status: str(r.status) as CommunityEventRsvp['status'],
    guests: num(r.guests),
    note: strOrNull(r.note),
    reminderSent: bool(r.reminder_sent),
    createdAt: str(r.created_at),
    updatedAt: str(r.updated_at),
  };
}

export function mapMarketplaceItem(r: Raw): MarketplaceItem {
  return {
    id: str(r.id),
    buildingId: str(r.building_id),
    sellerId: str(r.seller_id),
    title: str(r.title),
    description: strOrNull(r.description),
    category: str(r.category || 'other') as MarketplaceItem['category'],
    condition: str(r.condition || 'good') as MarketplaceItem['condition'],
    price: strOrNull(r.price),
    currency: strOrNull(r.currency),
    isFree: bool(r.is_free),
    isNegotiable: bool(r.is_negotiable),
    isTradeAccepted: bool(r.is_trade_accepted),
    photoUrls: strArr(r.photo_urls),
    location: strOrNull(r.location),
    pickupDetails: strOrNull(r.pickup_details),
    viewCount: num(r.view_count),
    inquiryCount: num(r.inquiry_count),
    status: str(r.status),
    soldTo: strOrNull(r.sold_to),
    soldAt: strOrNull(r.sold_at),
    expiresAt: strOrNull(r.expires_at),
    createdAt: str(r.created_at),
    updatedAt: str(r.updated_at),
  };
}

export function mapMarketplaceInquiry(r: Raw): MarketplaceInquiry {
  return {
    id: str(r.id),
    itemId: str(r.item_id),
    buyerId: str(r.buyer_id),
    message: str(r.message),
    offerPrice: strOrNull(r.offer_price),
    status: str(r.status),
    createdAt: str(r.created_at),
  };
}

// ============================================
// Groups
// ============================================

export async function listGroups(
  buildingId: string,
  signal?: AbortSignal
): Promise<CommunityGroupWithMembership[]> {
  const data = await request<unknown>(`/buildings/${buildingId}/groups`, { signal });
  return arr(data).map(mapCommunityGroupWithMembership);
}

export async function getGroup(id: string, signal?: AbortSignal): Promise<CommunityGroup> {
  return mapCommunityGroup(await request<Raw>(`/groups/${id}`, { signal }));
}

export async function createGroup(
  buildingId: string,
  data: CreateCommunityGroupRequest
): Promise<CommunityGroup> {
  const body = compact({
    name: data.name,
    description: data.description,
    group_type: data.groupType,
    visibility: data.visibility,
    icon: data.icon,
    color: data.color,
    rules: data.rules,
    max_members: data.maxMembers,
    auto_join_new_residents: data.autoJoinNewResidents,
    requires_approval: data.requiresApproval,
  });
  return mapCommunityGroup(
    await request<Raw>(`/buildings/${buildingId}/groups`, { method: 'POST', body })
  );
}

export async function deleteGroup(id: string): Promise<void> {
  await request<void>(`/groups/${id}`, { method: 'DELETE' });
}

export async function joinGroup(id: string): Promise<void> {
  await request<void>(`/groups/${id}/join`, { method: 'POST' });
}

export async function leaveGroup(id: string): Promise<void> {
  await request<void>(`/groups/${id}/leave`, { method: 'POST' });
}

// ============================================
// Posts
// ============================================

export async function listGroupPosts(
  groupId: string,
  params: CommunityPageParams = {},
  signal?: AbortSignal
): Promise<CommunityPost[]> {
  const data = await request<unknown>(
    `/groups/${groupId}/posts${qs({ limit: params.limit, offset: params.offset })}`,
    { signal }
  );
  return arr(data).map(mapCommunityPost);
}

export async function createPost(
  groupId: string,
  data: CreateCommunityPostRequest
): Promise<CommunityPost> {
  const body = compact({
    post_type: data.postType,
    title: data.title,
    content: data.content,
    media_urls: data.mediaUrls,
    is_anonymous: data.isAnonymous,
  });
  return mapCommunityPost(await request<Raw>(`/groups/${groupId}/posts`, { method: 'POST', body }));
}

export async function addPostReaction(
  postId: string,
  reactionType: CommunityReactionType = 'like'
): Promise<void> {
  await request<void>(`/posts/${postId}/reactions`, {
    method: 'POST',
    body: { reaction_type: reactionType },
  });
}

export async function createPostComment(
  postId: string,
  data: CreateCommunityCommentRequest
): Promise<CommunityComment> {
  const body = compact({
    content: data.content,
    parent_id: data.parentId,
    is_anonymous: data.isAnonymous,
  });
  return mapCommunityComment(
    await request<Raw>(`/posts/${postId}/comments`, { method: 'POST', body })
  );
}

// ============================================
// Events
// ============================================

/** Upcoming, scheduled events of a building (server orders by start time). */
export async function listEvents(
  buildingId: string,
  params: CommunityPageParams = {},
  signal?: AbortSignal
): Promise<CommunityEvent[]> {
  const data = await request<unknown>(
    `/buildings/${buildingId}/events${qs({ limit: params.limit, offset: params.offset })}`,
    { signal }
  );
  return arr(data).map(mapCommunityEvent);
}

export async function createEvent(
  buildingId: string,
  data: CreateCommunityEventRequest
): Promise<CommunityEvent> {
  const body = compact({
    group_id: data.groupId,
    title: data.title,
    description: data.description,
    event_type: data.eventType,
    location: data.location,
    location_details: data.locationDetails,
    is_virtual: data.isVirtual,
    virtual_link: data.virtualLink,
    start_time: data.startTime,
    end_time: data.endTime,
    all_day: data.allDay,
    max_attendees: data.maxAttendees,
    requires_rsvp: data.requiresRsvp,
    rsvp_deadline: data.rsvpDeadline,
    cost_per_person: data.costPerPerson,
    is_public: data.isPublic,
  });
  return mapCommunityEvent(
    await request<Raw>(`/buildings/${buildingId}/events`, { method: 'POST', body })
  );
}

export async function rsvpEvent(
  eventId: string,
  data: CommunityRsvpRequest
): Promise<CommunityEventRsvp> {
  const body = compact({ status: data.status, guests: data.guests, note: data.note });
  return mapCommunityEventRsvp(
    await request<Raw>(`/events/${eventId}/rsvp`, { method: 'POST', body })
  );
}

// ============================================
// Marketplace
// ============================================

export async function listItems(
  buildingId: string,
  params: ListMarketplaceItemsParams = {},
  signal?: AbortSignal
): Promise<MarketplaceItem[]> {
  const data = await request<unknown>(
    `/buildings/${buildingId}/marketplace${qs({
      category: params.category,
      limit: params.limit,
      offset: params.offset,
    })}`,
    { signal }
  );
  return arr(data).map(mapMarketplaceItem);
}

export async function getItem(id: string, signal?: AbortSignal): Promise<MarketplaceItem> {
  return mapMarketplaceItem(await request<Raw>(`/marketplace/${id}`, { signal }));
}

export async function createItem(
  buildingId: string,
  data: CreateMarketplaceItemRequest
): Promise<MarketplaceItem> {
  const body = compact({
    title: data.title,
    description: data.description,
    category: data.category,
    condition: data.condition,
    price: data.price,
    is_free: data.isFree,
    is_negotiable: data.isNegotiable,
    is_trade_accepted: data.isTradeAccepted,
    photo_urls: data.photoUrls,
    location: data.location,
    pickup_details: data.pickupDetails,
  });
  return mapMarketplaceItem(
    await request<Raw>(`/buildings/${buildingId}/marketplace`, { method: 'POST', body })
  );
}

export async function createItemInquiry(
  itemId: string,
  data: CreateMarketplaceInquiryRequest
): Promise<MarketplaceInquiry> {
  const body = compact({ message: data.message, offer_price: data.offerPrice });
  return mapMarketplaceInquiry(
    await request<Raw>(`/marketplace/${itemId}/inquiries`, { method: 'POST', body })
  );
}
