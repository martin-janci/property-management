/// <reference types="vitest/globals" />
/**
 * Building context: residents get their unit buildings, managers also get the
 * organization's buildings, and `?building=` selects (and persists) the
 * active one.
 */

import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { GroupsPage } from '../pages/GroupsPage';
import { BUILDING_ID, ME, mockApi, renderPage, unitsRoute } from '../test/communityTestUtils';

const h = vi.hoisted(() => ({
  user: { id: 'user-me', role: 'owner' } as { id: string; role?: string },
}));

vi.mock('../../../contexts', () => ({ useAuth: () => ({ user: h.user }) }));
vi.mock('../../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../../components')>()),
  useToast: () => ({ showToast: vi.fn() }),
}));

const orgBuildingsRoute = {
  method: 'GET',
  path: '/api/v1/buildings',
  reply: {
    body: {
      items: [
        {
          id: BUILDING_ID,
          name: 'Sunny House',
          address: { street: 'Main 1', city: 'BA', postalCode: '1' },
        },
        {
          id: 'b-2',
          name: 'River Tower',
          address: { street: 'River 2', city: 'BA', postalCode: '1' },
        },
      ],
      total: 2,
      page: 1,
      pageSize: 100,
    },
  },
};

const groupsRoute = {
  method: 'GET',
  path: /^\/api\/v1\/community\/buildings\/[^/]+\/groups$/,
  reply: { body: [] },
};

beforeEach(() => {
  h.user = { id: ME, role: 'owner' };
});
afterEach(() => vi.unstubAllGlobals());

describe('useCommunityBuilding', () => {
  it('residents use their unit building and never call the org building list', async () => {
    const api = mockApi([unitsRoute, orgBuildingsRoute, groupsRoute]);
    renderPage(<GroupsPage />, '/community/groups', '/community/groups');
    await screen.findByText('No groups found');
    expect(api.find('GET', `/api/v1/community/buildings/${BUILDING_ID}/groups`)).toHaveLength(1);
    expect(api.find('GET', '/api/v1/buildings')).toHaveLength(0);
    // A single building needs no selector.
    expect(screen.queryByLabelText('Building')).not.toBeInTheDocument();
  });

  it('managers pick among org buildings; ?building= selects and the choice is persisted', async () => {
    h.user = { id: ME, role: 'manager' };
    const user = userEvent.setup();
    const api = mockApi([
      { method: 'GET', path: '/api/v1/users/me/units', reply: { body: [] } },
      orgBuildingsRoute,
      groupsRoute,
    ]);
    renderPage(<GroupsPage />, '/community/groups?building=b-2', '/community/groups');

    const select = await screen.findByLabelText('Building');
    expect(select).toHaveValue('b-2');
    await waitFor(() =>
      expect(api.find('GET', '/api/v1/community/buildings/b-2/groups')).toHaveLength(1)
    );
    expect(api.find('GET', '/api/v1/buildings')[0].search).toBe('?pageSize=100');

    await user.selectOptions(select, BUILDING_ID);
    await waitFor(() =>
      expect(api.find('GET', `/api/v1/community/buildings/${BUILDING_ID}/groups`)).toHaveLength(1)
    );
    expect(screen.getByTestId('location')).toHaveTextContent(`?building=${BUILDING_ID}`);
    // Tabs carry the selection.
    expect(screen.getByRole('link', { name: 'Events' })).toHaveAttribute(
      'href',
      `/community/events?building=${BUILDING_ID}`
    );
  });
});
