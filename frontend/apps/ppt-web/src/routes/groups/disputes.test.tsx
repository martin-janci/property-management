/// <reference types="vitest/globals" />
/**
 * DisputeDetailRoute wiring tests (gap-80-3).
 *
 * The /disputes/:disputeId route wrapper wires the real DisputeDetailPage to
 * the dispute detail / activities / evidence / resolutions / actions queries
 * and their mutations, resolving display names from the party roster. These
 * tests pin that wiring so it can't silently regress back to a stub:
 *
 *  (a) route renders DisputeDetailPage with the API→UI-mapped props
 *  (b) every page handler calls its backend-backed mutation and toasts
 *  (c) actions with no backend route get no handler (no affordance)
 *  (d) load failure renders the error page and surfaces an error toast
 */
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { disputeRoutes } from './disputes';

// ── prop capture for the (mocked) lazy pages ──
const captured = vi.hoisted(() => ({
  detailProps: undefined as Record<string, unknown> | undefined,
}));

vi.mock('../lazyRoutes', () => ({
  DisputesPage: () => <div>disputes list</div>,
  DisputeDetailPage: (props: Record<string, unknown>) => {
    captured.detailProps = props;
    return <div data-testid="dispute-detail-page" />;
  },
  MediationWorkspacePage: () => <div>mediation workspace</div>,
}));

// ── api-client hooks ──
const mocks = vi.hoisted(() => ({
  useDispute: vi.fn(),
  useDisputeTimeline: vi.fn(),
  useDisputeEvidence: vi.fn(),
  useDisputeResolutions: vi.fn(),
  useDisputeActionItems: vi.fn(),
  mutateAsync: vi.fn(),
  // One mutateAsync spy per mutation hook, keyed by hook name.
  mutations: {} as Record<string, ReturnType<typeof vi.fn>>,
}));

function mutationHook(name: string) {
  return () => {
    mocks.mutations[name] ??= vi.fn().mockResolvedValue({});
    return { mutateAsync: mocks.mutations[name], isPending: false };
  };
}

vi.mock('@ppt/api-client', () => ({
  useDispute: (id: string) => mocks.useDispute(id),
  useDisputeTimeline: (id: string) => mocks.useDisputeTimeline(id),
  useDisputeEvidence: (id: string) => mocks.useDisputeEvidence(id),
  useDisputeResolutions: (id: string) => mocks.useDisputeResolutions(id),
  useDisputeActionItems: (id: string) => mocks.useDisputeActionItems(id),
  useUpdateDisputeStatus: () => ({ mutateAsync: mocks.mutateAsync, isPending: false }),
  useDeleteEvidence: mutationHook('deleteEvidence'),
  useProposeResolution: mutationHook('proposeResolution'),
  useVoteOnResolution: mutationHook('voteOnResolution'),
  useAcceptResolution: mutationHook('acceptResolution'),
  useImplementResolution: mutationHook('implementResolution'),
  useCreateActionItem: mutationHook('createActionItem'),
  useCompleteActionItem: mutationHook('completeActionItem'),
  useSendActionReminder: mutationHook('sendActionReminder'),
  useEscalateDispute: mutationHook('escalateDispute'),
  useDisputes: () => ({ data: undefined, isLoading: false, error: null }),
  // imported by FileDisputePageRoute (same module graph); never invoked here
  useCreateDispute: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useMyUnits: () => ({ data: [], isLoading: false }),
}));

// ── auth context: manager user with an org ──
vi.mock('../../contexts', () => ({
  useAuth: () => ({
    user: {
      id: 'user-1',
      organizationId: 'org-1',
      role: 'manager',
      email: 'm@example.com',
      firstName: 'Mia',
      lastName: 'Manager',
    },
  }),
}));

// ── toast + auth gate ──
const mockShowToast = vi.fn();
vi.mock('../../components', () => ({
  useToast: () => ({ showToast: mockShowToast }),
  AuthRequiredGate: () => <div>auth required</div>,
  // Route-level auth guard is transparent here — the guard itself is covered by
  // mgmt-routes-auth-guard.route.test.tsx; this suite exercises page behavior.
  ProtectedRoute: ({ children }: { children?: import('react').ReactNode }) => <>{children}</>,
}));

const party = (id: string, userId: string, role: string, userName: string) => ({
  id,
  userId,
  role,
  userName,
  userEmail: `${userId}@example.com`,
  createdAt: '2026-07-01T10:00:00Z',
});

const apiDispute = {
  id: 'd-1',
  organizationId: 'org-1',
  unitId: 'unit-1',
  referenceNumber: 'DSP-2026-00007',
  category: 'noise' as const,
  title: 'Loud music at night',
  description: 'Repeated late-night noise',
  status: 'under_review' as const,
  priority: 'high' as const,
  filedBy: 'user-2',
  createdAt: '2026-07-01T10:00:00Z',
  updatedAt: '2026-07-02T10:00:00Z',
  parties: [
    party('p-1', 'user-2', 'complainant', 'Fiona Filer'),
    party('p-2', 'user-4', 'respondent', 'Rob Respondent'),
    party('p-3', 'user-3', 'mediator', 'Max Mediator'),
  ],
  evidenceCount: 1,
  activityCount: 1,
  sessionCount: 0,
};

const apiTimeline = [
  {
    id: 'ev-1',
    disputeId: 'd-1',
    eventType: 'party_added' as const,
    actorId: 'user-3',
    description: 'Mediator added',
    createdAt: '2026-07-01T11:00:00Z',
  },
];

const apiEvidence = [
  {
    id: 'evd-1',
    disputeId: 'd-1',
    uploadedBy: 'user-2',
    filename: 'k/photo.jpg',
    originalFilename: 'photo.jpg',
    contentType: 'image/jpeg',
    sizeBytes: 1234,
    storageUrl: 'https://s3/photo.jpg',
    description: 'Photo of hallway',
    createdAt: '2026-07-01T12:00:00Z',
  },
];

const apiResolutions = [
  {
    resolution: {
      id: 'r-1',
      disputeId: 'd-1',
      proposedBy: 'user-3',
      resolutionText: 'Quiet hours after 22:00',
      terms: [],
      status: 'proposed' as const,
      proposedAt: '2026-07-03T10:00:00Z',
      createdAt: '2026-07-03T10:00:00Z',
      updatedAt: '2026-07-03T10:00:00Z',
    },
    votes: [
      { id: 'v-1', resolutionId: 'r-1', partyId: 'p-2', accepted: true, votedAt: '2026-07-04' },
    ],
    acceptanceRate: 50,
  },
];

const apiActions = [
  {
    id: 'a-1',
    disputeId: 'd-1',
    assignedTo: 'user-4',
    title: 'Install carpet',
    description: 'Reduce impact noise',
    dueDate: '2026-08-01T23:59:59Z',
    status: 'pending' as const,
    createdAt: '2026-07-05T10:00:00Z',
    updatedAt: '2026-07-05T10:00:00Z',
  },
];

function renderDetailRoute() {
  return render(
    <MemoryRouter initialEntries={['/disputes/d-1']}>
      <Routes>{disputeRoutes()}</Routes>
    </MemoryRouter>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  captured.detailProps = undefined;
  mocks.useDispute.mockReturnValue({ data: apiDispute, isLoading: false, error: null });
  mocks.useDisputeTimeline.mockReturnValue({ data: apiTimeline, isLoading: false });
  mocks.useDisputeEvidence.mockReturnValue({ data: apiEvidence, isLoading: false });
  mocks.useDisputeResolutions.mockReturnValue({ data: apiResolutions, isLoading: false });
  mocks.useDisputeActionItems.mockReturnValue({ data: apiActions, isLoading: false });
  for (const key of Object.keys(mocks.mutations)) delete mocks.mutations[key];
});

function props() {
  return captured.detailProps as Record<string, (...args: unknown[]) => Promise<void>> &
    Record<string, unknown>;
}

describe('DisputeDetailRoute (/disputes/:disputeId)', () => {
  it('renders DisputeDetailPage with API→UI-mapped dispute, parties, timeline, evidence', () => {
    renderDetailRoute();

    expect(screen.getByTestId('dispute-detail-page')).toBeInTheDocument();
    expect(mocks.useDispute).toHaveBeenCalledWith('d-1');
    expect(mocks.useDisputeTimeline).toHaveBeenCalledWith('d-1');
    expect(mocks.useDisputeEvidence).toHaveBeenCalledWith('d-1');

    const p = props();
    expect(p.dispute).toMatchObject({
      id: 'd-1',
      referenceNumber: 'DSP-2026-00007',
      category: 'noise',
      title: 'Loud music at night',
      status: 'under_review',
      priority: 'high',
      filedByName: 'Fiona Filer',
      // No assigned_to on the dispute: falls back to the mediator party.
      assignedTo: 'user-3',
      assignedToName: 'Max Mediator',
    });
    expect(p.parties).toHaveLength(3);
    expect(p.timeline).toEqual([
      expect.objectContaining({
        id: 'ev-1',
        activityType: 'party_added',
        actorName: 'Max Mediator',
      }),
    ]);
    expect(p.evidence).toEqual([
      expect.objectContaining({
        id: 'evd-1',
        uploaderName: 'Fiona Filer',
        originalFilename: 'photo.jpg',
        contentType: 'image/jpeg',
        sizeBytes: 1234,
        storageUrl: 'https://s3/photo.jpg',
      }),
    ]);
    expect(p.resolutions).toEqual([
      expect.objectContaining({
        id: 'r-1',
        proposedByName: 'Max Mediator',
        acceptanceRate: 50,
        votes: [expect.objectContaining({ partyId: 'p-2', partyName: 'Rob Respondent' })],
      }),
    ]);
    expect(p.actionItems).toEqual([
      expect.objectContaining({ id: 'a-1', assignedToName: 'Rob Respondent' }),
    ]);
    expect(p.isManager).toBe(true);
    expect(p.isParty).toBe(false);
    expect(p.isMediator).toBe(false);
    expect(p.isLoading).toBe(false);
    // No backend route for these: the page gets no handler, so no affordance.
    expect(p.onAddEvidence).toBeUndefined();
    expect(p.onCompleteResolutionTerm).toBeUndefined();
  });

  it('onUpdateStatus sends the status as-is (UI and API statuses match) and toasts', async () => {
    mocks.mutateAsync.mockResolvedValue({});
    renderDetailRoute();

    await props().onUpdateStatus('withdrawn', 'settled privately');

    expect(mocks.mutateAsync).toHaveBeenCalledWith({
      disputeId: 'd-1',
      data: { status: 'withdrawn', reason: 'settled privately' },
    });
    expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });

  it('wires resolution, action and evidence handlers to their mutations', async () => {
    renderDetailRoute();
    const p = props();

    await p.onProposeResolution('Split the cost', []);
    expect(mocks.mutations.proposeResolution).toHaveBeenCalledWith({
      resolutionText: 'Split the cost',
      terms: [],
    });
    await p.onVoteResolution('r-1', false, 'too vague');
    expect(mocks.mutations.voteOnResolution).toHaveBeenCalledWith({
      resolutionId: 'r-1',
      data: { accepted: false, comments: 'too vague' },
    });
    await p.onAcceptResolution('r-1');
    expect(mocks.mutations.acceptResolution).toHaveBeenCalledWith('r-1');
    await p.onImplementResolution('r-1');
    expect(mocks.mutations.implementResolution).toHaveBeenCalledWith('r-1');
    const action = { title: 'T', description: 'D', assignedTo: 'user-4', dueDate: '2026-08-01' };
    await p.onCreateAction(action);
    expect(mocks.mutations.createActionItem).toHaveBeenCalledWith(action);
    await p.onCompleteAction('a-1', 'done');
    expect(mocks.mutations.completeActionItem).toHaveBeenCalledWith({
      actionId: 'a-1',
      notes: 'done',
    });
    await p.onSendReminder('a-1');
    expect(mocks.mutations.sendActionReminder).toHaveBeenCalledWith('a-1');
    await p.onDeleteEvidence('evd-1');
    expect(mocks.mutations.deleteEvidence).toHaveBeenCalledWith('evd-1');
  });

  it('escalating an action item records an escalation linked to it', async () => {
    renderDetailRoute();
    await props().onEscalate('a-1');
    expect(mocks.mutations.escalateDispute).toHaveBeenCalledWith({
      disputeId: 'd-1',
      data: expect.objectContaining({ actionItemId: 'a-1', severity: 'formal_notice' }),
    });
  });

  it('flags the caller as party / mediator from the party roster', () => {
    mocks.useDispute.mockReturnValue({
      data: {
        ...apiDispute,
        parties: [...apiDispute.parties, party('p-9', 'user-1', 'witness', 'Mia Manager')],
      },
      isLoading: false,
      error: null,
    });
    renderDetailRoute();
    expect(props().isParty).toBe(true);
    expect(props().isMediator).toBe(false);
  });

  it('onUpdateStatus failure surfaces an error toast', async () => {
    mocks.mutateAsync.mockRejectedValue(new Error('boom'));
    renderDetailRoute();

    await props().onUpdateStatus('resolved');

    expect(mocks.mutateAsync).toHaveBeenCalledWith({
      disputeId: 'd-1',
      data: { status: 'resolved', reason: undefined },
    });
    expect(mockShowToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'error', message: 'boom' })
    );
  });

  it('load failure renders the error page and shows an error toast', async () => {
    mocks.useDispute.mockReturnValue({
      data: undefined,
      isLoading: false,
      error: new Error('not found'),
    });
    renderDetailRoute();

    expect(screen.queryByTestId('dispute-detail-page')).not.toBeInTheDocument();
    expect(document.querySelector('.error-page')).toBeInTheDocument();
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', message: 'not found' })
      )
    );
  });
});
