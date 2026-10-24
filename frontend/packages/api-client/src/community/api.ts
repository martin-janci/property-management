/**
 * Community API Client
 *
 * API functions for Community features (Epic 42).
 * Includes Groups, Posts, Events, and Marketplace.
 */

import type { PaginatedResponse } from '../announcements';
import { getToken } from '../auth';
import type {
  CommunityEvent,
  CommunityGroup,
  CommunityGroupSummary,
  CommunityPost,
  CommunityStats,
  ContactSellerRequest,
  CreateEventRequest,
  CreateGroupRequest,
  CreateItemRequest,
  CreatePostCommentRequest,
  CreatePostRequest,
  EventAttendee,
  GroupMember,
  ListEventsParams,
  ListGroupsParams,
  ListItemsParams,
  ListPostsParams,
  MarketplaceItem,
  PostComment,
  RsvpRequest,
  UpdateEventRequest,
  UpdateGroupRequest,
  UpdateItemRequest,
  UpdatePostRequest,
} from './types';

const API_BASE = '/api/v1/community';

/**
 * Build query string from parameters object.
 */
function buildQueryString(params: object): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null) {
      searchParams.append(key, String(value));
    }
  }
  const queryString = searchParams.toString();
  return queryString ? `?${queryString}` : '';
}

/**
 * Get authorization header from the configured token provider.
 */
function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

/**
 * Helper for API requests with error handling.
 */
async function apiRequest<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.message || `HTTP error ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

// ============================================
// Community Stats
// ============================================

export async function getCommunityStats(): Promise<CommunityStats> {
  return apiRequest<CommunityStats>(`${API_BASE}/stats`);
}

// ============================================
// Story 42.1: Community Groups
// ============================================

export async function listGroups(
  params?: ListGroupsParams
): Promise<PaginatedResponse<CommunityGroupSummary>> {
  const qs = buildQueryString(params || {});
  return apiRequest<PaginatedResponse<CommunityGroupSummary>>(`${API_BASE}/groups${qs}`);
}

export async function getGroup(id: string): Promise<CommunityGroup> {
  return apiRequest<CommunityGroup>(`${API_BASE}/groups/${id}`);
}

export async function getGroupMembers(groupId: string): Promise<GroupMember[]> {
  return apiRequest<GroupMember[]>(`${API_BASE}/groups/${groupId}/members`);
}

export async function createGroup(data: CreateGroupRequest): Promise<CommunityGroup> {
  return apiRequest<CommunityGroup>(`${API_BASE}/groups`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function updateGroup(id: string, data: UpdateGroupRequest): Promise<CommunityGroup> {
  return apiRequest<CommunityGroup>(`${API_BASE}/groups/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

export async function deleteGroup(id: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/groups/${id}`, {
    method: 'DELETE',
  });
}

export async function joinGroup(groupId: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/groups/${groupId}/join`, {
    method: 'POST',
  });
}

export async function leaveGroup(groupId: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/groups/${groupId}/leave`, {
    method: 'POST',
  });
}

// ============================================
// Story 42.2: Community Posts
// ============================================

export async function listPosts(
  params?: ListPostsParams
): Promise<PaginatedResponse<CommunityPost>> {
  const qs = buildQueryString(params || {});
  return apiRequest<PaginatedResponse<CommunityPost>>(`${API_BASE}/posts${qs}`);
}

export async function getPost(id: string): Promise<CommunityPost> {
  return apiRequest<CommunityPost>(`${API_BASE}/posts/${id}`);
}

export async function getPostComments(postId: string): Promise<PostComment[]> {
  return apiRequest<PostComment[]>(`${API_BASE}/posts/${postId}/comments`);
}

export async function createPost(data: CreatePostRequest): Promise<CommunityPost> {
  return apiRequest<CommunityPost>(`${API_BASE}/posts`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function updatePost(id: string, data: UpdatePostRequest): Promise<CommunityPost> {
  return apiRequest<CommunityPost>(`${API_BASE}/posts/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

export async function deletePost(id: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/posts/${id}`, {
    method: 'DELETE',
  });
}

export async function likePost(postId: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/posts/${postId}/like`, {
    method: 'POST',
  });
}

export async function unlikePost(postId: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/posts/${postId}/unlike`, {
    method: 'POST',
  });
}

export async function createPostComment(
  postId: string,
  data: CreatePostCommentRequest
): Promise<PostComment> {
  return apiRequest<PostComment>(`${API_BASE}/posts/${postId}/comments`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

// ============================================
// Story 42.3: Community Events
// ============================================

export async function listEvents(
  params?: ListEventsParams
): Promise<PaginatedResponse<CommunityEvent>> {
  const qs = buildQueryString(params || {});
  return apiRequest<PaginatedResponse<CommunityEvent>>(`${API_BASE}/events${qs}`);
}

export async function getEvent(id: string): Promise<CommunityEvent> {
  return apiRequest<CommunityEvent>(`${API_BASE}/events/${id}`);
}

export async function getEventAttendees(eventId: string): Promise<EventAttendee[]> {
  return apiRequest<EventAttendee[]>(`${API_BASE}/events/${eventId}/attendees`);
}

export async function createEvent(data: CreateEventRequest): Promise<CommunityEvent> {
  return apiRequest<CommunityEvent>(`${API_BASE}/events`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function updateEvent(id: string, data: UpdateEventRequest): Promise<CommunityEvent> {
  return apiRequest<CommunityEvent>(`${API_BASE}/events/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

export async function rsvpEvent(eventId: string, data: RsvpRequest): Promise<void> {
  return apiRequest<void>(`${API_BASE}/events/${eventId}/rsvp`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function cancelRsvp(eventId: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/events/${eventId}/rsvp`, {
    method: 'DELETE',
  });
}

// ============================================
// Story 42.4: Item Marketplace
// ============================================

export async function listItems(
  params?: ListItemsParams
): Promise<PaginatedResponse<MarketplaceItem>> {
  const qs = buildQueryString(params || {});
  return apiRequest<PaginatedResponse<MarketplaceItem>>(`${API_BASE}/marketplace${qs}`);
}

export async function getItem(id: string): Promise<MarketplaceItem> {
  return apiRequest<MarketplaceItem>(`${API_BASE}/marketplace/${id}`);
}

export async function createItem(data: CreateItemRequest): Promise<MarketplaceItem> {
  return apiRequest<MarketplaceItem>(`${API_BASE}/marketplace`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function updateItem(id: string, data: UpdateItemRequest): Promise<MarketplaceItem> {
  return apiRequest<MarketplaceItem>(`${API_BASE}/marketplace/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

export async function deleteItem(id: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/marketplace/${id}`, {
    method: 'DELETE',
  });
}

export async function contactSeller(itemId: string, data: ContactSellerRequest): Promise<void> {
  return apiRequest<void>(`${API_BASE}/marketplace/${itemId}/contact`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}
