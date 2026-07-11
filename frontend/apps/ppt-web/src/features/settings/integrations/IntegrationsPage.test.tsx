/**
 * Regression test for the Integrations settings surface (Gap 83-1 + Gap 83-2).
 *
 * Pins the wiring that `IntegrationsPage` adds: the Airbnb card renders its
 * live status and the connect/sync controls are wired to the corresponding
 * `@ppt/api-client` integrations hooks. This layer did not exist on `dev`
 * (the api-client hooks were built but no ppt-web route rendered them), so
 * this test fails without the page + route wiring.
 *
 * Gap 83-2 adds the Booking.com card: the property-mapping wizard (hotel ID +
 * Supply-XML credentials) drives `useConnectBooking`, and sync/disconnect
 * controls appear once connected.
 */

/// <reference types="vitest/globals" />
import { render, screen, within } from '@testing-library/react';
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

// ── api-client: Booking.com channel hooks ──
const bookingConnectMutateAsync = vi
  .fn()
  .mockResolvedValue({ success: true, message: 'Connected', hotel_id: 'H1' });
const bookingSyncMutateAsync = vi
  .fn()
  .mockResolvedValue({ success: true, items_synced: 2, synced_at: '' });
const bookingDisconnectMutateAsync = vi
  .fn()
  .mockResolvedValue({ success: true, message: 'Disconnected' });

let airbnbStatus: {
  data?: { connected: boolean; listings_count: number; reservations_count: number } | undefined;
  isLoading: boolean;
  isError: boolean;
} = {
  data: { connected: false, listings_count: 0, reservations_count: 0 },
  isLoading: false,
  isError: false,
};

let bookingStatus: {
  data?: { connected: boolean; properties_count: number; reservations_count: number } | undefined;
  isLoading: boolean;
  isError: boolean;
} = {
  data: { connected: false, properties_count: 0, reservations_count: 0 },
  isLoading: false,
  isError: false,
};

vi.mock('@ppt/api-client', () => ({
  useAirbnbStatus: () => airbnbStatus,
  useAirbnbListingMappings: () => ({ data: [], isLoading: false }),
  useConnectAirbnb: () => ({ mutateAsync: connectMutateAsync, isPending: false }),
  useSyncAirbnb: () => ({ mutateAsync: syncMutateAsync, isPending: false }),
  useDisconnectAirbnb: () => ({ mutateAsync: disconnectMutateAsync, isPending: false }),
  useBookingStatus: () => bookingStatus,
  useBookingConflicts: () => ({ data: { conflicts_found: 0, conflicts: [], truncated: false } }),
  useConnectBooking: () => ({ mutateAsync: bookingConnectMutateAsync, isPending: false }),
  useSyncBooking: () => ({ mutateAsync: bookingSyncMutateAsync, isPending: false }),
  useDisconnectBooking: () => ({ mutateAsync: bookingDisconnectMutateAsync, isPending: false }),
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
    bookingConnectMutateAsync.mockClear();
    bookingSyncMutateAsync.mockClear();
    bookingDisconnectMutateAsync.mockClear();
    mockShowToast.mockClear();
    airbnbStatus = {
      data: { connected: false, listings_count: 0, reservations_count: 0 },
      isLoading: false,
      isError: false,
    };
    bookingStatus = {
      data: { connected: false, properties_count: 0, reservations_count: 0 },
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

  it('does not send a frontend-origin redirect_uri (callback is backend-owned)', async () => {
    // The Airbnb OAuth callback + single-use CSRF `state` are verified
    // server-side, so the page must omit `redirect_uri` and let the backend
    // use its configured callback URL rather than pointing Airbnb back at the
    // SPA (which would land on an unhandled ?code=&state= page).
    render(<IntegrationsPage />);
    await userEvent.click(screen.getByRole('button', { name: /connect airbnb/i }));
    expect(connectMutateAsync).toHaveBeenCalledWith({});
    const arg = connectMutateAsync.mock.calls[0]?.[0] ?? {};
    expect(arg).not.toHaveProperty('redirect_uri');
  });

  it('redirects the browser to the returned Airbnb authorize URL', async () => {
    const originalLocation = window.location;
    // Replace window.location with a writable stub so we can observe the
    // handoff assignment without jsdom's unimplemented-navigation noise.
    Object.defineProperty(window, 'location', {
      configurable: true,
      writable: true,
      value: { href: '', origin: 'http://localhost' } as Location,
    });
    try {
      render(<IntegrationsPage />);
      await userEvent.click(screen.getByRole('button', { name: /connect airbnb/i }));
      expect(window.location.href).toBe('https://airbnb.test/oauth');
    } finally {
      Object.defineProperty(window, 'location', {
        configurable: true,
        writable: true,
        value: originalLocation,
      });
    }
  });

  it('shows an error toast when starting the Airbnb connection fails', async () => {
    connectMutateAsync.mockRejectedValueOnce(new Error('boom'));
    render(<IntegrationsPage />);
    await userEvent.click(screen.getByRole('button', { name: /connect airbnb/i }));
    expect(mockShowToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'error', message: 'boom' })
    );
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

  // ── Gap 83-2: Booking.com channel ──

  it('renders the Booking.com card', () => {
    render(<IntegrationsPage />);
    expect(screen.getByRole('heading', { level: 2, name: /booking\.com/i })).toBeInTheDocument();
  });

  it('drives the property-mapping wizard through useConnectBooking', async () => {
    render(<IntegrationsPage />);
    await userEvent.click(screen.getByRole('button', { name: /connect booking\.com/i }));
    // Wizard fields appear
    await userEvent.type(screen.getByLabelText(/hotel id/i), 'H1');
    await userEvent.type(screen.getByLabelText(/supply-xml username/i), 'user');
    await userEvent.type(screen.getByLabelText(/supply-xml password/i), 'secret');
    await userEvent.click(screen.getByRole('button', { name: /^connect$/i }));
    expect(bookingConnectMutateAsync).toHaveBeenCalledTimes(1);
    expect(bookingConnectMutateAsync).toHaveBeenCalledWith({
      hotel_id: 'H1',
      username: 'user',
      password: 'secret',
    });
  });

  it('exposes Booking.com sync + disconnect once connected', async () => {
    bookingStatus = {
      data: { connected: true, properties_count: 3, reservations_count: 7 },
      isLoading: false,
      isError: false,
    };
    render(<IntegrationsPage />);
    // Two "Sync now" buttons may exist (Airbnb + Booking) only if both connected;
    // here only Booking is connected, so scope by the Booking section.
    const bookingHeading = screen.getByRole('heading', { level: 2, name: /booking\.com/i });
    const section = bookingHeading.closest('section');
    expect(section).not.toBeNull();
    const sectionEl = section as HTMLElement;
    await userEvent.click(within(sectionEl).getByRole('button', { name: /sync now/i }));
    expect(bookingSyncMutateAsync).toHaveBeenCalledTimes(1);
    expect(within(sectionEl).getByRole('button', { name: /disconnect/i })).toBeInTheDocument();
  });
});
