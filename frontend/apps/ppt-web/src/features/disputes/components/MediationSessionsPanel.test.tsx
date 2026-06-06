/// <reference types="vitest/globals" />
/**
 * MediationSessionsPanel component tests (Story 80.3).
 * Covers upcoming-first ordering, empty state, and manager-only actions.
 */
import type { MediationSession } from '@ppt/api-client';
import { fireEvent, render, screen } from '@testing-library/react';
import { MediationSessionsPanel } from './MediationSessionsPanel';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { count?: number }) => {
      const map: Record<string, string> = {
        'disputes.mediation.sessions.title': 'Sessions',
        'disputes.mediation.sessions.schedule': '+ Schedule',
        'disputes.mediation.sessions.empty': 'No sessions scheduled yet.',
        'disputes.mediation.sessions.reschedule': 'Reschedule',
        'disputes.mediation.sessions.cancel': 'Cancel',
        'disputes.mediation.sessions.joinLink': 'Join meeting',
        'disputes.mediation.sessions.typeInPerson': 'In Person',
        'disputes.mediation.sessions.typeVideoCall': 'Video Call',
        'disputes.mediation.sessions.typePhone': 'Phone',
        'disputes.mediation.sessions.typeWritten': 'Written Exchange',
        'disputes.mediation.sessions.statusScheduled': 'Scheduled',
        'disputes.mediation.sessions.statusInProgress': 'In Progress',
        'disputes.mediation.sessions.statusCompleted': 'Completed',
        'disputes.mediation.sessions.statusCancelled': 'Cancelled',
        'disputes.mediation.sessions.statusRescheduled': 'Rescheduled',
        'common.loading': 'Loading',
      };
      if (key === 'disputes.mediation.sessions.minutesShort') {
        return `${opts?.count ?? 0} min`;
      }
      return map[key] ?? key;
    },
    i18n: { language: 'en' },
  }),
}));

function makeSession(overrides: Partial<MediationSession>): MediationSession {
  return {
    id: 'sess-1',
    dispute_id: 'd-1',
    mediator_id: 'm-1',
    session_type: 'video_call',
    scheduled_at: new Date(Date.now() + 86_400_000).toISOString(),
    duration_minutes: 60,
    location: null,
    meeting_url: null,
    status: 'scheduled',
    notes: null,
    outcome: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    ...overrides,
  };
}

const noop = () => {};

describe('MediationSessionsPanel', () => {
  it('shows the empty state when there are no sessions', () => {
    render(
      <MediationSessionsPanel
        sessions={[]}
        isLoading={false}
        canManage={false}
        onSchedule={noop}
        onReschedule={noop}
        onCancel={noop}
      />
    );
    expect(screen.getByText('No sessions scheduled yet.')).toBeInTheDocument();
  });

  it('orders upcoming sessions before past sessions', () => {
    const past = makeSession({
      id: 'past',
      status: 'completed',
      scheduled_at: new Date(Date.now() - 86_400_000).toISOString(),
      session_type: 'in_person',
    });
    const upcoming = makeSession({
      id: 'upcoming',
      status: 'scheduled',
      scheduled_at: new Date(Date.now() + 86_400_000).toISOString(),
      session_type: 'video_call',
    });
    render(
      <MediationSessionsPanel
        sessions={[past, upcoming]}
        isLoading={false}
        canManage={false}
        onSchedule={noop}
        onReschedule={noop}
        onCancel={noop}
      />
    );
    const items = screen.getAllByRole('listitem');
    // Upcoming (Video Call) must come first, past (In Person) second.
    expect(items[0]).toHaveTextContent('Video Call');
    expect(items[1]).toHaveTextContent('In Person');
  });

  it('hides manager actions when canManage is false', () => {
    render(
      <MediationSessionsPanel
        sessions={[makeSession({})]}
        isLoading={false}
        canManage={false}
        onSchedule={noop}
        onReschedule={noop}
        onCancel={noop}
      />
    );
    expect(screen.queryByText('Reschedule')).not.toBeInTheDocument();
    expect(screen.queryByText('+ Schedule')).not.toBeInTheDocument();
  });

  it('fires reschedule and cancel callbacks for actionable sessions when canManage', () => {
    const onReschedule = vi.fn();
    const onCancel = vi.fn();
    const session = makeSession({ id: 'sess-actionable', status: 'scheduled' });
    render(
      <MediationSessionsPanel
        sessions={[session]}
        isLoading={false}
        canManage
        onSchedule={noop}
        onReschedule={onReschedule}
        onCancel={onCancel}
      />
    );
    fireEvent.click(screen.getByText('Reschedule'));
    fireEvent.click(screen.getByText('Cancel'));
    expect(onReschedule).toHaveBeenCalledWith(session);
    expect(onCancel).toHaveBeenCalledWith(session);
  });

  it('does not show row actions for completed sessions', () => {
    render(
      <MediationSessionsPanel
        sessions={[makeSession({ status: 'completed' })]}
        isLoading={false}
        canManage
        onSchedule={noop}
        onReschedule={noop}
        onCancel={noop}
      />
    );
    expect(screen.queryByText('Reschedule')).not.toBeInTheDocument();
  });
});
