/// <reference types="vitest/globals" />
/**
 * MediationWorkspacePage sessions/submissions threading tests
 * (gap-80-3-sessions-submissions-workflow-not-fully-threaded).
 *
 * Guards that the mediation workspace — the only routed mediation page,
 * reachable from App.tsx via AppRoutes -> disputeRoutes ->
 * /disputes/:disputeId/mediation — actually threads BOTH sub-workflows to the
 * backend:
 *   - sessions: useMediationSessions data reaches MediationSessionsPanel, and
 *     the schedule action opens the session dialog whose confirm calls
 *     useScheduleSession.
 *   - submissions: the Submissions tab mounts MediationSubmissionsPanel wired
 *     with the dispute id and the party's canSubmit permission.
 *
 * Regression intent: a previous standalone MediationPage held these workflows
 * behind prop callbacks that were never wired from any route. This test pins
 * the fact that the live workspace threads them end-to-end so that dead-code
 * variant cannot silently reappear.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import { MediationWorkspacePage } from './MediationWorkspacePage';

const scheduleMutate = vi.fn();
const useMediationSessionsMock = vi.fn(() => ({ data: [], isLoading: false }));

vi.mock('@ppt/api-client', () => ({
  useDispute: vi.fn(),
  useMediationSessions: (...args: unknown[]) => useMediationSessionsMock(...args),
  useResolveDispute: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useEscalateDispute: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useAssignMediator: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useScheduleSession: vi.fn(() => ({ isPending: false, mutateAsync: scheduleMutate })),
  useUpdateSession: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useCancelSession: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: 'en' } }),
}));

vi.mock('../components/MediationTimelineView', () => ({
  MediationTimelineView: () => null,
}));
vi.mock('../components/MediationChatThread', () => ({
  MediationChatThread: () => <div data-testid="chat-thread" />,
}));

// Expose the props the workspace threads into the sessions panel so we can
// assert both the API data and the canManage permission actually arrive, and
// drive the schedule affordance.
vi.mock('../components/MediationSessionsPanel', () => ({
  MediationSessionsPanel: ({
    sessions,
    canManage,
    onSchedule,
  }: {
    sessions: Array<{ id: string }>;
    canManage: boolean;
    onSchedule: () => void;
  }) => (
    <div data-testid="sessions-panel">
      <span data-testid="sessions-count">{sessions.length}</span>
      <span data-testid="sessions-can-manage">{String(canManage)}</span>
      <button type="button" data-testid="sessions-schedule" onClick={onSchedule}>
        schedule
      </button>
    </div>
  ),
}));

// Expose the disputeId + canSubmit the workspace threads into the submissions
// panel.
vi.mock('../components/MediationSubmissionsPanel', () => ({
  MediationSubmissionsPanel: ({
    disputeId,
    canSubmit,
  }: {
    disputeId: string;
    canSubmit: boolean;
  }) => (
    <div data-testid="submissions-panel">
      <span data-testid="submissions-dispute-id">{disputeId}</span>
      <span data-testid="submissions-can-submit">{String(canSubmit)}</span>
    </div>
  ),
}));

// Session dialog stub: fire the schedule confirm so we can assert the workspace
// forwards it to the useScheduleSession mutation.
vi.mock('../components/MediationSessionDialog', () => ({
  MediationSessionDialog: ({ onScheduleConfirm }: { onScheduleConfirm: (d: unknown) => void }) => (
    <button
      type="button"
      data-testid="session-dialog"
      onClick={() => onScheduleConfirm({ session_type: 'video_call', scheduled_at: 'x' })}
    >
      session-dialog
    </button>
  ),
}));

vi.mock('../components/MediationResolutionForm', () => ({
  MediationResolutionForm: () => null,
}));

import { useDispute } from '@ppt/api-client';

const mockUseDispute = vi.mocked(useDispute);

const FILER_ID = 'user-filer';
const MEDIATOR_ID = 'user-mediator';

function mockDispute(overrides: Record<string, unknown> = {}) {
  mockUseDispute.mockReturnValue({
    data: {
      id: 'dsp-0001-abcd',
      subject: 'Leaky tap dispute',
      status: 'mediation',
      filedBy: FILER_ID,
      respondentId: 'user-respondent',
      assignedMediatorId: MEDIATOR_ID,
      filedAt: '2026-01-01T00:00:00.000Z',
      filerDetails: { name: 'Filer Fred', email: 'fred@example.com' },
      respondentDetails: { name: 'Respondent Rita', email: 'rita@example.com' },
      ...overrides,
    },
    isLoading: false,
    isError: false,
    isFetching: false,
    refetch: vi.fn(),
  } as unknown as ReturnType<typeof useDispute>);
}

function renderPage(props: Partial<React.ComponentProps<typeof MediationWorkspacePage>> = {}) {
  return render(
    <MediationWorkspacePage
      disputeId="dsp-0001-abcd"
      organizationId="org-1"
      onBack={vi.fn()}
      onToastSuccess={vi.fn()}
      onToastError={vi.fn()}
      {...props}
    />
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  useMediationSessionsMock.mockReturnValue({ data: [], isLoading: false });
});

describe('MediationWorkspacePage — sessions threading', () => {
  it('threads useMediationSessions data into the sessions panel', () => {
    useMediationSessionsMock.mockReturnValue({
      data: [{ id: 'sess-1' }, { id: 'sess-2' }],
      isLoading: false,
    });
    mockDispute();
    renderPage({ isManager: true });
    expect(screen.getByTestId('sessions-count').textContent).toBe('2');
  });

  it('grants manage rights on the sessions panel to a manager on an active dispute', () => {
    mockDispute();
    renderPage({ isManager: true });
    expect(screen.getByTestId('sessions-can-manage').textContent).toBe('true');
  });

  it('withholds manage rights from a plain party', () => {
    mockDispute();
    renderPage({ isManager: false, currentUserId: FILER_ID });
    expect(screen.getByTestId('sessions-can-manage').textContent).toBe('false');
  });

  it('forwards a scheduled session to the useScheduleSession mutation', () => {
    mockDispute();
    renderPage({ isManager: true });
    // Open the schedule dialog, then confirm it.
    fireEvent.click(screen.getByTestId('sessions-schedule'));
    fireEvent.click(screen.getByTestId('session-dialog'));
    expect(scheduleMutate).toHaveBeenCalledWith({
      session_type: 'video_call',
      scheduled_at: 'x',
    });
  });
});

describe('MediationWorkspacePage — submissions threading', () => {
  it('mounts the submissions panel with the dispute id when the tab is opened', () => {
    mockDispute();
    renderPage({ isManager: true, currentUserId: FILER_ID });
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabSubmissions' }));
    expect(screen.getByTestId('submissions-panel')).toBeInTheDocument();
    expect(screen.getByTestId('submissions-dispute-id').textContent).toBe('dsp-0001-abcd');
  });

  it('lets a party submit (canSubmit=true) but not an outsider', () => {
    mockDispute();
    renderPage({ currentUserId: FILER_ID });
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabSubmissions' }));
    expect(screen.getByTestId('submissions-can-submit').textContent).toBe('true');
  });

  it('marks canSubmit=false for a non-party viewer', () => {
    mockDispute();
    renderPage({ currentUserId: 'user-outsider' });
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabSubmissions' }));
    expect(screen.getByTestId('submissions-can-submit').textContent).toBe('false');
  });
});
