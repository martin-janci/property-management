/**
 * Integrations settings page (Gap 83-1 / Story 83.1 — Airbnb OAuth & Sync).
 *
 * The manager-facing surface that manages external channel integrations. The
 * api-client `integrations` module (Epic 61 + Gap 83) was already built with
 * live Airbnb hooks, but no ppt-web route rendered it. This page wires those
 * hooks into a reachable `/settings/integrations` surface.
 *
 * Airbnb calls (via @ppt/api-client):
 *   GET    /api/v1/integrations/organizations/{org}/airbnb/status   — useAirbnbStatus
 *   POST   /api/v1/integrations/organizations/{org}/airbnb/connect  — useConnectAirbnb (OAuth)
 *   POST   /api/v1/integrations/organizations/{org}/airbnb/sync     — useSyncAirbnb
 *   DELETE /api/v1/integrations/organizations/{org}/airbnb          — useDisconnectAirbnb
 *   GET    /api/v1/rentals/connections?platform=airbnb             — useAirbnbListingMappings
 */

import {
  useAirbnbListingMappings,
  useAirbnbStatus,
  useConnectAirbnb,
  useDisconnectAirbnb,
  useSyncAirbnb,
} from '@ppt/api-client';
import type React from 'react';
import { useToast } from '../../../components';
import { useAuth } from '../../../contexts';

function formatDate(value: string | null | undefined): string {
  if (!value) return '—';
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? '—' : d.toLocaleString();
}

export const IntegrationsPage: React.FC = () => {
  const { showToast } = useToast();
  const { user } = useAuth();
  const organizationId = user?.organizationId ?? '';

  const status = useAirbnbStatus(organizationId);
  const listings = useAirbnbListingMappings(organizationId);
  const connectAirbnb = useConnectAirbnb(organizationId);
  const syncAirbnb = useSyncAirbnb(organizationId);
  const disconnectAirbnb = useDisconnectAirbnb(organizationId);

  const connected = status.data?.connected ?? false;

  const handleConnect = async () => {
    try {
      const res = await connectAirbnb.mutateAsync({
        redirect_uri: `${window.location.origin}/settings/integrations`,
      });
      // Backend returns the OAuth authorize URL; hand off to Airbnb.
      window.location.href = res.auth_url;
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Could not start Airbnb connection',
        message: err instanceof Error ? err.message : 'An unexpected error occurred.',
      });
    }
  };

  const handleSync = async () => {
    try {
      const res = await syncAirbnb.mutateAsync();
      showToast({
        type: res.success ? 'success' : 'error',
        title: res.success ? 'Airbnb synced' : 'Airbnb sync failed',
        message: res.success
          ? `${res.items_synced} item(s) synced.`
          : (res.error ?? 'The sync did not complete.'),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Airbnb sync failed',
        message: err instanceof Error ? err.message : 'An unexpected error occurred.',
      });
    }
  };

  const handleDisconnect = async () => {
    try {
      await disconnectAirbnb.mutateAsync();
      showToast({
        type: 'success',
        title: 'Airbnb disconnected',
        message: 'The Airbnb integration has been removed.',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Could not disconnect Airbnb',
        message: err instanceof Error ? err.message : 'An unexpected error occurred.',
      });
    }
  };

  return (
    <div className="max-w-4xl mx-auto p-6 space-y-6">
      <header>
        <h1 className="text-2xl font-bold">Integrations</h1>
        <p className="text-sm" style={{ color: 'var(--color-text-secondary)' }}>
          Connect external booking channels to sync listings and reservations.
        </p>
      </header>

      {!organizationId && (
        <div
          className="rounded-lg border border-yellow-300 bg-yellow-50 p-4 text-sm text-yellow-800"
          role="alert"
        >
          No organization is selected for your account, so integrations cannot be managed.
        </div>
      )}

      {/* Airbnb integration card (Gap 83-1) */}
      <section className="rounded-lg border border-gray-200 bg-white p-6 space-y-4">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-lg font-semibold flex items-center gap-2">
              <span
                aria-hidden="true"
                className="inline-flex h-8 w-8 items-center justify-center rounded-full bg-rose-500 text-white text-sm font-bold"
              >
                A
              </span>
              Airbnb
            </h2>
            <p className="text-sm text-gray-500">
              OAuth-connected channel for short-term rental listings and reservations.
            </p>
          </div>
          <span
            className={`shrink-0 rounded-full px-3 py-1 text-xs font-medium ${
              connected ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-600'
            }`}
          >
            {status.isLoading ? 'Checking…' : connected ? 'Connected' : 'Not connected'}
          </span>
        </div>

        {status.isError && (
          <div className="rounded-md bg-red-50 p-3 text-sm text-red-700" role="alert">
            Could not load Airbnb status.
          </div>
        )}

        {connected && status.data && (
          <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm sm:grid-cols-4">
            <div>
              <dt className="text-gray-500">Account</dt>
              <dd className="font-medium text-gray-900">
                {status.data.external_account_id ?? '—'}
              </dd>
            </div>
            <div>
              <dt className="text-gray-500">Last sync</dt>
              <dd className="font-medium text-gray-900">{formatDate(status.data.last_sync_at)}</dd>
            </div>
            <div>
              <dt className="text-gray-500">Listings</dt>
              <dd className="font-medium text-gray-900">{status.data.listings_count}</dd>
            </div>
            <div>
              <dt className="text-gray-500">Reservations</dt>
              <dd className="font-medium text-gray-900">{status.data.reservations_count}</dd>
            </div>
          </dl>
        )}

        {connected && status.data?.sync_error && (
          <div className="rounded-md bg-red-50 p-3 text-sm text-red-700" role="alert">
            Last sync error: {status.data.sync_error}
          </div>
        )}

        <div className="flex flex-wrap gap-3 pt-2">
          {!connected ? (
            <button
              type="button"
              onClick={handleConnect}
              disabled={!organizationId || connectAirbnb.isPending}
              className="rounded-md bg-rose-500 px-4 py-2 text-sm font-medium text-white hover:bg-rose-600 disabled:opacity-50"
            >
              {connectAirbnb.isPending ? 'Redirecting…' : 'Connect Airbnb'}
            </button>
          ) : (
            <>
              <button
                type="button"
                onClick={handleSync}
                disabled={syncAirbnb.isPending}
                className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
              >
                {syncAirbnb.isPending ? 'Syncing…' : 'Sync now'}
              </button>
              <button
                type="button"
                onClick={handleDisconnect}
                disabled={disconnectAirbnb.isPending}
                className="rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
              >
                {disconnectAirbnb.isPending ? 'Disconnecting…' : 'Disconnect'}
              </button>
            </>
          )}
        </div>

        {connected && (
          <div className="pt-2">
            <h3 className="text-sm font-semibold text-gray-700">Listing mappings</h3>
            {listings.isLoading ? (
              <p className="text-sm text-gray-500">Loading listings…</p>
            ) : (listings.data?.length ?? 0) === 0 ? (
              <p className="text-sm text-gray-500">No Airbnb listings mapped yet.</p>
            ) : (
              <ul className="mt-2 divide-y divide-gray-100 rounded-md border border-gray-100">
                {listings.data?.map((mapping) => (
                  <li
                    key={mapping.id}
                    className="flex items-center justify-between px-3 py-2 text-sm"
                  >
                    <span className="font-medium text-gray-900">{mapping.unit_name}</span>
                    <span className={mapping.is_active ? 'text-green-600' : 'text-gray-400'}>
                      {mapping.is_active ? 'Active' : 'Inactive'}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </section>
    </div>
  );
};

export default IntegrationsPage;
