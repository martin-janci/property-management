/// <reference types="vitest/globals" />
/**
 * Events list / RSVP / create — wired to the real Community client over a
 * mocked `fetch`.
 */

import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  BUILDING_ID,
  ME,
  mockApi,
  rawEvent,
  renderPage,
  unitsRoute,
} from '../test/communityTestUtils';
import { CreateEventPage } from './CreateEventPage';
import { EventsPage } from './EventsPage';

const h = vi.hoisted(() => ({
  showToast: vi.fn(),
  user: { id: 'user-me', role: 'owner' } as { id: string; role?: string },
}));

vi.mock('../../../contexts', () => ({ useAuth: () => ({ user: h.user }) }));
vi.mock('../../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../../components')>()),
  useToast: () => ({ showToast: h.showToast }),
}));

const eventsPath = `/api/v1/community/buildings/${BUILDING_ID}/events`;

beforeEach(() => {
  h.showToast.mockReset();
  h.user = { id: ME, role: 'owner' };
});
afterEach(() => vi.unstubAllGlobals());

describe('EventsPage', () => {
  it('lists upcoming events and records an RSVP', async () => {
    const user = userEvent.setup();
    const api = mockApi([
      unitsRoute,
      {
        method: 'GET',
        path: eventsPath,
        reply: { body: [rawEvent(), rawEvent({ id: 'e-2', title: 'Yoga', event_type: 'sports' })] },
      },
      {
        method: 'POST',
        path: '/api/v1/community/events/e-1/rsvp',
        reply: (call) => ({
          body: {
            id: 'r-1',
            event_id: 'e-1',
            user_id: ME,
            status: (call.body as { status: string }).status,
            guests: 0,
            note: null,
            reminder_sent: false,
            created_at: 't',
            updated_at: 't',
          },
        }),
      },
    ]);

    renderPage(<EventsPage />, '/community/events', '/community/events');

    const bbq = (await screen.findByText('Courtyard BBQ')).closest('article') as HTMLElement;
    expect(api.find('GET', eventsPath)[0].search).toBe('?limit=50');
    expect(within(bbq).getByText(/7 going/)).toBeInTheDocument();
    expect(within(bbq).getByText(/Max 30/)).toBeInTheDocument();

    await user.click(within(bbq).getByRole('button', { name: 'Going' }));
    await waitFor(() =>
      expect(api.find('POST', '/api/v1/community/events/e-1/rsvp')[0]?.body).toEqual({
        status: 'going',
      })
    );
    expect(await within(bbq).findByText("You're going")).toBeInTheDocument();
    expect(within(bbq).getByRole('button', { name: 'Going' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );

    await user.selectOptions(screen.getByLabelText('Event type'), 'sports');
    expect(screen.queryByText('Courtyard BBQ')).not.toBeInTheDocument();
    expect(screen.getByText('Yoga')).toBeInTheDocument();
  });

  it('surfaces an RSVP failure as an error toast', async () => {
    const user = userEvent.setup();
    mockApi([
      unitsRoute,
      { method: 'GET', path: eventsPath, reply: { body: [rawEvent()] } },
      {
        method: 'POST',
        path: '/api/v1/community/events/e-1/rsvp',
        reply: {
          status: 500,
          body: { code: 'DATABASE_ERROR', message: 'Failed to RSVP to event' },
        },
      },
    ]);
    renderPage(<EventsPage />, '/community/events', '/community/events');
    await user.click(await screen.findByRole('button', { name: 'Maybe' }));
    await waitFor(() =>
      expect(h.showToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', message: 'Failed to RSVP to event' })
      )
    );
  });

  it('shows the empty state when nothing is scheduled', async () => {
    mockApi([unitsRoute, { method: 'GET', path: eventsPath, reply: { body: [] } }]);
    renderPage(<EventsPage />, '/community/events', '/community/events');
    expect(await screen.findByText('No events found')).toBeInTheDocument();
  });
});

describe('CreateEventPage', () => {
  it('creates an event with ISO start/end times and returns to the list', async () => {
    const user = userEvent.setup();
    const api = mockApi([
      unitsRoute,
      { method: 'POST', path: eventsPath, reply: { status: 201, body: rawEvent() } },
    ]);
    renderPage(<CreateEventPage />, '/community/events/new', '/community/events/new');

    await user.type(await screen.findByLabelText(/Event Title/), 'Courtyard BBQ');
    await user.selectOptions(screen.getByLabelText('Event type'), 'meeting');
    await user.type(screen.getByLabelText('Location'), 'Courtyard');
    await user.type(screen.getByLabelText(/Start/), '2026-12-01T16:00');
    await user.type(screen.getByLabelText(/End/), '2026-12-01T19:00');
    await user.type(screen.getByLabelText(/Maximum Attendees/), '30');
    await user.type(screen.getByLabelText(/Cost per person/), '5');
    await user.click(screen.getByRole('button', { name: 'Create Event' }));

    await waitFor(() => expect(api.find('POST', eventsPath)).toHaveLength(1));
    expect(api.find('POST', eventsPath)[0].body).toEqual({
      title: 'Courtyard BBQ',
      event_type: 'meeting',
      location: 'Courtyard',
      start_time: new Date('2026-12-01T16:00').toISOString(),
      end_time: new Date('2026-12-01T19:00').toISOString(),
      max_attendees: 30,
      cost_per_person: '5.00',
    });
    await waitFor(() =>
      expect(screen.getByTestId('location')).toHaveTextContent(
        `/community/events?building=${BUILDING_ID}`
      )
    );
  });

  it('rejects an end before the start without calling the API', async () => {
    const user = userEvent.setup();
    const api = mockApi([unitsRoute]);
    renderPage(<CreateEventPage />, '/community/events/new', '/community/events/new');
    await user.type(await screen.findByLabelText(/Event Title/), 'Late');
    await user.type(screen.getByLabelText(/Start/), '2026-12-01T19:00');
    await user.type(screen.getByLabelText(/End/), '2026-12-01T16:00');
    await user.click(screen.getByRole('button', { name: 'Create Event' }));
    expect(screen.getByText('End date must be after start date')).toBeInTheDocument();
    expect(api.find('POST', eventsPath)).toHaveLength(0);
  });
});
