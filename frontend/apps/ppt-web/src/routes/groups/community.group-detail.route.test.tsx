/**
 * Route-wiring regression test for the community group-detail route (Epic 42,
 * Story 42.1).
 *
 * Before the fix, `GroupDetailPageRoute` ignored the API entirely and rendered
 * a hardcoded `mockGroup` named "Sample Group" for every `groupId`, with all
 * actions as no-ops. This test pins the fixed contract of
 * `GroupDetailPageInner`:
 *
 *  - It fetches the real group for the given `groupId` via `useGroup` and
 *    renders THAT group's name — never the fabricated "Sample Group".
 *  - It shows a loading spinner while the group query is pending.
 *  - It renders a not-found state (not a placeholder group) when the group is
 *    missing or the query errors.
 *  - Join / Leave are wired to the real `useJoinGroup` / `useLeaveGroup`
 *    mutations with the route's `groupId`.
 *  - Membership/role (`isMember`, `isOwner`) is derived from the member list,
 *    not hardcoded.
 *
 * The presentational `GroupDetailPage` is stubbed via `../lazyRoutes` so the
 * test isolates the route-wrapper wiring from the (lazy) page component.
 */

/// <reference types="vitest/globals" />
import type { CommunityGroup, GroupMember } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// --- Mocks ----------------------------------------------------------------

let groupResult: { data: CommunityGroup | undefined; isLoading: boolean; error: unknown } = {
  data: undefined,
  isLoading: false,
  error: null,
};
let membersResult: { data: GroupMember[] } = { data: [] };
const joinMutate = vi.fn();
const leaveMutate = vi.fn();
const showToastMock = vi.fn();

vi.mock('@ppt/api-client', () => ({
  useGroup: () => groupResult,
  useGroupMembers: () => membersResult,
  useJoinGroup: () => ({ mutate: joinMutate, isPending: false }),
  useLeaveGroup: () => ({ mutate: leaveMutate, isPending: false }),
}));

const navigateMock = vi.fn();
vi.mock('react-router-dom', () => ({
  useNavigate: () => navigateMock,
  useParams: () => ({ groupId: 'grp-1' }),
  Route: () => null,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, def?: unknown) =>
      typeof def === 'string'
        ? def
        : ((def as { defaultValue?: string } | undefined)?.defaultValue ?? key),
  }),
}));

vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { id: 'user-me' } }),
}));

vi.mock('../../components', () => ({
  Spinner: () => <div data-testid="spinner">loading</div>,
  useToast: () => ({ showToast: showToastMock }),
}));

vi.mock('../../lib/api', () => ({
  getErrorMessage: (err: unknown) => (err instanceof Error ? err.message : String(err)),
}));

// Stub the (lazy) presentational page so we can assert what the wrapper feeds it.
vi.mock('../lazyRoutes', () => ({
  GroupDetailPage: (props: {
    group: CommunityGroup;
    members: GroupMember[];
    isMember?: boolean;
    isOwner?: boolean;
    onJoin: () => void;
    onLeave: () => void;
  }) => (
    <div>
      <h1>{props.group.name}</h1>
      <span data-testid="member-count">{props.members.length}</span>
      <span data-testid="is-member">{String(props.isMember)}</span>
      <span data-testid="is-owner">{String(props.isOwner)}</span>
      <button type="button" onClick={props.onJoin}>
        join
      </button>
      <button type="button" onClick={props.onLeave}>
        leave
      </button>
    </div>
  ),
  FeedPage: () => null,
  GroupsPage: () => null,
  CreateGroupPage: () => null,
  EventsPage: () => null,
  MarketplacePage: () => null,
}));

// Import after the mocks are registered.
import { GroupDetailPageInner } from './community';

function makeGroup(overrides: Partial<CommunityGroup> = {}): CommunityGroup {
  return {
    id: 'grp-1',
    buildingId: 'bld-1',
    name: 'Rooftop Gardeners',
    description: 'A real community group',
    category: 'hobbies',
    visibility: 'public',
    memberCount: 3,
    postCount: 0,
    isOfficial: false,
    createdBy: 'user-owner',
    createdAt: '2026-06-01T00:00:00Z',
    updatedAt: '2026-06-01T00:00:00Z',
    ...overrides,
  };
}

function makeMember(overrides: Partial<GroupMember> = {}): GroupMember {
  return {
    id: 'mem-1',
    groupId: 'grp-1',
    userId: 'user-me',
    userName: 'Me',
    role: 'member',
    status: 'active',
    joinedAt: '2026-06-01T00:00:00Z',
    ...overrides,
  };
}

describe('GroupDetailPageInner — route wiring (Story 42.1)', () => {
  beforeEach(() => {
    joinMutate.mockReset();
    leaveMutate.mockReset();
    showToastMock.mockClear();
    navigateMock.mockClear();
    groupResult = { data: undefined, isLoading: false, error: null };
    membersResult = { data: [] };
  });

  it('renders the real group from the API and never the fabricated "Sample Group"', () => {
    groupResult = { data: makeGroup({ name: 'Rooftop Gardeners' }), isLoading: false, error: null };
    render(<GroupDetailPageInner groupId="grp-1" />);

    expect(screen.getByRole('heading', { name: 'Rooftop Gardeners' })).toBeInTheDocument();
    expect(screen.queryByText('Sample Group')).not.toBeInTheDocument();
  });

  it('shows a loading spinner while the group query is pending', () => {
    groupResult = { data: undefined, isLoading: true, error: null };
    render(<GroupDetailPageInner groupId="grp-1" />);

    expect(screen.getByTestId('spinner')).toBeInTheDocument();
    expect(screen.queryByText('Sample Group')).not.toBeInTheDocument();
  });

  it('renders a not-found state when the group is missing', () => {
    groupResult = { data: undefined, isLoading: false, error: null };
    render(<GroupDetailPageInner groupId="grp-1" />);

    expect(screen.getByText('errors.groupNotFound')).toBeInTheDocument();
  });

  it('renders a not-found state when the group query errors', () => {
    groupResult = { data: undefined, isLoading: false, error: new Error('boom') };
    render(<GroupDetailPageInner groupId="grp-1" />);

    expect(screen.getByText('errors.groupNotFound')).toBeInTheDocument();
  });

  it('wires Join to useJoinGroup().mutate with the route groupId', async () => {
    groupResult = { data: makeGroup(), isLoading: false, error: null };
    membersResult = { data: [] };
    render(<GroupDetailPageInner groupId="grp-1" />);

    await userEvent.click(screen.getByRole('button', { name: 'join' }));
    expect(joinMutate).toHaveBeenCalledTimes(1);
    // First positional arg is the route groupId; the mutation may also receive
    // an options object ({ onError, onSuccess }) for toast wiring.
    expect(joinMutate.mock.calls[0]?.[0]).toBe('grp-1');
  });

  it('wires Leave to useLeaveGroup().mutate with the route groupId', async () => {
    groupResult = { data: makeGroup(), isLoading: false, error: null };
    membersResult = { data: [makeMember()] };
    render(<GroupDetailPageInner groupId="grp-1" />);

    await userEvent.click(screen.getByRole('button', { name: 'leave' }));
    expect(leaveMutate).toHaveBeenCalledTimes(1);
    expect(leaveMutate.mock.calls[0]?.[0]).toBe('grp-1');
  });

  it('derives membership and owner role from the member list', () => {
    groupResult = { data: makeGroup(), isLoading: false, error: null };
    membersResult = { data: [makeMember({ userId: 'user-me', role: 'owner', status: 'active' })] };
    render(<GroupDetailPageInner groupId="grp-1" />);

    expect(screen.getByTestId('is-member')).toHaveTextContent('true');
    expect(screen.getByTestId('is-owner')).toHaveTextContent('true');
    expect(screen.getByTestId('member-count')).toHaveTextContent('1');
  });

  it('treats a non-member viewer as not a member', () => {
    groupResult = { data: makeGroup(), isLoading: false, error: null };
    membersResult = { data: [makeMember({ userId: 'someone-else', role: 'owner' })] };
    render(<GroupDetailPageInner groupId="grp-1" />);

    expect(screen.getByTestId('is-member')).toHaveTextContent('false');
    expect(screen.getByTestId('is-owner')).toHaveTextContent('false');
  });

  // Regression: join/leave used to call `.mutate(groupId)` with no options, so
  // a 403/409/5xx rejection failed silently — the user saw nothing. The route
  // container must now wire `onError`/`onSuccess` and surface a toast.
  it('surfaces an error toast when joining the group fails (403/409/5xx)', async () => {
    groupResult = { data: makeGroup(), isLoading: false, error: null };
    membersResult = { data: [] };
    joinMutate.mockImplementation((_id: string, opts?: { onError?: (e: unknown) => void }) =>
      opts?.onError?.(new Error('You are not allowed to join this group'))
    );
    render(<GroupDetailPageInner groupId="grp-1" />);

    await userEvent.click(screen.getByRole('button', { name: 'join' }));

    expect(showToastMock).toHaveBeenCalledTimes(1);
    expect(showToastMock).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'error',
        message: 'You are not allowed to join this group',
      })
    );
  });

  it('surfaces an error toast when leaving the group fails (403/409/5xx)', async () => {
    groupResult = { data: makeGroup(), isLoading: false, error: null };
    membersResult = { data: [makeMember()] };
    leaveMutate.mockImplementation((_id: string, opts?: { onError?: (e: unknown) => void }) =>
      opts?.onError?.(new Error('Owners cannot leave the group'))
    );
    render(<GroupDetailPageInner groupId="grp-1" />);

    await userEvent.click(screen.getByRole('button', { name: 'leave' }));

    expect(showToastMock).toHaveBeenCalledTimes(1);
    expect(showToastMock).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'error',
        message: 'Owners cannot leave the group',
      })
    );
  });

  it('surfaces a success toast when joining the group succeeds', async () => {
    groupResult = { data: makeGroup(), isLoading: false, error: null };
    membersResult = { data: [] };
    joinMutate.mockImplementation((_id: string, opts?: { onSuccess?: () => void }) =>
      opts?.onSuccess?.()
    );
    render(<GroupDetailPageInner groupId="grp-1" />);

    await userEvent.click(screen.getByRole('button', { name: 'join' }));

    expect(showToastMock).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });
});
