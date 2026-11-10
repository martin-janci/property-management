/// <reference types="vitest/globals" />
/**
 * FeedPage — wired to the real Community client over a mocked `fetch`.
 */

import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  BUILDING_ID,
  ME,
  mockApi,
  rawGroupWithMembership,
  rawPost,
  renderPage,
  unitsRoute,
} from '../test/communityTestUtils';
import { FeedPage } from './FeedPage';

const h = vi.hoisted(() => ({
  showToast: vi.fn(),
  user: { id: 'user-me', role: 'owner' } as { id: string; role?: string },
}));

vi.mock('../../../contexts', () => ({ useAuth: () => ({ user: h.user }) }));
vi.mock('../../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../../components')>()),
  useToast: () => ({ showToast: h.showToast }),
}));

const groupsPath = `/api/v1/community/buildings/${BUILDING_ID}/groups`;

beforeEach(() => {
  h.showToast.mockReset();
  h.user = { id: ME, role: 'owner' };
});
afterEach(() => vi.unstubAllGlobals());

describe('FeedPage', () => {
  it('shows posts from the groups the user belongs to, and only fetches those', async () => {
    const api = mockApi([
      unitsRoute,
      {
        method: 'GET',
        path: groupsPath,
        reply: {
          body: [
            rawGroupWithMembership({ id: 'g-1', name: 'Gardeners' }, { is_member: true }),
            rawGroupWithMembership({ id: 'g-2', name: 'Chess' }),
          ],
        },
      },
      { method: 'GET', path: '/api/v1/community/groups/g-1/posts', reply: { body: [rawPost()] } },
    ]);

    renderPage(<FeedPage />, '/community', '/community');

    expect(await screen.findByText('Tomatoes are ripe!')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Gardeners' })).toBeInTheDocument();
    expect(api.find('GET', '/api/v1/community/groups/g-1/posts')[0].search).toBe('?limit=20');
    expect(api.find('GET', '/api/v1/community/groups/g-2/posts')).toHaveLength(0);
  });

  it('publishes a post to the chosen group and likes a post', async () => {
    const user = userEvent.setup();
    const api = mockApi([
      unitsRoute,
      {
        method: 'GET',
        path: groupsPath,
        reply: { body: [rawGroupWithMembership({ id: 'g-1' }, { is_member: true })] },
      },
      { method: 'GET', path: '/api/v1/community/groups/g-1/posts', reply: { body: [rawPost()] } },
      {
        method: 'POST',
        path: '/api/v1/community/groups/g-1/posts',
        reply: { status: 201, body: rawPost({ id: 'p-2', content: 'Hello neighbours' }) },
      },
      { method: 'POST', path: '/api/v1/community/posts/p-1/reactions', reply: { status: 200 } },
    ]);

    renderPage(<FeedPage />, '/community', '/community');
    await screen.findByText('Tomatoes are ripe!');

    await user.type(screen.getByLabelText("What's on your mind?"), 'Hello neighbours');
    await user.click(screen.getByRole('button', { name: 'Post' }));

    await waitFor(() =>
      expect(api.find('POST', '/api/v1/community/groups/g-1/posts')[0]?.body).toEqual({
        content: 'Hello neighbours',
        post_type: 'text',
      })
    );
    await waitFor(() =>
      expect(h.showToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }))
    );

    await user.click(screen.getByRole('button', { name: 'Like' }));
    await waitFor(() =>
      expect(api.find('POST', '/api/v1/community/posts/p-1/reactions')[0]?.body).toEqual({
        reaction_type: 'like',
      })
    );
  });

  it('invites the user to join a group when they belong to none', async () => {
    mockApi([
      unitsRoute,
      { method: 'GET', path: groupsPath, reply: { body: [rawGroupWithMembership()] } },
    ]);
    renderPage(<FeedPage />, '/community', '/community');
    expect(await screen.findByText('Your feed is empty')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Browse groups' }));
    expect(screen.getByTestId('location')).toHaveTextContent(
      `/community/groups?building=${BUILDING_ID}`
    );
  });

  it('shows an error with retry when the groups request fails', async () => {
    mockApi([
      unitsRoute,
      { method: 'GET', path: groupsPath, reply: { status: 500, body: { code: 'DATABASE_ERROR' } } },
    ]);
    renderPage(<FeedPage />, '/community', '/community');
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load the data');
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });

  it('explains the missing building when the user has none', async () => {
    mockApi([{ method: 'GET', path: '/api/v1/users/me/units', reply: { body: [] } }]);
    renderPage(<FeedPage />, '/community', '/community');
    expect(await screen.findByText('No building available')).toBeInTheDocument();
  });
});
