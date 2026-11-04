/**
 * /rentals/connections end to end through the route wrapper: disconnect only
 * after the confirmation dialog (PUT is_active=false), reconnect, the settings
 * dialog (GET + PUT /rentals/connections/{id}) and removing a connection
 * (DELETE) — with the real page, the app's ConfirmationDialog and mocked
 * `@ppt/api-client` calls.
 */
import {
  deleteRentalConnection,
  getRentalConnection,
  listRentalBuildings,
  listRentalConnections,
  updateRentalConnection,
} from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../components';
import { rentalRoutes } from './rentals';

vi.mock('@ppt/api-client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@ppt/api-client')>()),
  getToken: vi.fn(() => 'tok'),
  listRentalConnections: vi.fn(),
  listRentalBuildings: vi.fn(),
  listRentalBuildingUnits: vi.fn(),
  getRentalConnection: vi.fn(),
  updateRentalConnection: vi.fn(),
  deleteRentalConnection: vi.fn(),
  syncAirbnb: vi.fn(),
  syncBooking: vi.fn(),
}));

const logout = vi.fn();
vi.mock('../../contexts/AuthContext', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../contexts/AuthContext')>()),
  useAuth: () => ({
    isAuthenticated: true,
    isLoading: false,
    user: { id: 'u1', organizationId: 'org-1', role: 'manager' },
    logout,
  }),
}));

const HEADERS = { Authorization: 'Bearer tok', 'X-Tenant-ID': 'org-1' };

const connected = {
  id: 'c1',
  unit_id: 'u1',
  unit_name: 'Byt 1A',
  platform: 'airbnb',
  is_active: true,
  last_sync_at: '2026-11-04T00:00:00Z',
  sync_error: null,
};
const inactive = { ...connected, id: 'c2', unit_name: 'Byt 2B', is_active: false };

const detail = {
  id: 'c1',
  organization_id: 'org-1',
  unit_id: 'u1',
  platform: 'airbnb',
  is_connected: true,
  external_property_id: 'ABNB-42',
  external_listing_url: null,
  is_active: true,
  sync_calendar: true,
  sync_interval_minutes: 15,
  block_other_platforms: true,
  created_at: '2026-10-01T00:00:00Z',
  updated_at: '2026-10-01T00:00:00Z',
};

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  render(
    <QueryClientProvider client={queryClient}>
      <ToastProvider>
        <MemoryRouter initialEntries={['/rentals/connections']}>
          <Routes>
            {rentalRoutes()}
            <Route path="/settings/integrations" element={<p>integrations page</p>} />
          </Routes>
        </MemoryRouter>
      </ToastProvider>
    </QueryClientProvider>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(listRentalConnections).mockResolvedValue([connected, inactive]);
  vi.mocked(listRentalBuildings).mockResolvedValue({ buildings: [], total: 0 } as never);
  vi.mocked(getRentalConnection).mockResolvedValue(detail);
  vi.mocked(updateRentalConnection).mockResolvedValue(detail);
  vi.mocked(deleteRentalConnection).mockResolvedValue(undefined);
});

describe('/rentals/connections', () => {
  it('disconnects only after the confirmation dialog, via PUT is_active=false', async () => {
    renderPage();
    fireEvent.click(await screen.findByRole('button', { name: 'Disconnect' }));

    const dialog = await screen.findByRole('alertdialog');
    expect(within(dialog).getByText('Disconnect this platform?')).toBeInTheDocument();
    expect(within(dialog).getByText(/Byt 1A/)).toBeInTheDocument();
    expect(updateRentalConnection).not.toHaveBeenCalled();

    fireEvent.click(within(dialog).getByRole('button', { name: 'Disconnect' }));
    await waitFor(() =>
      expect(updateRentalConnection).toHaveBeenCalledWith('c1', { is_active: false }, HEADERS)
    );
    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument());
    expect(await screen.findByText('Platform disconnected')).toBeInTheDocument();
  });

  it('does nothing when the disconnect dialog is cancelled', async () => {
    renderPage();
    fireEvent.click(await screen.findByRole('button', { name: 'Disconnect' }));
    fireEvent.click(within(await screen.findByRole('alertdialog')).getByText('Cancel'));
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(updateRentalConnection).not.toHaveBeenCalled();
  });

  it('reconnects a disconnected connection via PUT is_active=true', async () => {
    renderPage();
    fireEvent.click(await screen.findByRole('button', { name: 'Reconnect' }));
    await waitFor(() =>
      expect(updateRentalConnection).toHaveBeenCalledWith('c2', { is_active: true }, HEADERS)
    );
  });

  it('loads the settings, saves the edited values and closes', async () => {
    renderPage();
    const [settingsButton] = await screen.findAllByRole('button', {
      name: 'Airbnb connection settings',
    });
    fireEvent.click(settingsButton);

    expect(getRentalConnection).toHaveBeenCalledWith('c1', HEADERS);
    const propertyId = await screen.findByLabelText('Listing ID on the platform');
    expect(propertyId).toHaveValue('ABNB-42');

    fireEvent.change(screen.getByLabelText('Listing URL'), {
      target: { value: 'https://airbnb.com/rooms/42' },
    });
    fireEvent.change(screen.getByLabelText('Sync interval'), { target: { value: '60' } });
    fireEvent.click(screen.getByLabelText(/Block dates on the other platforms/));
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() =>
      expect(updateRentalConnection).toHaveBeenCalledWith(
        'c1',
        {
          sync_calendar: true,
          sync_interval_minutes: 60,
          block_other_platforms: false,
          external_property_id: 'ABNB-42',
          external_listing_url: 'https://airbnb.com/rooms/42',
        },
        HEADERS
      )
    );
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(await screen.findByText('Connection settings saved')).toBeInTheDocument();
  });

  it('blocks saving an invalid listing URL', async () => {
    renderPage();
    const [settingsButton] = await screen.findAllByRole('button', {
      name: 'Airbnb connection settings',
    });
    fireEvent.click(settingsButton);
    fireEvent.change(await screen.findByLabelText('Listing URL'), {
      target: { value: 'airbnb rooms 42' },
    });
    expect(screen.getByText('Enter a full address starting with https://')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  it('keeps the settings dialog open with an error when saving fails', async () => {
    vi.mocked(updateRentalConnection).mockRejectedValueOnce(new Error('500'));
    renderPage();
    const [settingsButton] = await screen.findAllByRole('button', {
      name: 'Airbnb connection settings',
    });
    fireEvent.click(settingsButton);
    await screen.findByLabelText('Listing ID on the platform');
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(
      await screen.findByText('The settings could not be saved. Please try again.')
    ).toBeInTheDocument();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('removes a connection from the settings dialog after confirming (DELETE)', async () => {
    renderPage();
    const [settingsButton] = await screen.findAllByRole('button', {
      name: 'Airbnb connection settings',
    });
    fireEvent.click(settingsButton);
    await screen.findByLabelText('Listing ID on the platform');
    fireEvent.click(screen.getByRole('button', { name: 'Remove connection' }));

    const confirm = await screen.findByRole('alertdialog');
    expect(deleteRentalConnection).not.toHaveBeenCalled();
    fireEvent.click(within(confirm).getByRole('button', { name: 'Remove' }));
    await waitFor(() => expect(deleteRentalConnection).toHaveBeenCalledWith('c1', HEADERS));
    expect(await screen.findByText('Connection removed')).toBeInTheDocument();
  });

  it('sends a pending (not yet authorized) connection to the integrations page', async () => {
    vi.mocked(listRentalConnections).mockResolvedValue([{ ...connected, last_sync_at: null }]);
    renderPage();
    fireEvent.click(await screen.findByRole('button', { name: 'Connect Airbnb' }));
    expect(await screen.findByText('integrations page')).toBeInTheDocument();
    expect(updateRentalConnection).not.toHaveBeenCalled();
  });
});
