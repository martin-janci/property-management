/**
 * Regression test for the /profile fabricated-identity bug.
 *
 * The page used to render a hard-coded `MOCK_PROFILE` ("Tomáš Novotný" plus
 * fabricated listings / activity / reviews) to EVERY signed-in user, so every
 * visitor saw the same fake identity — a UX/privacy bug. These tests pin that
 * the page now reflects the REAL signed-in user and never prints the old mock
 * identity.
 *
 * Only framework boundaries are mocked: auth-context (so ProtectedRoute renders
 * its children for an authenticated user) and the Header/Footer chrome. `fetch`
 * is stubbed to the real reality-server contracts — here returning an empty
 * portfolio and no realtor profile, i.e. the common "plain signed-in user"
 * case — which must surface the user's own name, never the fixture's.
 */
/// <reference types="vitest/globals" />

import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

const REAL_USER = {
  user_id: 'user-42',
  email: 'real.person@example.com',
  name: 'Real Signed-in Person',
};

vi.mock('@/lib/auth-context', () => ({
  useAuth: () => ({
    isLoading: false,
    isAuthenticated: true,
    user: REAL_USER,
    login: vi.fn(),
    logout: vi.fn(),
    refreshSession: vi.fn(),
  }),
}));

vi.mock('@/components/ui', () => ({
  Header: () => <header />,
  Footer: () => <footer />,
}));

import ProfilePage from './page';

/**
 * Stub every reality-server endpoint the profile page touches. Defaults model a
 * plain signed-in user with no listings and no realtor profile (404), so the
 * page must fall back to the SSO identity rather than any fabricated one.
 */
function stubFetch() {
  const fetchMock = vi.fn((url: string) => {
    if (url.includes('/api/v1/my/listings')) {
      return Promise.resolve({
        ok: true,
        status: 200,
        json: async () => ({ listings: [], total: 0 }),
      });
    }
    if (url.includes('/api/v1/realtors/me/stats')) {
      return Promise.resolve({
        ok: false,
        status: 404,
        json: async () => ({ message: 'Not a realtor' }),
      });
    }
    if (url.includes('/api/v1/realtors/me')) {
      return Promise.resolve({
        ok: false,
        status: 404,
        json: async () => ({ message: 'Not a realtor' }),
      });
    }
    return Promise.resolve({ ok: true, status: 200, json: async () => ({}) });
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

describe('ProfilePage — real identity, not a shared fixture', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('never renders the fabricated MOCK_PROFILE identity', async () => {
    stubFetch();
    render(<ProfilePage />);

    // The old shared fixture identity must not appear for any signed-in user.
    await waitFor(() => {
      expect(screen.queryByText(/Tomáš Novotný/)).not.toBeInTheDocument();
    });
    expect(screen.queryByText(/t\.novotny@email\.sk/)).not.toBeInTheDocument();
    // A fabricated listing title from the old mock must not appear either.
    expect(screen.queryByText(/3-izbový byt Petržalka/)).not.toBeInTheDocument();
  });

  it('shows the real signed-in user name', async () => {
    stubFetch();
    render(<ProfilePage />);

    await waitFor(() => {
      expect(screen.getByText(REAL_USER.name)).toBeInTheDocument();
    });
  });
});
