/// <reference types="vitest/globals" />
/**
 * ActionQueue navigation + availability (nav-fixes).
 *
 *  - Without an action-queue backend (prod / non-mock modes) the widget
 *    renders nothing instead of a false "all caught up".
 *  - "View" navigates to real routes (`/voting/:id`, not `/votes/:id`).
 *  - Entities with no detail route (budget approvals, person-months) render
 *    without a View button instead of linking to a 404.
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import type { ActionItem } from '../hooks/useActionQueue';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: unknown) =>
      typeof opts === 'string' ? opts : ((opts as { defaultValue?: string })?.defaultValue ?? key),
  }),
}));

const state = vi.hoisted(() => ({ enabled: true, items: [] as unknown[] }));

vi.mock('../hooks/useActionQueue', () => ({
  isActionQueueAvailable: () => state.enabled,
  useActionQueue: () => ({
    items: state.items,
    total: state.items.length,
    stats: { total: state.items.length, urgent: 0, high: 0, medium: 0, low: 0 },
    isLoading: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
    dismissItem: vi.fn(),
    handleAction: vi.fn(),
    isExecuting: false,
  }),
}));

import { ActionQueue, actionItemRoute } from './ActionQueue';

function item(overrides: Partial<ActionItem>): ActionItem {
  return {
    id: 'i-1',
    type: 'vote_active',
    title: 'Garden vote',
    description: 'desc',
    priority: 'medium',
    createdAt: '2026-11-01T00:00:00Z',
    entityId: 'v-1',
    entityType: 'vote',
    actions: [{ id: 'view', label: 'View', variant: 'primary', action: 'view' }],
    ...overrides,
  };
}

function Where() {
  return <div data-testid="where">{useLocation().pathname}</div>;
}

function renderQueue() {
  return render(
    <MemoryRouter initialEntries={['/dashboard']}>
      <Routes>
        <Route path="/dashboard" element={<ActionQueue userRole="manager" />} />
        <Route path="*" element={<Where />} />
      </Routes>
    </MemoryRouter>
  );
}

beforeEach(() => {
  state.enabled = true;
  state.items = [];
});

describe('actionItemRoute', () => {
  it.each([
    ['fault', '/faults/x'],
    ['vote', '/voting/x'],
    ['message', '/messages/x'],
    ['announcement', '/announcements/x'],
    ['meter_reading', '/meters'],
  ])('%s → %s', (entityType, route) => {
    expect(actionItemRoute({ entityType, entityId: 'x' })).toBe(route);
  });

  it.each(['budget_approval', 'person_months'])('%s has no route', (entityType) => {
    expect(actionItemRoute({ entityType, entityId: 'x' })).toBeUndefined();
  });

  it('booking → its building pending-bookings page', () => {
    expect(
      actionItemRoute({ entityType: 'booking', entityId: 'x', metadata: { buildingId: 'b-1' } })
    ).toBe('/buildings/b-1/facilities/pending');
    expect(actionItemRoute({ entityType: 'booking', entityId: 'x' })).toBeUndefined();
  });
});

describe('ActionQueue', () => {
  it('renders nothing when no action-queue backend is available', () => {
    state.enabled = false;
    const { container } = renderQueue();
    expect(container).toBeEmptyDOMElement();
  });

  it('View on a vote navigates to /voting/:id', async () => {
    state.items = [item({})];
    renderQueue();
    await userEvent.click(screen.getByRole('button', { name: 'View' }));
    expect(screen.getByTestId('where')).toHaveTextContent('/voting/v-1');
  });

  it('budget approvals render without a View button', () => {
    state.items = [
      item({
        id: 'a-1',
        type: 'approval_pending',
        title: 'HVAC budget',
        entityType: 'budget_approval',
        actions: [
          { id: 'view', label: 'View', variant: 'secondary', action: 'view' },
          { id: 'approve', label: 'Approve', variant: 'primary', action: 'approve' },
        ],
      }),
    ];
    renderQueue();
    expect(screen.getByText('HVAC budget')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'View' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Approve' })).toBeInTheDocument();
  });
});
