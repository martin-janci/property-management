/**
 * Contract tests for the Community API client.
 *
 * Every function is pinned to the exact path / method / query / body the
 * api-server router serves (`backend/servers/api-server/src/routes/community.rs`
 * nested at `/api/v1/community`), and each response mapper is checked against
 * the server's snake_case shape. The client previously called ~19 paths that do
 * not exist (`/community/groups`, `/community/posts`, `/community/events`,
 * `/groups/{id}/members`, …) and expected camelCase/paginated envelopes the
 * server never sends.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from '../lib/fetch';
import {
  addPostReaction,
  createEvent,
  createGroup,
  createItem,
  createItemInquiry,
  createPost,
  createPostComment,
  deleteGroup,
  getGroup,
  getItem,
  joinGroup,
  leaveGroup,
  listEvents,
  listGroupPosts,
  listGroups,
  listItems,
  rsvpEvent,
} from './api';

const fetchMock = vi.fn();

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function empty(status = 200): Response {
  return new Response(status === 204 ? null : '', { status });
}

function lastCall(): { url: string; method: string; body: unknown } {
  const [url, init] = fetchMock.mock.calls[fetchMock.mock.calls.length - 1] as [
    string,
    RequestInit,
  ];
  return {
    // `authenticatedFetch` resolves paths against the generated client's base
    // URL; the contract under test is the path + query, so drop the origin.
    url: url.replace(/^https?:\/\/[^/]+/, ''),
    method: init.method ?? 'GET',
    body: init.body ? JSON.parse(init.body as string) : undefined,
  };
}

const rawGroup = {
  id: 'g1',
  building_id: 'b1',
  name: 'Gardeners',
  slug: 'gardeners',
  description: 'Plants',
  group_type: 'interest',
  visibility: 'public',
  cover_image_url: null,
  icon: null,
  color: null,
  rules: null,
  max_members: null,
  auto_join_new_residents: false,
  requires_approval: false,
  is_official: true,
  member_count: 4,
  post_count: 2,
  created_by: 'u1',
  created_at: '2026-11-01T10:00:00Z',
  updated_at: '2026-11-01T10:00:00Z',
};

const rawPost = {
  id: 'p1',
  group_id: 'g1',
  author_id: 'u1',
  post_type: 'text',
  title: null,
  content: 'Hello',
  media_urls: null,
  poll_options: null,
  poll_ends_at: null,
  poll_multiple_choice: null,
  is_pinned: false,
  is_locked: false,
  is_anonymous: false,
  view_count: 1,
  like_count: 3,
  comment_count: 0,
  share_count: 0,
  created_at: '2026-11-02T10:00:00Z',
  updated_at: '2026-11-02T10:00:00Z',
  edited_at: null,
};

const rawEvent = {
  id: 'e1',
  group_id: null,
  building_id: 'b1',
  organizer_id: 'u1',
  title: 'BBQ',
  description: null,
  event_type: 'social',
  location: 'Courtyard',
  location_details: null,
  is_virtual: false,
  virtual_link: null,
  start_time: '2026-12-01T16:00:00Z',
  end_time: '2026-12-01T19:00:00Z',
  all_day: false,
  cover_image_url: null,
  max_attendees: 30,
  requires_rsvp: false,
  rsvp_deadline: null,
  cost_per_person: '5.00',
  cost_currency: 'EUR',
  is_public: true,
  status: 'scheduled',
  attendee_count: 7,
  created_at: '2026-11-01T10:00:00Z',
  updated_at: '2026-11-01T10:00:00Z',
};

const rawItem = {
  id: 'i1',
  building_id: 'b1',
  seller_id: 'u2',
  title: 'Sofa',
  description: 'Comfy',
  category: 'furniture',
  condition: 'good',
  price: '120.50',
  currency: 'EUR',
  is_free: false,
  is_negotiable: true,
  is_trade_accepted: false,
  photo_urls: ['https://x/y.jpg'],
  location: null,
  pickup_details: null,
  view_count: 9,
  inquiry_count: 1,
  status: 'active',
  sold_to: null,
  sold_at: null,
  expires_at: null,
  created_at: '2026-11-01T10:00:00Z',
  updated_at: '2026-11-01T10:00:00Z',
};

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal('fetch', fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('groups', () => {
  it('listGroups GETs the per-building list and flattens membership', async () => {
    fetchMock.mockResolvedValueOnce(
      json([{ group: rawGroup, is_member: true, member_role: 'owner', joined_at: 'x' }])
    );
    const groups = await listGroups('b1');
    expect(lastCall()).toMatchObject({
      url: '/api/v1/community/buildings/b1/groups',
      method: 'GET',
    });
    expect(groups).toEqual([
      expect.objectContaining({
        id: 'g1',
        buildingId: 'b1',
        groupType: 'interest',
        isOfficial: true,
        memberCount: 4,
        isMember: true,
        memberRole: 'owner',
      }),
    ]);
  });

  it('getGroup GETs /groups/{id} and maps snake_case', async () => {
    fetchMock.mockResolvedValueOnce(json(rawGroup));
    const group = await getGroup('g1');
    expect(lastCall()).toMatchObject({ url: '/api/v1/community/groups/g1', method: 'GET' });
    expect(group).toMatchObject({ id: 'g1', postCount: 2, createdBy: 'u1', description: 'Plants' });
  });

  it('createGroup POSTs a snake_case body to the building and omits unset fields', async () => {
    fetchMock.mockResolvedValueOnce(json(rawGroup, 201));
    await createGroup('b1', {
      name: 'Gardeners',
      description: 'Plants',
      groupType: 'interest',
      visibility: 'private',
    });
    expect(lastCall()).toEqual({
      url: '/api/v1/community/buildings/b1/groups',
      method: 'POST',
      body: {
        name: 'Gardeners',
        description: 'Plants',
        group_type: 'interest',
        visibility: 'private',
      },
    });
  });

  it('createGroup surfaces server errors as ApiError', async () => {
    fetchMock.mockResolvedValueOnce(
      json({ code: 'BUILDING_ACCESS_DENIED', message: 'Caller is not a member' }, 403)
    );
    const err = await createGroup('b1', { name: 'x' }).catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err).toMatchObject({ status: 403, code: 'BUILDING_ACCESS_DENIED' });
    expect(err.message).toBe('Caller is not a member');
  });

  it('deleteGroup DELETEs /groups/{id}', async () => {
    fetchMock.mockResolvedValueOnce(empty(204));
    await deleteGroup('g1');
    expect(lastCall()).toMatchObject({ url: '/api/v1/community/groups/g1', method: 'DELETE' });
  });

  it('joinGroup / leaveGroup POST with no body and tolerate an empty 200', async () => {
    fetchMock.mockResolvedValueOnce(empty(200));
    await joinGroup('g1');
    expect(lastCall()).toEqual({
      url: '/api/v1/community/groups/g1/join',
      method: 'POST',
      body: undefined,
    });
    fetchMock.mockResolvedValueOnce(empty(200));
    await leaveGroup('g1');
    expect(lastCall()).toEqual({
      url: '/api/v1/community/groups/g1/leave',
      method: 'POST',
      body: undefined,
    });
  });
});

describe('posts', () => {
  it('listGroupPosts GETs /groups/{id}/posts with limit/offset', async () => {
    fetchMock.mockResolvedValueOnce(json([rawPost]));
    const posts = await listGroupPosts('g1', { limit: 10, offset: 20 });
    expect(lastCall().url).toBe('/api/v1/community/groups/g1/posts?limit=10&offset=20');
    expect(posts[0]).toMatchObject({
      id: 'p1',
      groupId: 'g1',
      authorId: 'u1',
      postType: 'text',
      likeCount: 3,
      mediaUrls: [],
    });
  });

  it('listGroupPosts sends no query string when no paging is given', async () => {
    fetchMock.mockResolvedValueOnce(json([]));
    await listGroupPosts('g1');
    expect(lastCall().url).toBe('/api/v1/community/groups/g1/posts');
  });

  it('createPost POSTs to the group with a snake_case body', async () => {
    fetchMock.mockResolvedValueOnce(json(rawPost, 201));
    await createPost('g1', { content: 'Hello', postType: 'question', isAnonymous: true });
    expect(lastCall()).toEqual({
      url: '/api/v1/community/groups/g1/posts',
      method: 'POST',
      body: { content: 'Hello', post_type: 'question', is_anonymous: true },
    });
  });

  it('addPostReaction POSTs reaction_type (default like)', async () => {
    fetchMock.mockResolvedValueOnce(empty(200));
    await addPostReaction('p1');
    expect(lastCall()).toEqual({
      url: '/api/v1/community/posts/p1/reactions',
      method: 'POST',
      body: { reaction_type: 'like' },
    });
  });

  it('createPostComment POSTs to /posts/{id}/comments', async () => {
    fetchMock.mockResolvedValueOnce(
      json(
        {
          id: 'c1',
          post_id: 'p1',
          parent_id: null,
          author_id: 'u1',
          content: 'Nice',
          is_anonymous: false,
          like_count: 0,
          created_at: 't',
          updated_at: 't',
          edited_at: null,
        },
        201
      )
    );
    const comment = await createPostComment('p1', { content: 'Nice', parentId: 'c0' });
    expect(lastCall()).toEqual({
      url: '/api/v1/community/posts/p1/comments',
      method: 'POST',
      body: { content: 'Nice', parent_id: 'c0' },
    });
    expect(comment).toMatchObject({ id: 'c1', postId: 'p1', parentId: null });
  });
});

describe('events', () => {
  it('listEvents GETs the per-building list with limit', async () => {
    fetchMock.mockResolvedValueOnce(json([rawEvent]));
    const events = await listEvents('b1', { limit: 50 });
    expect(lastCall()).toMatchObject({
      url: '/api/v1/community/buildings/b1/events?limit=50',
      method: 'GET',
    });
    expect(events[0]).toMatchObject({
      id: 'e1',
      startTime: '2026-12-01T16:00:00Z',
      attendeeCount: 7,
      costPerPerson: '5.00',
      maxAttendees: 30,
    });
  });

  it('createEvent POSTs a snake_case body', async () => {
    fetchMock.mockResolvedValueOnce(json(rawEvent, 201));
    await createEvent('b1', {
      title: 'BBQ',
      eventType: 'social',
      location: 'Courtyard',
      startTime: '2026-12-01T16:00:00.000Z',
      endTime: '2026-12-01T19:00:00.000Z',
      maxAttendees: 30,
    });
    expect(lastCall()).toEqual({
      url: '/api/v1/community/buildings/b1/events',
      method: 'POST',
      body: {
        title: 'BBQ',
        event_type: 'social',
        location: 'Courtyard',
        start_time: '2026-12-01T16:00:00.000Z',
        end_time: '2026-12-01T19:00:00.000Z',
        max_attendees: 30,
      },
    });
  });

  it('rsvpEvent POSTs the status and maps the RSVP', async () => {
    fetchMock.mockResolvedValueOnce(
      json({
        id: 'r1',
        event_id: 'e1',
        user_id: 'u1',
        status: 'maybe',
        guests: 0,
        note: null,
        reminder_sent: false,
        created_at: 't',
        updated_at: 't',
      })
    );
    const rsvp = await rsvpEvent('e1', { status: 'maybe' });
    expect(lastCall()).toEqual({
      url: '/api/v1/community/events/e1/rsvp',
      method: 'POST',
      body: { status: 'maybe' },
    });
    expect(rsvp).toMatchObject({ eventId: 'e1', status: 'maybe' });
  });
});

describe('marketplace', () => {
  it('listItems GETs the per-building list with category/limit/offset', async () => {
    fetchMock.mockResolvedValueOnce(json([rawItem]));
    const items = await listItems('b1', { category: 'furniture', limit: 24, offset: 0 });
    expect(lastCall()).toMatchObject({
      url: '/api/v1/community/buildings/b1/marketplace?category=furniture&limit=24&offset=0',
      method: 'GET',
    });
    expect(items[0]).toMatchObject({
      id: 'i1',
      sellerId: 'u2',
      price: '120.50',
      isNegotiable: true,
      photoUrls: ['https://x/y.jpg'],
      inquiryCount: 1,
    });
  });

  it('getItem GETs /marketplace/{id}', async () => {
    fetchMock.mockResolvedValueOnce(json(rawItem));
    const item = await getItem('i1');
    expect(lastCall()).toMatchObject({ url: '/api/v1/community/marketplace/i1', method: 'GET' });
    expect(item.title).toBe('Sofa');
  });

  it('createItem POSTs a snake_case body to the building', async () => {
    fetchMock.mockResolvedValueOnce(json(rawItem, 201));
    await createItem('b1', {
      title: 'Sofa',
      category: 'furniture',
      condition: 'like_new',
      price: '120.50',
      isNegotiable: true,
    });
    expect(lastCall()).toEqual({
      url: '/api/v1/community/buildings/b1/marketplace',
      method: 'POST',
      body: {
        title: 'Sofa',
        category: 'furniture',
        condition: 'like_new',
        price: '120.50',
        is_negotiable: true,
      },
    });
  });

  it('createItemInquiry POSTs to /marketplace/{id}/inquiries', async () => {
    fetchMock.mockResolvedValueOnce(
      json(
        {
          id: 'q1',
          item_id: 'i1',
          buyer_id: 'u1',
          message: 'Still available?',
          offer_price: '100',
          status: 'pending',
          created_at: 't',
        },
        201
      )
    );
    const inquiry = await createItemInquiry('i1', {
      message: 'Still available?',
      offerPrice: '100',
    });
    expect(lastCall()).toEqual({
      url: '/api/v1/community/marketplace/i1/inquiries',
      method: 'POST',
      body: { message: 'Still available?', offer_price: '100' },
    });
    expect(inquiry).toMatchObject({ itemId: 'i1', offerPrice: '100', status: 'pending' });
  });
});
