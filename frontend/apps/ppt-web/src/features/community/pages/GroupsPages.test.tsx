/// <reference types="vitest/globals" />
/**
 * Groups list / create / detail — wired to the real Community client over a
 * mocked `fetch`, asserting the exact endpoints each action hits.
 */

import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  BUILDING_ID,
  ME,
  mockApi,
  rawGroup,
  rawGroupWithMembership,
  rawPost,
  renderPage,
  unitsRoute,
} from '../test/communityTestUtils';
import { CreateGroupPage } from './CreateGroupPage';
import { GroupDetailPage } from './GroupDetailPage';
import { GroupsPage } from './GroupsPage';

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

describe('GroupsPage', () => {
  const list = [
    rawGroupWithMembership(
      { id: 'g-1', name: 'Gardeners' },
      { is_member: true, member_role: 'member' }
    ),
    rawGroupWithMembership({
      id: 'g-2',
      name: 'Chess Club',
      group_type: 'floor',
      description: 'Weekly games',
    }),
  ];

  it('lists the building groups and joins / leaves via the real endpoints', async () => {
    const user = userEvent.setup();
    const api = mockApi([
      unitsRoute,
      { method: 'GET', path: groupsPath, reply: { body: list } },
      { method: 'POST', path: '/api/v1/community/groups/g-2/join', reply: { status: 200 } },
      { method: 'POST', path: '/api/v1/community/groups/g-1/leave', reply: { status: 200 } },
    ]);

    renderPage(<GroupsPage />, `/community/groups?building=${BUILDING_ID}`, '/community/groups');

    const chess = (await screen.findByText('Chess Club')).closest('article') as HTMLElement;
    expect(within(chess).getByText(/Floor/)).toBeInTheDocument();
    await user.click(within(chess).getByRole('button', { name: 'Join' }));
    await waitFor(() =>
      expect(api.find('POST', '/api/v1/community/groups/g-2/join')).toHaveLength(1)
    );

    const gardeners = screen.getByText('Gardeners').closest('article') as HTMLElement;
    await user.click(within(gardeners).getByRole('button', { name: 'Leave' }));
    await waitFor(() =>
      expect(api.find('POST', '/api/v1/community/groups/g-1/leave')).toHaveLength(1)
    );
    await waitFor(() => expect(h.showToast).toHaveBeenCalledTimes(2));
  });

  it('filters by search text and by membership', async () => {
    const user = userEvent.setup();
    mockApi([unitsRoute, { method: 'GET', path: groupsPath, reply: { body: list } }]);
    renderPage(<GroupsPage />, '/community/groups', '/community/groups');
    await screen.findByText('Chess Club');

    await user.type(screen.getByRole('searchbox'), 'weekly');
    expect(screen.queryByText('Gardeners')).not.toBeInTheDocument();
    expect(screen.getByText('Chess Club')).toBeInTheDocument();

    await user.clear(screen.getByRole('searchbox'));
    await user.click(screen.getByLabelText('My Groups Only'));
    expect(screen.getByText('Gardeners')).toBeInTheDocument();
    expect(screen.queryByText('Chess Club')).not.toBeInTheDocument();
  });

  it('navigates to create with the building preserved', async () => {
    mockApi([unitsRoute, { method: 'GET', path: groupsPath, reply: { body: [] } }]);
    renderPage(<GroupsPage />, '/community/groups', '/community/groups');
    expect(await screen.findByText('No groups found')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Create Group' }));
    expect(screen.getByTestId('location')).toHaveTextContent(
      `/community/groups/new?building=${BUILDING_ID}`
    );
  });
});

describe('CreateGroupPage', () => {
  it('creates the group in the selected building and opens it', async () => {
    const user = userEvent.setup();
    const api = mockApi([
      unitsRoute,
      {
        method: 'POST',
        path: groupsPath,
        reply: { status: 201, body: rawGroup({ id: 'g-new', name: 'Runners' }) },
      },
    ]);
    renderPage(<CreateGroupPage />, '/community/groups/new', '/community/groups/new');

    await user.type(await screen.findByLabelText(/Group Name/), 'Runners');
    await user.type(screen.getByLabelText('Description'), 'Morning runs');
    await user.selectOptions(screen.getByLabelText('Group type'), 'building');
    await user.click(screen.getByLabelText(/Private/));
    await user.click(screen.getByRole('button', { name: 'Create Group' }));

    await waitFor(() =>
      expect(api.find('POST', groupsPath)[0]?.body).toEqual({
        name: 'Runners',
        description: 'Morning runs',
        group_type: 'building',
        visibility: 'private',
      })
    );
    await waitFor(() =>
      expect(screen.getByTestId('location')).toHaveTextContent(
        `/community/groups/g-new?building=${BUILDING_ID}`
      )
    );
    expect(h.showToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });

  it('shows the server error and stays on the form when creation fails', async () => {
    const user = userEvent.setup();
    mockApi([
      unitsRoute,
      {
        method: 'POST',
        path: groupsPath,
        reply: {
          status: 403,
          body: { code: 'BUILDING_ACCESS_DENIED', message: 'Caller is not a member' },
        },
      },
    ]);
    renderPage(<CreateGroupPage />, '/community/groups/new', '/community/groups/new');

    await user.type(await screen.findByLabelText(/Group Name/), 'Runners');
    await user.click(screen.getByRole('button', { name: 'Create Group' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Caller is not a member');
    expect(h.showToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'error', message: 'Caller is not a member' })
    );
    expect(screen.getByTestId('location')).toHaveTextContent('/community/groups/new');
    expect(screen.getByLabelText(/Group Name/)).toHaveValue('Runners');
  });

  it('validates the name before calling the API', async () => {
    const user = userEvent.setup();
    const api = mockApi([unitsRoute]);
    renderPage(<CreateGroupPage />, '/community/groups/new', '/community/groups/new');
    await user.click(await screen.findByRole('button', { name: 'Create Group' }));
    expect(screen.getByText('Group name is required')).toBeInTheDocument();
    expect(api.find('POST', groupsPath)).toHaveLength(0);
  });
});

describe('GroupDetailPage', () => {
  const route = '/community/groups/:groupId';

  it('loads the group, membership and posts; owners can post and delete', async () => {
    const user = userEvent.setup();
    const api = mockApi([
      {
        method: 'GET',
        path: '/api/v1/community/groups/g-1',
        reply: { body: rawGroup({ rules: 'Be kind' }) },
      },
      {
        method: 'GET',
        path: groupsPath,
        reply: { body: [rawGroupWithMembership({}, { is_member: true, member_role: 'owner' })] },
      },
      { method: 'GET', path: '/api/v1/community/groups/g-1/posts', reply: { body: [rawPost()] } },
      {
        method: 'POST',
        path: '/api/v1/community/groups/g-1/posts',
        reply: { status: 201, body: rawPost({ id: 'p-9' }) },
      },
      { method: 'DELETE', path: '/api/v1/community/groups/g-1', reply: { status: 204 } },
    ]);

    renderPage(<GroupDetailPage />, '/community/groups/g-1', route);

    expect(await screen.findByRole('heading', { name: 'Gardeners' })).toBeInTheDocument();
    expect(screen.getByText('Be kind')).toBeInTheDocument();
    expect(await screen.findByText('Tomatoes are ripe!')).toBeInTheDocument();
    expect(api.find('GET', '/api/v1/community/groups/g-1/posts')[0].search).toBe('?limit=50');
    // Owners cannot leave; members/invite/edit controls do not exist.
    expect(screen.queryByRole('button', { name: 'Leave Group' })).not.toBeInTheDocument();
    expect(screen.queryByText('Invite Members')).not.toBeInTheDocument();

    await user.type(
      await screen.findByLabelText("What's on your mind?"),
      'Seedlings swap Saturday'
    );
    await user.click(screen.getByRole('button', { name: 'Post' }));
    await waitFor(() =>
      expect(api.find('POST', '/api/v1/community/groups/g-1/posts')[0]?.body).toEqual({
        content: 'Seedlings swap Saturday',
        post_type: 'text',
      })
    );

    await user.click(screen.getByRole('button', { name: 'Delete Group' }));
    const dialog = await screen.findByRole('alertdialog');
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }));
    await waitFor(() => expect(api.find('DELETE', '/api/v1/community/groups/g-1')).toHaveLength(1));
    await waitFor(() =>
      expect(screen.getByTestId('location')).toHaveTextContent(
        `/community/groups?building=${BUILDING_ID}`
      )
    );
  });

  it('lets a non-member join and hides the composer and delete', async () => {
    const user = userEvent.setup();
    const api = mockApi([
      { method: 'GET', path: '/api/v1/community/groups/g-1', reply: { body: rawGroup() } },
      { method: 'GET', path: groupsPath, reply: { body: [rawGroupWithMembership()] } },
      { method: 'GET', path: '/api/v1/community/groups/g-1/posts', reply: { body: [] } },
      { method: 'POST', path: '/api/v1/community/groups/g-1/join', reply: { status: 200 } },
    ]);
    renderPage(<GroupDetailPage />, '/community/groups/g-1', route);

    await user.click(await screen.findByRole('button', { name: 'Join Group' }));
    await waitFor(() =>
      expect(api.find('POST', '/api/v1/community/groups/g-1/join')).toHaveLength(1)
    );
    expect(screen.getByText('Join this group to post.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Delete Group' })).not.toBeInTheDocument();
    expect(screen.getByText('No posts yet')).toBeInTheDocument();
  });

  it('managers may delete groups they do not own', async () => {
    h.user = { id: ME, role: 'manager' };
    mockApi([
      { method: 'GET', path: '/api/v1/community/groups/g-1', reply: { body: rawGroup() } },
      { method: 'GET', path: groupsPath, reply: { body: [rawGroupWithMembership()] } },
      { method: 'GET', path: '/api/v1/community/groups/g-1/posts', reply: { body: [] } },
    ]);
    renderPage(<GroupDetailPage />, '/community/groups/g-1', route);
    expect(await screen.findByRole('button', { name: 'Delete Group' })).toBeInTheDocument();
  });

  it('renders not-found when the group does not exist', async () => {
    mockApi([
      {
        method: 'GET',
        path: '/api/v1/community/groups/g-1',
        reply: { status: 404, body: { code: 'NOT_FOUND', message: 'Group not found' } },
      },
    ]);
    renderPage(<GroupDetailPage />, '/community/groups/g-1', route);
    expect(await screen.findByText('Group not found')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Back' })).toBeInTheDocument();
  });
});
