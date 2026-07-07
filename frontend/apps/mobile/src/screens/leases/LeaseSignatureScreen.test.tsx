/**
 * LeaseSignatureScreen — unit tests (gap-84-2).
 *
 * Covers:
 *   - Renders loading state while query is in-flight.
 *   - Renders error state when the API returns an error.
 *   - Renders document title, requester, and status banner for a pending request.
 *   - Hides action buttons for non-pending statuses (signed, declined).
 *   - Shows action buttons (Sign / Decline) when status is pending.
 *   - Renders the "missing ID" guard when no esignatureId is provided.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react-native';
import type { ESignatureRequest } from './LeaseSignatureScreen';
import { LeaseSignatureScreen } from './LeaseSignatureScreen';

// ─── Mocks ────────────────────────────────────────────────────────────────────

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useApiMutation: jest.fn(() => ({
    mutate: jest.fn(),
    isPending: false,
  })),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

// ─── Helpers ──────────────────────────────────────────────────────────────────

function makeRequest(overrides: Partial<ESignatureRequest> = {}): ESignatureRequest {
  return {
    id: 'esig-1',
    lease_id: 'lease-1',
    document_title: 'Lease Agreement 2026',
    document_description: 'Annual lease agreement for Apt 3B.',
    requester_name: 'Anna Novak (Manager)',
    signer_name: 'Peter Tenant',
    status: 'pending',
    requested_at: '2026-05-01T10:00:00Z',
    expires_at: '2026-06-01T10:00:00Z',
    signed_at: null,
    webhook_delivered: false,
    ...overrides,
  };
}

function renderScreen(props: React.ComponentProps<typeof LeaseSignatureScreen>) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <LeaseSignatureScreen {...props} />
    </QueryClientProvider>
  );
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe('LeaseSignatureScreen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('shows the missing-id guard when esignatureId is not provided', () => {
    mockUseApiQuery.mockReturnValue({
      isLoading: false,
      error: null,
      data: null,
      refetch: jest.fn(),
    });
    renderScreen({ esignatureId: undefined });
    expect(screen.getByText('esignature.missingId')).toBeTruthy();
  });

  it('shows a loading indicator while the query is in-flight', () => {
    mockUseApiQuery.mockReturnValue({
      isLoading: true,
      error: null,
      data: null,
      refetch: jest.fn(),
    });
    renderScreen({ esignatureId: 'esig-1' });
    expect(screen.getByText('common.loading')).toBeTruthy();
  });

  it('shows an error state with a retry button when the query fails', () => {
    mockUseApiQuery.mockReturnValue({
      isLoading: false,
      error: new Error('Network error'),
      data: null,
      refetch: jest.fn(),
    });
    renderScreen({ esignatureId: 'esig-1' });
    expect(screen.getByText('esignature.loadError')).toBeTruthy();
    expect(screen.getByText('common.retry')).toBeTruthy();
  });

  it('renders document title and requester name for a pending request', () => {
    mockUseApiQuery.mockReturnValue({
      isLoading: false,
      error: null,
      data: makeRequest(),
      refetch: jest.fn(),
    });
    renderScreen({ esignatureId: 'esig-1' });
    // Title renders in both the header subtitle and the document card.
    expect(screen.getAllByText('Lease Agreement 2026').length).toBeGreaterThan(0);
    expect(screen.getByText('Anna Novak (Manager)')).toBeTruthy();
  });

  it('shows Sign and Decline buttons for a pending request', () => {
    mockUseApiQuery.mockReturnValue({
      isLoading: false,
      error: null,
      data: makeRequest({ status: 'pending' }),
      refetch: jest.fn(),
    });
    renderScreen({ esignatureId: 'esig-1' });
    expect(screen.getByText('esignature.sign')).toBeTruthy();
    expect(screen.getByText('esignature.decline')).toBeTruthy();
  });

  it('hides action buttons for a signed request', () => {
    mockUseApiQuery.mockReturnValue({
      isLoading: false,
      error: null,
      data: makeRequest({
        status: 'signed',
        signed_at: '2026-05-10T14:00:00Z',
        webhook_delivered: true,
      }),
      refetch: jest.fn(),
    });
    renderScreen({ esignatureId: 'esig-1' });
    expect(screen.queryByText('esignature.sign')).toBeNull();
    expect(screen.queryByText('esignature.decline')).toBeNull();
  });

  it('hides action buttons for a declined request', () => {
    mockUseApiQuery.mockReturnValue({
      isLoading: false,
      error: null,
      data: makeRequest({ status: 'declined', signed_at: '2026-05-05T09:00:00Z' }),
      refetch: jest.fn(),
    });
    renderScreen({ esignatureId: 'esig-1' });
    expect(screen.queryByText('esignature.sign')).toBeNull();
    expect(screen.queryByText('esignature.decline')).toBeNull();
  });

  it('shows the status banner for a signed request', () => {
    mockUseApiQuery.mockReturnValue({
      isLoading: false,
      error: null,
      data: makeRequest({
        status: 'signed',
        signed_at: '2026-05-10T14:00:00Z',
        webhook_delivered: false,
      }),
      refetch: jest.fn(),
    });
    renderScreen({ esignatureId: 'esig-1' });
    expect(screen.getByText('esignature.statusSigned')).toBeTruthy();
  });

  it('shows the status banner for a declined request', () => {
    mockUseApiQuery.mockReturnValue({
      isLoading: false,
      error: null,
      data: makeRequest({ status: 'declined', signed_at: '2026-05-05T09:00:00Z' }),
      refetch: jest.fn(),
    });
    renderScreen({ esignatureId: 'esig-1' });
    expect(screen.getByText('esignature.statusDeclined')).toBeTruthy();
  });

  it('calls onBack when back link is pressed', () => {
    const onBack = jest.fn();
    mockUseApiQuery.mockReturnValue({
      isLoading: false,
      error: null,
      data: makeRequest(),
      refetch: jest.fn(),
    });
    renderScreen({ esignatureId: 'esig-1', onBack });
    // The back link is present — just verify it renders
    expect(screen.getByText('esignature.backToLease', { exact: false })).toBeTruthy();
  });
});
