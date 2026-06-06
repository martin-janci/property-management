/**
 * Airbnb Connection Panel (Gap 83-1 / Story 83.1)
 *
 * Manager-facing UI for the Airbnb integration:
 *  - OAuth connect / disconnect flow
 *  - Sync-now action + sync status badge
 *  - Listing mapping table (rentals connections filtered to Airbnb)
 *  - Reservation feed (rentals bookings filtered to Airbnb)
 *
 * Wires to the backend install-surface Airbnb routes from PR #538:
 *   GET    /api/v1/integrations/organizations/{org}/airbnb/status
 *   POST   /api/v1/integrations/organizations/{org}/airbnb/connect
 *   POST   /api/v1/integrations/organizations/{org}/airbnb/sync
 *   DELETE /api/v1/integrations/organizations/{org}/airbnb
 */

import type { AirbnbListingMapping, AirbnbReservation } from '@ppt/api-client';
import {
  useAirbnbListingMappings,
  useAirbnbReservations,
  useAirbnbStatus,
  useConnectAirbnb,
  useDisconnectAirbnb,
  useSyncAirbnb,
} from '@ppt/api-client';
import { useState } from 'react';
import { ConfirmationDialog, useToast } from '../../../components';

interface AirbnbConnectionPanelProps {
  organizationId: string;
}

function formatDateTime(value?: string | null): string {
  if (!value) return '—';
  return new Date(value).toLocaleString();
}

function formatDate(value?: string | null): string {
  if (!value) return '—';
  return new Date(value).toLocaleDateString();
}

/** Sync status badge — green when connected and error-free, red on sync error, gray otherwise. */
function SyncStatusBadge({
  connected,
  syncError,
}: {
  connected: boolean;
  syncError?: string | null;
}) {
  let label = 'Disconnected';
  let className = 'bg-gray-100 text-gray-800';
  if (syncError) {
    label = 'Error';
    className = 'bg-red-100 text-red-800';
  } else if (connected) {
    label = 'Connected';
    className = 'bg-green-100 text-green-800';
  }
  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${className}`}
    >
      {label}
    </span>
  );
}

function reservationStatusColor(status: string): string {
  switch (status) {
    case 'confirmed':
    case 'checked_in':
      return 'bg-green-100 text-green-800';
    case 'pending':
      return 'bg-yellow-100 text-yellow-800';
    case 'cancelled':
    case 'no_show':
      return 'bg-red-100 text-red-800';
    default:
      return 'bg-gray-100 text-gray-800';
  }
}

export function AirbnbConnectionPanel({ organizationId }: AirbnbConnectionPanelProps) {
  const { showToast } = useToast();
  const [disconnectOpen, setDisconnectOpen] = useState(false);

  const { data: status, isLoading: statusLoading } = useAirbnbStatus(organizationId);
  const { data: listings, isLoading: listingsLoading } = useAirbnbListingMappings(organizationId);
  const { data: reservationFeed, isLoading: reservationsLoading } =
    useAirbnbReservations(organizationId);

  const connectAirbnb = useConnectAirbnb(organizationId);
  const syncAirbnb = useSyncAirbnb(organizationId);
  const disconnectAirbnb = useDisconnectAirbnb(organizationId);

  const connected = status?.connected ?? false;

  const handleConnect = async () => {
    try {
      const result = await connectAirbnb.mutateAsync(undefined);
      // Redirect the browser to Airbnb's OAuth authorize page. After consent,
      // Airbnb redirects back to the backend callback which stores the tokens.
      window.location.href = result.auth_url;
    } catch {
      showToast({ title: 'Failed to start Airbnb connection', type: 'error' });
    }
  };

  const handleSync = async () => {
    try {
      const result = await syncAirbnb.mutateAsync();
      showToast({
        title: `Airbnb synced — ${result.items_synced} item(s)`,
        type: 'success',
      });
    } catch {
      showToast({ title: 'Failed to sync Airbnb', type: 'error' });
    }
  };

  const handleDisconnectConfirm = async () => {
    try {
      await disconnectAirbnb.mutateAsync();
      showToast({ title: 'Airbnb disconnected', type: 'success' });
    } catch {
      showToast({ title: 'Failed to disconnect Airbnb', type: 'error' });
    } finally {
      setDisconnectOpen(false);
    }
  };

  const reservations: AirbnbReservation[] = reservationFeed?.bookings ?? [];

  return (
    <div className="space-y-6">
      {/* Connection card */}
      <div className="rounded-lg border bg-card p-6">
        <div className="flex items-start justify-between mb-4">
          <div className="flex items-center gap-4">
            <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-rose-100 text-2xl font-bold text-rose-500">
              A
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-lg font-semibold">Airbnb</h3>
                <SyncStatusBadge connected={connected} syncError={status?.sync_error} />
              </div>
              <p className="text-sm text-muted-foreground">
                Sync listings and reservations from your Airbnb account
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            {connected ? (
              <>
                <button
                  type="button"
                  onClick={handleSync}
                  disabled={syncAirbnb.isPending}
                  className="px-3 py-2 text-sm border rounded-md hover:bg-muted disabled:opacity-50"
                >
                  {syncAirbnb.isPending ? 'Syncing…' : 'Sync Now'}
                </button>
                <button
                  type="button"
                  onClick={() => setDisconnectOpen(true)}
                  disabled={disconnectAirbnb.isPending}
                  className="px-3 py-2 text-sm text-red-600 border border-red-200 rounded-md hover:bg-red-50 disabled:opacity-50"
                >
                  Disconnect
                </button>
              </>
            ) : (
              <button
                type="button"
                onClick={handleConnect}
                disabled={connectAirbnb.isPending}
                className="px-4 py-2 text-sm font-medium text-white bg-rose-500 rounded-md hover:bg-rose-600 disabled:opacity-50"
              >
                {connectAirbnb.isPending ? 'Connecting…' : 'Connect Airbnb'}
              </button>
            )}
          </div>
        </div>

        {/* Status summary */}
        {statusLoading ? (
          <p className="text-sm text-muted-foreground">Loading status…</p>
        ) : (
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
            <div>
              <p className="text-xs text-muted-foreground">Listings</p>
              <p className="text-xl font-semibold">{status?.listings_count ?? 0}</p>
            </div>
            <div>
              <p className="text-xs text-muted-foreground">Reservations</p>
              <p className="text-xl font-semibold">{status?.reservations_count ?? 0}</p>
            </div>
            <div>
              <p className="text-xs text-muted-foreground">Account</p>
              <p className="text-sm font-mono break-all">{status?.external_account_id ?? '—'}</p>
            </div>
            <div>
              <p className="text-xs text-muted-foreground">Last Sync</p>
              <p className="text-sm">{formatDateTime(status?.last_sync_at)}</p>
            </div>
          </div>
        )}

        {status?.sync_error && (
          <div className="mt-4 rounded-md bg-red-50 p-3 text-sm text-red-700">
            {status.sync_error}
          </div>
        )}
      </div>

      {/* Listing mapping table */}
      <div className="rounded-lg border bg-card p-6">
        <h3 className="text-lg font-semibold mb-1">Listing Mappings</h3>
        <p className="text-sm text-muted-foreground mb-4">Units linked to Airbnb listings</p>
        {listingsLoading ? (
          <p className="text-sm text-muted-foreground">Loading…</p>
        ) : !listings || listings.length === 0 ? (
          <p className="text-sm text-muted-foreground">No Airbnb listings mapped yet.</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b text-left text-muted-foreground">
                  <th className="py-2 pr-4 font-medium">Unit</th>
                  <th className="py-2 pr-4 font-medium">Status</th>
                  <th className="py-2 pr-4 font-medium">Last Sync</th>
                  <th className="py-2 font-medium">Error</th>
                </tr>
              </thead>
              <tbody>
                {listings.map((mapping: AirbnbListingMapping) => (
                  <tr key={mapping.id} className="border-b last:border-0">
                    <td className="py-2 pr-4">{mapping.unit_name}</td>
                    <td className="py-2 pr-4">
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                          mapping.is_active
                            ? 'bg-green-100 text-green-800'
                            : 'bg-gray-100 text-gray-800'
                        }`}
                      >
                        {mapping.is_active ? 'Active' : 'Inactive'}
                      </span>
                    </td>
                    <td className="py-2 pr-4">{formatDateTime(mapping.last_sync_at)}</td>
                    <td className="py-2 text-red-600">{mapping.sync_error ?? '—'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Reservation feed */}
      <div className="rounded-lg border bg-card p-6">
        <h3 className="text-lg font-semibold mb-1">Reservations</h3>
        <p className="text-sm text-muted-foreground mb-4">Recent reservations pulled from Airbnb</p>
        {reservationsLoading ? (
          <p className="text-sm text-muted-foreground">Loading…</p>
        ) : reservations.length === 0 ? (
          <p className="text-sm text-muted-foreground">No Airbnb reservations yet.</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b text-left text-muted-foreground">
                  <th className="py-2 pr-4 font-medium">Guest</th>
                  <th className="py-2 pr-4 font-medium">Unit</th>
                  <th className="py-2 pr-4 font-medium">Check-in</th>
                  <th className="py-2 pr-4 font-medium">Check-out</th>
                  <th className="py-2 pr-4 font-medium">Nights</th>
                  <th className="py-2 pr-4 font-medium">Total</th>
                  <th className="py-2 font-medium">Status</th>
                </tr>
              </thead>
              <tbody>
                {reservations.map((res: AirbnbReservation) => (
                  <tr key={res.id} className="border-b last:border-0">
                    <td className="py-2 pr-4">
                      {res.guest_name}
                      <span className="text-muted-foreground"> ({res.guest_count})</span>
                    </td>
                    <td className="py-2 pr-4">{res.unit_name}</td>
                    <td className="py-2 pr-4">{formatDate(res.check_in)}</td>
                    <td className="py-2 pr-4">{formatDate(res.check_out)}</td>
                    <td className="py-2 pr-4">{res.nights}</td>
                    <td className="py-2 pr-4">
                      {res.total_amount ? `${res.total_amount} ${res.currency ?? ''}` : '—'}
                    </td>
                    <td className="py-2">
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${reservationStatusColor(
                          res.status
                        )}`}
                      >
                        {res.status}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <ConfirmationDialog
        isOpen={disconnectOpen}
        title="Disconnect Airbnb"
        message="Are you sure you want to disconnect Airbnb? This will stop syncing listings and reservations and clear stored tokens."
        confirmLabel="Disconnect"
        cancelLabel="Cancel"
        variant="danger"
        onConfirm={handleDisconnectConfirm}
        onCancel={() => setDisconnectOpen(false)}
        isLoading={disconnectAirbnb.isPending}
      />
    </div>
  );
}
