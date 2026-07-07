/// <reference types="vitest/globals" />
/**
 * DisputeDetailRoute wiring tests (gap-80-3).
 *
 * The /disputes/:disputeId route wrapper replaced an inline JSX stub with the
 * real DisputeDetailPage, wiring useDispute / useDisputeTimeline /
 * useDisputeEvidence / useUpdateDisputeStatus from @ppt/api-client and mapping
 * API types → UI types. These tests pin that wiring so it can't silently
 * regress back to a stub:
 *
 *  (a) route renders DisputeDetailPage with the API→UI-mapped dispute,
 *      timeline, and evidence props
 *  (b) onUpdateStatus maps the UI status to the API status, calls the
 *      mutation, and shows a success toast
 *  (c) load failure renders the error page and surfaces an error toast
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
  mutateAsync: vi.fn(),
}));

vi.mock('@ppt/api-client', () => ({
  useDispute: (id: string) => mocks.useDispute(id),
  useDisputeTimeline: (id: string) => mocks.useDisputeTimeline(id),
  useDisputeEvidence: (id: string) => mocks.useDisputeEvidence(id),
  useUpdateDisputeStatus: () => ({ mutateAsync: mocks.mutateAsync, isPending: false }),
  useDisputes: () => ({ data: undefined, isLoading: false, error: null }),
  // imported by FileDisputePageRoute (same module graph); never invoked here
  useCreateDispute: () => ({ mutateAsync: vi.fn(), isPending: false }),
  uploadEvidence: vi.fn(),
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
}));

const apiDispute = {
  id: 'd-1',
  organizationId: 'org-1',
  unitId: 'unit-1',
  type: 'noise' as const,
  subject: 'Loud music at night',
  description: 'Repeated late-night noise',
  status: 'under_review' as const,
  filedBy: 'user-2',
  filerDetails: { id: 'user-2', name: 'Fiona Filer' },
  assignedMediatorId: 'user-3',
  assignedMediator: 'Max Mediator',
  createdAt: '2026-07-01T10:00:00Z',
  updatedAt: '2026-07-02T10:00:00Z',
};

const apiTimeline = [
  {
    id: 'ev-1',
    disputeId: 'd-1',
    eventType: 'mediator_assigned' as const,
    actorId: 'user-3',
    actorName: 'Max Mediator',
    description: 'Mediator assigned',
    createdAt: '2026-07-01T11:00:00Z',
  },
];

const apiEvidence = [
  {
    id: 'evd-1',
    disputeId: 'd-1',
    fileName: 'photo.jpg',
    fileType: 'image/jpeg',
    fileSize: 1234,
    fileUrl: 'https://s3/photo.jpg',
    description: 'Photo of hallway',
    uploadedBy: 'user-2',
    uploadedAt: '2026-07-01T12:00:00Z',
    createdAt: '2026-07-01T12:00:00Z',
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
});

describe('DisputeDetailRoute (/disputes/:disputeId)', () => {
  it('renders DisputeDetailPage with API→UI-mapped dispute, timeline, and evidence', () => {
    renderDetailRoute();

    expect(screen.getByTestId('dispute-detail-page')).toBeInTheDocument();
    expect(mocks.useDispute).toHaveBeenCalledWith('d-1');
    expect(mocks.useDisputeTimeline).toHaveBeenCalledWith('d-1');
    expect(mocks.useDisputeEvidence).toHaveBeenCalledWith('d-1');

    const props = captured.detailProps as Record<string, unknown>;
    expect(props.dispute).toMatchObject({
      id: 'd-1',
      referenceNumber: 'DSP-D-1',
      category: 'noise',
      title: 'Loud music at night',
      status: 'under_review',
      filedByName: 'Fiona Filer',
      assignedToName: 'Max Mediator',
    });
    expect(props.timeline).toEqual([
      expect.objectContaining({
        id: 'ev-1',
        activityType: 'party_added', // mediator_assigned → party_added
        actorName: 'Max Mediator',
      }),
    ]);
    expect(props.evidence).toEqual([
      expect.objectContaining({
        id: 'evd-1',
        filename: 'photo.jpg',
        contentType: 'image/jpeg',
        sizeBytes: 1234,
        storageUrl: 'https://s3/photo.jpg',
      }),
    ]);
    expect(props.isManager).toBe(true);
    expect(props.isLoading).toBe(false);
  });

  it('onUpdateStatus maps UI status to API status, mutates, and toasts success', async () => {
    mocks.mutateAsync.mockResolvedValue({});
    renderDetailRoute();

    const onUpdateStatus = (captured.detailProps as Record<string, unknown>).onUpdateStatus as (
      status: string,
      reason?: string
    ) => Promise<void>;
    await onUpdateStatus('withdrawn', 'settled privately');

    expect(mocks.mutateAsync).toHaveBeenCalledWith({
      disputeId: 'd-1',
      // UI 'withdrawn' has no API equivalent → mapped to 'closed'
      data: { status: 'closed', reason: 'settled privately' },
    });
    expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });

  it('onUpdateStatus failure surfaces an error toast', async () => {
    mocks.mutateAsync.mockRejectedValue(new Error('boom'));
    renderDetailRoute();

    const onUpdateStatus = (captured.detailProps as Record<string, unknown>).onUpdateStatus as (
      status: string
    ) => Promise<void>;
    await onUpdateStatus('resolved');

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
