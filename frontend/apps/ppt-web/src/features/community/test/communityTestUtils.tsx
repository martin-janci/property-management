/**
 * Test harness for the Community pages: a URL-routed `fetch` mock (so the real
 * `@ppt/api-client` functions and hooks run and the exact endpoints can be
 * asserted) plus a QueryClient + MemoryRouter render helper.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render } from '@testing-library/react';
import type { ReactElement } from 'react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { vi } from 'vitest';

export interface RecordedCall {
  method: string;
  path: string;
  search: string;
  body: unknown;
}

type Reply = { status?: number; body?: unknown };
type Responder = Reply | ((call: RecordedCall) => Reply);

export interface MockRoute {
  method: string;
  /** Exact pathname, or a RegExp tested against the pathname. */
  path: string | RegExp;
  reply: Responder;
}

/** Install a `fetch` stub that answers by method + pathname and records every call. */
export function mockApi(routes: MockRoute[]) {
  const calls: RecordedCall[] = [];
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const raw = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
    const url = new URL(raw, 'http://localhost');
    const method = (init?.method ?? 'GET').toUpperCase();
    const call: RecordedCall = {
      method,
      path: url.pathname,
      search: url.search,
      body: typeof init?.body === 'string' && init.body ? JSON.parse(init.body) : undefined,
    };
    calls.push(call);
    const route = routes.find(
      (r) =>
        r.method === method &&
        (typeof r.path === 'string' ? r.path === url.pathname : r.path.test(url.pathname))
    );
    if (!route) {
      return new Response(JSON.stringify({ code: 'NOT_MOCKED', message: 'not mocked' }), {
        status: 404,
        headers: { 'Content-Type': 'application/json' },
      });
    }
    const reply = typeof route.reply === 'function' ? route.reply(call) : route.reply;
    const status = reply.status ?? 200;
    const body = reply.body === undefined || status === 204 ? null : JSON.stringify(reply.body);
    return new Response(body, { status, headers: { 'Content-Type': 'application/json' } });
  });
  vi.stubGlobal('fetch', fetchMock);
  return {
    calls,
    find: (method: string, path: string) =>
      calls.filter((c) => c.method === method && c.path === path),
  };
}

function LocationProbe() {
  const location = useLocation();
  return <div data-testid="location">{`${location.pathname}${location.search}`}</div>;
}

/** Render `element` at `url` under a route `path`, with a fresh QueryClient. */
export function renderPage(element: ReactElement, url: string, path: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={[url]}>
        <Routes>
          <Route path={path} element={element} />
          <Route path="*" element={null} />
        </Routes>
        <LocationProbe />
      </MemoryRouter>
    </QueryClientProvider>
  );
}

// ---------------------------------------------------------------------------
// Server-shaped (snake_case) fixtures
// ---------------------------------------------------------------------------

export const BUILDING_ID = 'b-1';
export const ME = 'user-me';

export const myUnitsFixture = [
  {
    association: { residentId: 'r1', residentType: 'owner', isPrimary: true, isActive: true },
    unit: { id: 'u1', buildingId: BUILDING_ID, designation: 'A1' },
    building: {
      id: BUILDING_ID,
      name: 'Sunny House',
      street: 'Main 1',
      city: 'Bratislava',
      postalCode: '81101',
    },
  },
];

export function rawGroup(overrides: Record<string, unknown> = {}) {
  return {
    id: 'g-1',
    building_id: BUILDING_ID,
    name: 'Gardeners',
    slug: 'gardeners',
    description: 'We grow tomatoes',
    group_type: 'interest',
    visibility: 'public',
    cover_image_url: null,
    icon: null,
    color: null,
    rules: null,
    max_members: null,
    auto_join_new_residents: false,
    requires_approval: false,
    is_official: false,
    member_count: 3,
    post_count: 1,
    created_by: 'someone-else',
    created_at: '2026-10-01T10:00:00Z',
    updated_at: '2026-10-01T10:00:00Z',
    ...overrides,
  };
}

export function rawGroupWithMembership(
  overrides: Record<string, unknown> = {},
  membership: { is_member?: boolean; member_role?: string | null } = {}
) {
  return {
    group: rawGroup(overrides),
    is_member: membership.is_member ?? false,
    member_role: membership.member_role ?? null,
    joined_at: membership.is_member ? '2026-10-02T10:00:00Z' : null,
  };
}

export function rawPost(overrides: Record<string, unknown> = {}) {
  return {
    id: 'p-1',
    group_id: 'g-1',
    author_id: 'someone-else',
    post_type: 'text',
    title: null,
    content: 'Tomatoes are ripe!',
    media_urls: null,
    poll_options: null,
    poll_ends_at: null,
    poll_multiple_choice: null,
    is_pinned: false,
    is_locked: false,
    is_anonymous: false,
    view_count: 0,
    like_count: 2,
    comment_count: 0,
    share_count: 0,
    created_at: '2026-11-03T10:00:00Z',
    updated_at: '2026-11-03T10:00:00Z',
    edited_at: null,
    ...overrides,
  };
}

export function rawEvent(overrides: Record<string, unknown> = {}) {
  return {
    id: 'e-1',
    group_id: null,
    building_id: BUILDING_ID,
    organizer_id: 'someone-else',
    title: 'Courtyard BBQ',
    description: 'Bring a salad',
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
    cost_per_person: null,
    cost_currency: 'EUR',
    is_public: true,
    status: 'scheduled',
    attendee_count: 7,
    created_at: '2026-11-01T10:00:00Z',
    updated_at: '2026-11-01T10:00:00Z',
    ...overrides,
  };
}

export function rawItem(overrides: Record<string, unknown> = {}) {
  return {
    id: 'i-1',
    building_id: BUILDING_ID,
    seller_id: 'someone-else',
    title: 'Oak bookshelf',
    description: 'Solid wood',
    category: 'furniture',
    condition: 'good',
    price: '45.00',
    currency: 'EUR',
    is_free: false,
    is_negotiable: true,
    is_trade_accepted: false,
    photo_urls: null,
    location: null,
    pickup_details: null,
    view_count: 12,
    inquiry_count: 1,
    status: 'active',
    sold_to: null,
    sold_at: null,
    expires_at: null,
    created_at: '2026-11-01T10:00:00Z',
    updated_at: '2026-11-01T10:00:00Z',
    ...overrides,
  };
}

export const unitsRoute: MockRoute = {
  method: 'GET',
  path: '/api/v1/users/me/units',
  reply: { body: myUnitsFixture },
};
