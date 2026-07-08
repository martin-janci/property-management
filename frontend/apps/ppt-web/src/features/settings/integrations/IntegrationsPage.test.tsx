/**
 * Regression test for the Integrations settings surface (Gap 83-1).
 *
 * Pins the wiring that `IntegrationsPage` adds: the Airbnb card renders its
 * live status and the connect/sync controls are wired to the corresponding
 * `@ppt/api-client` integrations hooks. This layer did not exist on `dev`
 * (the api-client hooks were built but no ppt-web route rendered them), so
 * this test fails without the page + route wiring.
 */

/// <reference types="vitest/globals" />
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { IntegrationsPage } from './IntegrationsPage';

// ── api-client: Airbnb integration hooks ──
const connectMutateAsync = vi
  .fn()
  .mockResolvedValue({ auth_url: 'https://airbnb.test/oauth', state: 's' });
const syncMutateAsync = vi
  .fn()
  .mockResolvedValue({ success: true, items_synced: 3, synced_at: '' });
const disconnectMutateAsync = vi.fn().mockResolvedValue(undefined);

let airbnbStatus: {
  data?: { connected: boolean; listings_count: number; reservations_count: number } | undefined;
  isLoading: boolean;
  isError: boolean;
} = {
  data: { connected: false, listings_count: 0, reservations_count: 0 },
  isLoading: false,
  isError: false,
};

vi.mock('@ppt/api-client', () => ({
  useAirbnbStatus: () => airbnbStatus,
  useAirbnbListingMappings: () => ({ data: [], isLoading: false }),
  useConnectAirbnb: () => ({ mutateAsync: connectMutateAsync, isPending: false }),
  useSyncAirbnb: () => ({ mutateAsync: syncMutateAsync, isPending: false }),
  useDisconnectAirbnb: () => ({ mutateAsync: disconnectMutateAsync, isPending: false }),
}));

vi.mock('../../../contexts', () => ({
  useAuth: () => ({ user: { organizationId: 'org-1' } }),
}));

const mockShowToast = vi.fn();
vi.mock('../../../components', () => ({
  useToast: () => ({ showToast: mockShowToast }),
}));

describe('IntegrationsPage (Gap 83-1)', () => {
  beforeEach(() => {
    connectMutateAsync.mockClear();
    syncMutateAsync.mockClear();
    disconnectMutateAsync.mockClear();
    mockShowToast.mockClear();
    airbnbStatus = {
      data: { connected: false, listings_count: 0, reservations_count: 0 },
      isLoading: false,
      isError: false,
    };
  });

  it('renders the integrations surface with the Airbnb card', () => {
    render(<IntegrationsPage />);
    expect(screen.getByRole('heading', { level: 1, name: /integrations/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 2, name: /airbnb/i })).toBeInTheDocument();
  });

  it('fires the Airbnb OAuth connect flow when not connected', async () => {
    render(<IntegrationsPage />);
    await userEvent.click(screen.getByRole('button', { name: /connect airbnb/i }));
    expect(connectMutateAsync).toHaveBeenCalledTimes(1);
  });

  it('exposes sync + disconnect controls once connected', async () => {
    airbnbStatus = {
      data: { connected: true, listings_count: 2, reservations_count: 5 },
      isLoading: false,
      isError: false,
    };
    render(<IntegrationsPage />);
    await userEvent.click(screen.getByRole('button', { name: /sync now/i }));
    expect(syncMutateAsync).toHaveBeenCalledTimes(1);
    expect(screen.getByRole('button', { name: /disconnect/i })).toBeInTheDocument();
  });
});
