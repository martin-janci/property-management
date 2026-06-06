/**
 * Booking.com Channel Panel (Gap 83.2)
 *
 * Connection management UI for the Booking.com channel-manager integration:
 * connect/disconnect, availability/rate sync status, manual sync trigger, and
 * cross-platform booking-conflict feed. Wires to the api-server
 * `routes/integrations/{install,booking_channel}.rs` surface.
 */

import type { BookingConnectRequest } from '@ppt/api-client';
import {
  useBookingConflicts,
  useBookingStatus,
  useConnectBooking,
  useDisconnectBooking,
  useSyncBooking,
} from '@ppt/api-client';
import { useState } from 'react';
import { ConfirmationDialog, useToast } from '../../../components';

interface BookingChannelPanelProps {
  organizationId: string;
}

function formatDateTime(value?: string | null): string {
  if (!value) return 'Never';
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? value : d.toLocaleString();
}

export function BookingChannelPanel({ organizationId }: BookingChannelPanelProps) {
  const { showToast } = useToast();

  const { data: status, isLoading } = useBookingStatus(organizationId);
  const connect = useConnectBooking(organizationId);
  const disconnect = useDisconnectBooking(organizationId);
  const sync = useSyncBooking(organizationId);

  const connected = !!status?.connected;
  const {
    data: conflicts,
    isLoading: conflictsLoading,
    refetch: refetchConflicts,
  } = useBookingConflicts(organizationId, connected);

  const [showConnectDialog, setShowConnectDialog] = useState(false);
  const [disconnectOpen, setDisconnectOpen] = useState(false);
  const [form, setForm] = useState<BookingConnectRequest>({
    hotel_id: '',
    username: '',
    password: '',
  });

  const resetForm = () => setForm({ hotel_id: '', username: '', password: '' });

  const handleConnect = async () => {
    try {
      const res = await connect.mutateAsync(form);
      showToast({ title: res.message || 'Booking.com connected', type: 'success' });
      setShowConnectDialog(false);
      resetForm();
    } catch (error) {
      showToast({
        title: error instanceof Error ? error.message : 'Failed to connect Booking.com',
        type: 'error',
      });
    }
  };

  const handleDisconnect = async () => {
    try {
      await disconnect.mutateAsync();
      showToast({ title: 'Booking.com disconnected', type: 'success' });
    } catch (error) {
      showToast({
        title: error instanceof Error ? error.message : 'Failed to disconnect',
        type: 'error',
      });
    } finally {
      setDisconnectOpen(false);
    }
  };

  const handleSync = async () => {
    try {
      const res = await sync.mutateAsync();
      if (res.success) {
        showToast({
          title: `Synced ${res.items_synced} item(s) from Booking.com`,
          type: 'success',
        });
      } else {
        showToast({ title: res.error || 'Sync reported an error', type: 'error' });
      }
    } catch (error) {
      showToast({
        title: error instanceof Error ? error.message : 'Failed to sync',
        type: 'error',
      });
    }
  };

  if (isLoading) {
    return <div className="text-sm text-muted-foreground">Loading Booking.com status…</div>;
  }

  return (
    <div className="space-y-6">
      {/* Connection status card */}
      <div className="rounded-lg border p-5">
        <div className="flex items-start justify-between gap-4">
          <div className="flex items-center gap-4">
            <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-blue-100 text-blue-700 font-semibold">
              B
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-lg font-semibold">Booking.com</h3>
                <span
                  className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${
                    connected ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-600'
                  }`}
                >
                  {connected ? 'Connected' : 'Not connected'}
                </span>
              </div>
              <p className="text-sm text-muted-foreground">
                Channel-manager sync for availability, rates &amp; reservations
              </p>
            </div>
          </div>

          <div className="flex gap-2">
            {connected ? (
              <>
                <button
                  type="button"
                  onClick={handleSync}
                  disabled={sync.isPending}
                  className="px-3 py-2 text-sm border rounded-md hover:bg-muted disabled:opacity-50"
                >
                  {sync.isPending ? 'Syncing…' : 'Sync now'}
                </button>
                <button
                  type="button"
                  onClick={() => setDisconnectOpen(true)}
                  className="px-3 py-2 text-sm border rounded-md text-red-600 hover:bg-red-50"
                >
                  Disconnect
                </button>
              </>
            ) : (
              <button
                type="button"
                onClick={() => setShowConnectDialog(true)}
                className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90"
              >
                Connect
              </button>
            )}
          </div>
        </div>

        {connected && (
          <dl className="mt-5 grid grid-cols-2 gap-4 sm:grid-cols-4">
            <div>
              <dt className="text-xs text-muted-foreground">Hotel ID</dt>
              <dd className="text-sm font-medium">{status?.hotel_id || '—'}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Properties mapped</dt>
              <dd className="text-sm font-medium">{status?.properties_count ?? 0}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Reservations</dt>
              <dd className="text-sm font-medium">{status?.reservations_count ?? 0}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Last sync</dt>
              <dd className="text-sm font-medium">{formatDateTime(status?.last_sync_at)}</dd>
            </div>
          </dl>
        )}

        {connected && status?.sync_error && (
          <div className="mt-4 rounded-md bg-red-50 px-3 py-2 text-sm text-red-700">
            Last sync error: {status.sync_error}
          </div>
        )}
      </div>

      {/* Conflict feed */}
      {connected && (
        <div className="rounded-lg border p-5">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-base font-semibold">Reservation conflicts</h3>
              <p className="text-sm text-muted-foreground">
                Overlapping bookings between Booking.com and other channels
              </p>
            </div>
            <button
              type="button"
              onClick={() => refetchConflicts()}
              disabled={conflictsLoading}
              className="px-3 py-2 text-sm border rounded-md hover:bg-muted disabled:opacity-50"
            >
              {conflictsLoading ? 'Checking…' : 'Refresh'}
            </button>
          </div>

          {conflictsLoading ? (
            <p className="mt-4 text-sm text-muted-foreground">Checking for conflicts…</p>
          ) : conflicts && conflicts.conflicts_found > 0 ? (
            <div className="mt-4 space-y-3">
              {conflicts.truncated && (
                <div className="rounded-md bg-amber-50 px-3 py-2 text-xs text-amber-700">
                  Results may be incomplete — the upstream reservation cap was reached.
                </div>
              )}
              <ul className="divide-y rounded-md border">
                {conflicts.conflicts.map((c) => (
                  <li
                    key={`${c.booking_reservation_id}-${c.conflicting_booking_id}`}
                    className="flex items-center justify-between gap-4 px-4 py-3"
                  >
                    <div>
                      <div className="text-sm font-medium">
                        Booking.com reservation {c.booking_reservation_id}
                      </div>
                      <div className="text-xs text-muted-foreground">
                        Conflicts with {c.conflicting_platform} · {c.overlap_start} →{' '}
                        {c.overlap_end}
                      </div>
                    </div>
                    <span className="inline-flex items-center rounded-full bg-red-100 px-2 py-0.5 text-xs font-medium text-red-700">
                      Overlap
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          ) : (
            <p className="mt-4 text-sm text-muted-foreground">No conflicts detected.</p>
          )}
        </div>
      )}

      {/* Connect dialog */}
      {showConnectDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-full max-w-md bg-background rounded-lg shadow-lg p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-xl font-semibold">Connect Booking.com</h2>
              <button
                type="button"
                onClick={() => {
                  setShowConnectDialog(false);
                  resetForm();
                }}
                className="text-muted-foreground hover:text-foreground"
              >
                X
              </button>
            </div>

            <p className="text-sm text-muted-foreground mb-4">
              Enter your Booking.com Supply XML (Connectivity) credentials. These authorize
              availability and rate pushes for your hotel.
            </p>

            <div className="space-y-3">
              <div>
                <label htmlFor="booking-hotel-id" className="block text-sm font-medium mb-1">
                  Hotel ID
                </label>
                <input
                  id="booking-hotel-id"
                  type="text"
                  value={form.hotel_id}
                  onChange={(e) => setForm((f) => ({ ...f, hotel_id: e.target.value }))}
                  placeholder="e.g. 1234567"
                  className="w-full px-3 py-2 border rounded-md bg-background"
                />
              </div>
              <div>
                <label htmlFor="booking-username" className="block text-sm font-medium mb-1">
                  Username
                </label>
                <input
                  id="booking-username"
                  type="text"
                  value={form.username}
                  onChange={(e) => setForm((f) => ({ ...f, username: e.target.value }))}
                  autoComplete="off"
                  className="w-full px-3 py-2 border rounded-md bg-background"
                />
              </div>
              <div>
                <label htmlFor="booking-password" className="block text-sm font-medium mb-1">
                  Password
                </label>
                <input
                  id="booking-password"
                  type="password"
                  value={form.password}
                  onChange={(e) => setForm((f) => ({ ...f, password: e.target.value }))}
                  autoComplete="new-password"
                  className="w-full px-3 py-2 border rounded-md bg-background"
                />
              </div>
            </div>

            <div className="flex justify-end gap-2 pt-5">
              <button
                type="button"
                onClick={() => {
                  setShowConnectDialog(false);
                  resetForm();
                }}
                className="px-4 py-2 text-sm border rounded-md hover:bg-muted"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleConnect}
                disabled={connect.isPending || !form.hotel_id || !form.username || !form.password}
                className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50"
              >
                {connect.isPending ? 'Connecting…' : 'Connect'}
              </button>
            </div>
          </div>
        </div>
      )}

      <ConfirmationDialog
        isOpen={disconnectOpen}
        title="Disconnect Booking.com?"
        message="This removes the stored credentials. Availability and rate sync will stop until you reconnect."
        confirmLabel="Disconnect"
        variant="danger"
        onConfirm={handleDisconnect}
        onCancel={() => setDisconnectOpen(false)}
      />
    </div>
  );
}
