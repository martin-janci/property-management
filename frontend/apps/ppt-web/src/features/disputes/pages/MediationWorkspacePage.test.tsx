/// <reference types="vitest/globals" />
/**
 * MediationWorkspacePage tests (bug-mediation-page-no-error-state).
 *
 * Regression: when the dispute fetch query errors the page used to fall
 * through to rendering the workspace with an "unknown" status placeholder.
 * It must instead show an explicit error banner (role="alert") with a retry
 * affordance.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import { MediationWorkspacePage } from './MediationWorkspacePage';

vi.mock('@ppt/api-client', () => ({
  useDispute: vi.fn(),
  useMediationSessions: vi.fn(() => ({ data: [], isLoading: false })),
  useResolveDispute: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useEscalateDispute: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useAssignMediator: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useScheduleSession: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useUpdateSession: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
  useCancelSession: vi.fn(() => ({ isPending: false, mutateAsync: vi.fn() })),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'disputes.mediation.backToDispute': 'Back to Dispute',
        'disputes.mediation.loadError.title': 'Failed to load dispute',
        'disputes.mediation.loadError.message': 'We could not load this case.',
        'disputes.mediation.loadError.retry': 'Try again',
        'disputes.mediation.loadError.retrying': 'Retrying',
        'common.loading': 'Loading',
      };
      return map[key] ?? key;
    },
    i18n: { language: 'en' },
  }),
}));

// Child components reach for other api-client hooks via their own imports;
// stub them so the page renders in isolation.
vi.mock('../components/MediationTimelineView', () => ({
  MediationTimelineView: () => null,
}));
vi.mock('../components/MediationChatThread', () => ({ MediationChatThread: () => null }));
vi.mock('../components/MediationSessionsPanel', () => ({ MediationSessionsPanel: () => null }));

import { useDispute } from '@ppt/api-client';

const mockUseDispute = vi.mocked(useDispute);

function renderPage() {
  return render(
    <MediationWorkspacePage
      disputeId="dsp-1"
      organizationId="org-1"
      onBack={vi.fn()}
      onToastSuccess={vi.fn()}
      onToastError={vi.fn()}
    />
  );
}

describe('MediationWorkspacePage error state', () => {
  it('renders an alert banner with retry when the dispute fetch errors', () => {
    mockUseDispute.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      isFetching: false,
      refetch: vi.fn(),
    } as unknown as ReturnType<typeof useDispute>);

    renderPage();

    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent('Failed to load dispute');
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
    // Must NOT fall through to the workspace title / unknown placeholder.
    expect(screen.queryByText('disputes.mediation.title')).not.toBeInTheDocument();
  });

  it('calls refetch when the retry button is clicked', () => {
    const refetch = vi.fn();
    mockUseDispute.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      isFetching: false,
      refetch,
    } as unknown as ReturnType<typeof useDispute>);

    renderPage();
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(refetch).toHaveBeenCalledTimes(1);
  });

  it('does not show the error banner while loading', () => {
    mockUseDispute.mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
      isFetching: true,
      refetch: vi.fn(),
    } as unknown as ReturnType<typeof useDispute>);

    renderPage();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByRole('status', { name: 'Loading' })).toBeInTheDocument();
  });
});
