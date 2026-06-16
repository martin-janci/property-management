/// <reference types="vitest/globals" />
/**
 * MediationWorkspacePage flow + permission tests (test-mediation-resolution-flow).
 *
 * Coverage 80-3: deeper coverage of the dispute lifecycle as seen by the
 * mediation workspace — the open -> mediation -> resolved state transitions
 * (the `isActive` gate around the Resolve tab) and the role-based permission
 * matrix (manager / mediator / party / outsider) that decides who can manage,
 * chat, and record a resolution.
 *
 * These complement MediationWorkspacePage.test.tsx, which only exercises the
 * load-error / loading states. Additive only — no behaviour change.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import { MediationWorkspacePage } from './MediationWorkspacePage';

const mutateAsyncResolve = vi.fn();

vi.mock('@ppt/api-client', () => ({
  useDispute: vi.fn(),
  useMediationSessions: vi.fn(() => ({ data: [], isLoading: false })),
  useResolveDispute: vi.fn(() => ({ isPending: false, mutateAsync: mutateAsyncResolve })),
  useEscalateDispute: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useAssignMediator: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useScheduleSession: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useUpdateSession: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useCancelSession: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
}));

// Identity translator: keys pass through so assertions can target stable keys
// regardless of copy. Interpolation params are ignored — none are asserted here.
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: 'en' },
  }),
}));

// Stub the heavy children so the page renders in isolation and so we can detect
// whether the chat thread mounted (canChat gate) and resolution form mounted.
vi.mock('../components/MediationTimelineView', () => ({
  MediationTimelineView: () => null,
}));
vi.mock('../components/MediationChatThread', () => ({
  MediationChatThread: () => <div data-testid="chat-thread" />,
}));
vi.mock('../components/MediationSessionsPanel', () => ({ MediationSessionsPanel: () => null }));
vi.mock('../components/MediationResolutionForm', () => ({
  MediationResolutionForm: ({ onResolve }: { onResolve: (d: unknown) => void }) => (
    <button type="button" data-testid="resolution-form" onClick={() => onResolve({ stub: true })}>
      resolution-form
    </button>
  ),
}));

import { useDispute } from '@ppt/api-client';

const mockUseDispute = vi.mocked(useDispute);

const FILER_ID = 'user-filer';
const RESPONDENT_ID = 'user-respondent';
const MEDIATOR_ID = 'user-mediator';
const OUTSIDER_ID = 'user-outsider';

type DisputeOverrides = Record<string, unknown>;

/** Build a dispute object with sane defaults for the workspace page. */
function makeDispute(status: string, overrides: DisputeOverrides = {}) {
  return {
    id: 'dsp-0001-abcd',
    subject: 'Leaky tap dispute',
    status,
    filedBy: FILER_ID,
    respondentId: RESPONDENT_ID,
    assignedMediatorId: MEDIATOR_ID,
    filedAt: '2026-01-01T00:00:00.000Z',
    filerDetails: { name: 'Filer Fred', email: 'fred@example.com' },
    respondentDetails: { name: 'Respondent Rita', email: 'rita@example.com' },
    ...overrides,
  };
}

function mockDispute(status: string, overrides: DisputeOverrides = {}) {
  mockUseDispute.mockReturnValue({
    data: makeDispute(status, overrides),
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
});

describe('MediationWorkspacePage — status / state transitions', () => {
  it('renders the in-mediation status label for an active mediation dispute', () => {
    mockDispute('mediation');
    renderPage({ isManager: true });
    expect(screen.getByText('disputes.statusInMediation')).toBeInTheDocument();
  });

  it('shows manage actions (assign / escalate / resolve) while the dispute is active', () => {
    mockDispute('mediation');
    renderPage({ isManager: true });
    expect(
      screen.getByRole('button', { name: 'disputes.mediation.assignMediator' })
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'disputes.mediation.escalate' })).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'disputes.mediation.resolveDispute' })
    ).toBeInTheDocument();
  });

  it('renders the resolution form on the resolve tab for an active dispute (manager)', () => {
    mockDispute('mediation');
    renderPage({ isManager: true });
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabResolution' }));
    expect(screen.getByTestId('resolution-form')).toBeInTheDocument();
  });

  it('hides manage actions once the dispute is resolved', () => {
    mockDispute('resolved');
    renderPage({ isManager: true });
    expect(
      screen.queryByRole('button', { name: 'disputes.mediation.resolveDispute' })
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'disputes.mediation.assignMediator' })
    ).not.toBeInTheDocument();
  });

  it('shows the "no longer active" message instead of the resolution form when resolved', () => {
    mockDispute('resolved');
    renderPage({ isManager: true });
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabResolution' }));
    expect(screen.getByText('disputes.mediation.noLongerActive')).toBeInTheDocument();
    expect(screen.queryByTestId('resolution-form')).not.toBeInTheDocument();
  });

  it('treats a closed dispute as inactive (no resolution form)', () => {
    mockDispute('closed');
    renderPage({ isManager: true });
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabResolution' }));
    expect(screen.getByText('disputes.mediation.noLongerActive')).toBeInTheDocument();
    expect(screen.queryByTestId('resolution-form')).not.toBeInTheDocument();
  });

  it('forwards the resolution payload to the resolve mutation', () => {
    mockDispute('mediation');
    renderPage({ isManager: true });
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabResolution' }));
    fireEvent.click(screen.getByTestId('resolution-form'));
    expect(mutateAsyncResolve).toHaveBeenCalledWith({
      disputeId: 'dsp-0001-abcd',
      data: { stub: true },
    });
  });
});

describe('MediationWorkspacePage — permission matrix', () => {
  it('manager can manage and resolve', () => {
    mockDispute('mediation');
    renderPage({ isManager: true, currentUserId: OUTSIDER_ID });
    expect(
      screen.getByRole('button', { name: 'disputes.mediation.resolveDispute' })
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabResolution' }));
    expect(screen.getByTestId('resolution-form')).toBeInTheDocument();
  });

  it('assigned mediator can manage and resolve even without the manager flag', () => {
    mockDispute('mediation');
    renderPage({ isManager: false, currentUserId: MEDIATOR_ID });
    expect(
      screen.getByRole('button', { name: 'disputes.mediation.resolveDispute' })
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabResolution' }));
    expect(screen.getByTestId('resolution-form')).toBeInTheDocument();
  });

  it('a party (filer) cannot manage or resolve but can use chat', () => {
    mockDispute('mediation');
    renderPage({ isManager: false, currentUserId: FILER_ID });
    // No manage actions.
    expect(
      screen.queryByRole('button', { name: 'disputes.mediation.resolveDispute' })
    ).not.toBeInTheDocument();
    // Resolve tab shows the manager-only gate, not the form.
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabResolution' }));
    expect(screen.getByText('disputes.mediation.onlyManagersCanResolve')).toBeInTheDocument();
    expect(screen.queryByTestId('resolution-form')).not.toBeInTheDocument();
    // But chat is available to a party.
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabDiscussion' }));
    expect(screen.getByTestId('chat-thread')).toBeInTheDocument();
  });

  it('the respondent party can also access chat', () => {
    mockDispute('mediation');
    renderPage({ isManager: false, currentUserId: RESPONDENT_ID });
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabDiscussion' }));
    expect(screen.getByTestId('chat-thread')).toBeInTheDocument();
  });

  it('an outsider (not manager / mediator / party) cannot chat or resolve', () => {
    mockDispute('mediation');
    renderPage({ isManager: false, currentUserId: OUTSIDER_ID });
    // No manage actions.
    expect(
      screen.queryByRole('button', { name: 'disputes.mediation.resolveDispute' })
    ).not.toBeInTheDocument();
    // Chat gated.
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabDiscussion' }));
    expect(screen.queryByTestId('chat-thread')).not.toBeInTheDocument();
    expect(screen.getByText('disputes.mediation.mustBeParty')).toBeInTheDocument();
    // Resolve gated.
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabResolution' }));
    expect(screen.getByText('disputes.mediation.onlyManagersCanResolve')).toBeInTheDocument();
  });

  it('does not offer the resolution form to a mediator once the dispute is resolved', () => {
    mockDispute('resolved');
    renderPage({ isManager: false, currentUserId: MEDIATOR_ID });
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.tabResolution' }));
    // Inactive gate wins over the canManage path.
    expect(screen.getByText('disputes.mediation.noLongerActive')).toBeInTheDocument();
    expect(screen.queryByTestId('resolution-form')).not.toBeInTheDocument();
  });
});

describe('MediationWorkspacePage — header rendering', () => {
  it('renders the dispute reference and subject in the header', () => {
    mockDispute('mediation');
    renderPage({ isManager: true });
    const heading = screen.getByRole('heading', { name: 'disputes.mediation.title' });
    expect(heading).toBeInTheDocument();
    // Reference derived from the dispute id via formatDisputeReference:
    // 'dsp-0001-abcd' -> strip hyphens -> 'dsp0001abcd' -> first 8 -> 'DSP0001A'.
    expect(screen.getByText('DSP-DSP0001A')).toBeInTheDocument();
    expect(screen.getByText('Leaky tap dispute')).toBeInTheDocument();
  });

  it('invokes onBack when the back button is clicked', () => {
    mockDispute('mediation');
    const onBack = vi.fn();
    renderPage({ isManager: true, onBack });
    fireEvent.click(screen.getByRole('button', { name: 'disputes.mediation.backToDispute' }));
    expect(onBack).toHaveBeenCalledTimes(1);
  });
});
