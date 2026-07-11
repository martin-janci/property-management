/**
 * Integrations settings page (Gap 83-1 / Story 83.1 — Airbnb OAuth & Sync;
 * Gap 83-2 / Story 83.2 — Booking.com channel connect + property mapping & sync).
 *
 * The manager-facing surface that manages external channel integrations. The
 * api-client `integrations` module (Epic 61 + Gap 83) was already built with
 * live Airbnb + Booking.com hooks, but no ppt-web route rendered them. This
 * page wires those hooks into a reachable `/settings/integrations` surface.
 *
 * Airbnb calls (via @ppt/api-client):
 *   GET    /api/v1/integrations/organizations/{org}/airbnb/status   — useAirbnbStatus
 *   POST   /api/v1/integrations/organizations/{org}/airbnb/connect  — useConnectAirbnb (OAuth)
 *   POST   /api/v1/integrations/organizations/{org}/airbnb/sync     — useSyncAirbnb
 *   DELETE /api/v1/integrations/organizations/{org}/airbnb          — useDisconnectAirbnb
 *   GET    /api/v1/rentals/connections?platform=airbnb             — useAirbnbListingMappings
 *
 * Booking.com calls (via @ppt/api-client):
 *   GET    /api/v1/integrations/organizations/{org}/booking/status    — useBookingStatus
 *   POST   /api/v1/integrations/organizations/{org}/booking/connect   — useConnectBooking (property mapping wizard)
 *   POST   /api/v1/integrations/organizations/{org}/booking/sync      — useSyncBooking
 *   DELETE /api/v1/integrations/organizations/{org}/booking           — useDisconnectBooking
 *   GET    /api/v1/integrations/organizations/{org}/booking/conflicts — useBookingConflicts
 */

import {
  useAirbnbListingMappings,
  useAirbnbStatus,
  useBookingConflicts,
  useBookingStatus,
  useConnectAirbnb,
  useConnectBooking,
  useDisconnectAirbnb,
  useDisconnectBooking,
  useSyncAirbnb,
  useSyncBooking,
} from '@ppt/api-client';
import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../../../components';
import { useAuth } from '../../../contexts';

function formatDate(value: string | null | undefined): string {
  if (!value) return '—';
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? '—' : d.toLocaleString();
}

/**
 * Booking.com channel card (Gap 83-2).
 *
 * Unlike Airbnb's OAuth redirect, Booking.com connects through Supply-XML
 * credentials, so the "connect" action is a small property-mapping wizard:
 * the manager supplies the hotel ID + Supply-XML username/password, which maps
 * the org's units onto the Booking.com property. Once connected the card
 * surfaces the sync status, a manual sync, disconnect, and any cross-platform
 * reservation conflicts detected by the backend.
 */
const BookingIntegrationCard: React.FC<{ organizationId: string }> = ({ organizationId }) => {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const status = useBookingStatus(organizationId);
  const connectBooking = useConnectBooking(organizationId);
  const syncBooking = useSyncBooking(organizationId);
  const disconnectBooking = useDisconnectBooking(organizationId);
  const connected = status.data?.connected ?? false;
  const conflicts = useBookingConflicts(organizationId, connected);

  const [showWizard, setShowWizard] = useState(false);
  const [hotelId, setHotelId] = useState('');
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');

  const resetWizard = () => {
    setShowWizard(false);
    setHotelId('');
    setUsername('');
    setPassword('');
  };

  const handleConnect = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      const res = await connectBooking.mutateAsync({
        hotel_id: hotelId.trim(),
        username: username.trim(),
        password,
      });
      showToast({
        type: res.success ? 'success' : 'error',
        title: res.success
          ? t('settings.integrations.booking.toast.connectedTitle')
          : t('settings.integrations.booking.toast.connectFailedTitle'),
        message: res.message,
      });
      if (res.success) resetWizard();
    } catch (err) {
      showToast({
        type: 'error',
        title: t('settings.integrations.booking.toast.connectFailedTitle'),
        message:
          err instanceof Error ? err.message : t('settings.integrations.common.unexpectedError'),
      });
    }
  };

  const handleSync = async () => {
    try {
      const res = await syncBooking.mutateAsync();
      showToast({
        type: res.success ? 'success' : 'error',
        title: res.success
          ? t('settings.integrations.booking.toast.syncedTitle')
          : t('settings.integrations.booking.toast.syncFailedTitle'),
        message: res.success
          ? t('settings.integrations.common.itemsSynced', { items: res.items_synced })
          : (res.error ?? t('settings.integrations.common.syncIncomplete')),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('settings.integrations.booking.toast.syncFailedTitle'),
        message:
          err instanceof Error ? err.message : t('settings.integrations.common.unexpectedError'),
      });
    }
  };

  const handleDisconnect = async () => {
    try {
      const res = await disconnectBooking.mutateAsync();
      showToast({
        type: res.success ? 'success' : 'error',
        title: res.success
          ? t('settings.integrations.booking.toast.disconnectedTitle')
          : t('settings.integrations.booking.toast.disconnectFailedTitle'),
        message: res.message,
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('settings.integrations.booking.toast.disconnectFailedTitle'),
        message:
          err instanceof Error ? err.message : t('settings.integrations.common.unexpectedError'),
      });
    }
  };

  const canSubmit = hotelId.trim() !== '' && username.trim() !== '' && password !== '';
  const conflictCount = conflicts.data?.conflicts_found ?? 0;

  return (
    <section className="rounded-lg border border-gray-200 bg-white p-6 space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <span
              aria-hidden="true"
              className="inline-flex h-8 w-8 items-center justify-center rounded-full bg-blue-600 text-white text-sm font-bold"
            >
              B
            </span>
            {t('settings.integrations.booking.name', { defaultValue: 'Booking.com' })}
          </h2>
          <p className="text-sm text-gray-500">{t('settings.integrations.booking.description')}</p>
        </div>
        <span
          className={`shrink-0 rounded-full px-3 py-1 text-xs font-medium ${
            connected ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-600'
          }`}
        >
          {status.isLoading
            ? t('settings.integrations.status.checking')
            : connected
              ? t('settings.integrations.status.connected')
              : t('settings.integrations.status.notConnected')}
        </span>
      </div>

      {status.isError && (
        <div className="rounded-md bg-red-50 p-3 text-sm text-red-700" role="alert">
          {t('settings.integrations.booking.loadError')}
        </div>
      )}

      {connected && status.data && (
        <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm sm:grid-cols-4">
          <div>
            <dt className="text-gray-500">{t('settings.integrations.booking.hotelId')}</dt>
            <dd className="font-medium text-gray-900">{status.data.hotel_id ?? '—'}</dd>
          </div>
          <div>
            <dt className="text-gray-500">{t('settings.integrations.common.lastSync')}</dt>
            <dd className="font-medium text-gray-900">{formatDate(status.data.last_sync_at)}</dd>
          </div>
          <div>
            <dt className="text-gray-500">{t('settings.integrations.booking.properties')}</dt>
            <dd className="font-medium text-gray-900">{status.data.properties_count}</dd>
          </div>
          <div>
            <dt className="text-gray-500">{t('settings.integrations.common.reservations')}</dt>
            <dd className="font-medium text-gray-900">{status.data.reservations_count}</dd>
          </div>
        </dl>
      )}

      {connected && status.data?.sync_error && (
        <div className="rounded-md bg-red-50 p-3 text-sm text-red-700" role="alert">
          {t('settings.integrations.common.lastSyncError', { error: status.data.sync_error })}
        </div>
      )}

      {connected && conflictCount > 0 && (
        <div className="rounded-md bg-yellow-50 p-3 text-sm text-yellow-800" role="alert">
          {t('settings.integrations.booking.conflicts', { count: conflictCount })}
          {conflicts.data?.truncated ? t('settings.integrations.booking.conflictsTruncated') : ''}
        </div>
      )}

      {/* Property mapping wizard (connect form) */}
      {!connected && showWizard && (
        <form onSubmit={handleConnect} className="space-y-3 rounded-md bg-gray-50 p-4">
          <p className="text-sm font-medium text-gray-700">
            {t('settings.integrations.booking.wizard.title')}
          </p>
          <div>
            <label htmlFor="booking-hotel-id" className="block text-xs font-medium text-gray-600">
              {t('settings.integrations.booking.hotelId')}
            </label>
            <input
              id="booking-hotel-id"
              type="text"
              value={hotelId}
              onChange={(e) => setHotelId(e.target.value)}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
              autoComplete="off"
            />
          </div>
          <div>
            <label htmlFor="booking-username" className="block text-xs font-medium text-gray-600">
              {t('settings.integrations.booking.wizard.username')}
            </label>
            <input
              id="booking-username"
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
              autoComplete="off"
            />
          </div>
          <div>
            <label htmlFor="booking-password" className="block text-xs font-medium text-gray-600">
              {t('settings.integrations.booking.wizard.password')}
            </label>
            <input
              id="booking-password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
              autoComplete="off"
            />
          </div>
          <div className="flex flex-wrap gap-3 pt-1">
            <button
              type="submit"
              disabled={!canSubmit || connectBooking.isPending}
              className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {connectBooking.isPending
                ? t('settings.integrations.booking.wizard.connecting')
                : t('settings.integrations.booking.wizard.connect')}
            </button>
            <button
              type="button"
              onClick={resetWizard}
              className="rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
            >
              {t('settings.integrations.booking.wizard.cancel')}
            </button>
          </div>
        </form>
      )}

      <div className="flex flex-wrap gap-3 pt-2">
        {!connected ? (
          !showWizard && (
            <button
              type="button"
              onClick={() => setShowWizard(true)}
              disabled={!organizationId}
              className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {t('settings.integrations.booking.connect')}
            </button>
          )
        ) : (
          <>
            <button
              type="button"
              onClick={handleSync}
              disabled={syncBooking.isPending}
              className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {syncBooking.isPending
                ? t('settings.integrations.common.syncing')
                : t('settings.integrations.common.syncNow')}
            </button>
            <button
              type="button"
              onClick={handleDisconnect}
              disabled={disconnectBooking.isPending}
              className="rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
            >
              {disconnectBooking.isPending
                ? t('settings.integrations.common.disconnecting')
                : t('settings.integrations.common.disconnect')}
            </button>
          </>
        )}
      </div>
    </section>
  );
};

export const IntegrationsPage: React.FC = () => {
  const { t } = useTranslation();
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
      // The Airbnb OAuth callback is backend-owned: the browser is redirected
      // to `GET /api/v1/integrations/organizations/{org}/airbnb/callback`, which
      // exchanges the code and verifies+consumes the single-use, server-bound
      // CSRF `state` (issued server-side in the connect handler and stored in
      // Redis — see backend routes/integrations/oauth.rs::airbnb_oauth_callback).
      // We therefore must NOT pass a frontend-origin `redirect_uri` (that would
      // land the browser on this page with an unhandled `?code=&state=` and skip
      // the token exchange), and we deliberately do not persist `res.state` —
      // CSRF verification happens on the server, not the client. Omitting
      // `redirect_uri` lets the backend use its configured callback URL.
      const res = await connectAirbnb.mutateAsync({});
      // Backend returns the OAuth authorize URL; hand off to Airbnb.
      window.location.href = res.auth_url;
    } catch (err) {
      showToast({
        type: 'error',
        title: t('settings.integrations.airbnb.toast.connectFailedTitle'),
        message:
          err instanceof Error ? err.message : t('settings.integrations.common.unexpectedError'),
      });
    }
  };

  const handleSync = async () => {
    try {
      const res = await syncAirbnb.mutateAsync();
      showToast({
        type: res.success ? 'success' : 'error',
        title: res.success
          ? t('settings.integrations.airbnb.toast.syncedTitle')
          : t('settings.integrations.airbnb.toast.syncFailedTitle'),
        message: res.success
          ? t('settings.integrations.common.itemsSynced', { items: res.items_synced })
          : (res.error ?? t('settings.integrations.common.syncIncomplete')),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('settings.integrations.airbnb.toast.syncFailedTitle'),
        message:
          err instanceof Error ? err.message : t('settings.integrations.common.unexpectedError'),
      });
    }
  };

  const handleDisconnect = async () => {
    try {
      await disconnectAirbnb.mutateAsync();
      showToast({
        type: 'success',
        title: t('settings.integrations.airbnb.toast.disconnectedTitle'),
        message: t('settings.integrations.airbnb.toast.disconnectedMessage'),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('settings.integrations.airbnb.toast.disconnectFailedTitle'),
        message:
          err instanceof Error ? err.message : t('settings.integrations.common.unexpectedError'),
      });
    }
  };

  return (
    <div className="max-w-4xl mx-auto p-6 space-y-6">
      <header>
        <h1 className="text-2xl font-bold">
          {t('settings.integrations.title', { defaultValue: 'Integrations' })}
        </h1>
        <p className="text-sm" style={{ color: 'var(--color-text-secondary)' }}>
          {t('settings.integrations.subtitle')}
        </p>
      </header>

      {!organizationId && (
        <div
          className="rounded-lg border border-yellow-300 bg-yellow-50 p-4 text-sm text-yellow-800"
          role="alert"
        >
          {t('settings.integrations.noOrganization')}
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
              {t('settings.integrations.airbnb.name', { defaultValue: 'Airbnb' })}
            </h2>
            <p className="text-sm text-gray-500">{t('settings.integrations.airbnb.description')}</p>
          </div>
          <span
            className={`shrink-0 rounded-full px-3 py-1 text-xs font-medium ${
              connected ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-600'
            }`}
          >
            {status.isLoading
              ? t('settings.integrations.status.checking')
              : connected
                ? t('settings.integrations.status.connected')
                : t('settings.integrations.status.notConnected')}
          </span>
        </div>

        {status.isError && (
          <div className="rounded-md bg-red-50 p-3 text-sm text-red-700" role="alert">
            {t('settings.integrations.airbnb.loadError')}
          </div>
        )}

        {connected && status.data && (
          <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm sm:grid-cols-4">
            <div>
              <dt className="text-gray-500">{t('settings.integrations.airbnb.account')}</dt>
              <dd className="font-medium text-gray-900">
                {status.data.external_account_id ?? '—'}
              </dd>
            </div>
            <div>
              <dt className="text-gray-500">{t('settings.integrations.common.lastSync')}</dt>
              <dd className="font-medium text-gray-900">{formatDate(status.data.last_sync_at)}</dd>
            </div>
            <div>
              <dt className="text-gray-500">{t('settings.integrations.airbnb.listings')}</dt>
              <dd className="font-medium text-gray-900">{status.data.listings_count}</dd>
            </div>
            <div>
              <dt className="text-gray-500">{t('settings.integrations.common.reservations')}</dt>
              <dd className="font-medium text-gray-900">{status.data.reservations_count}</dd>
            </div>
          </dl>
        )}

        {connected && status.data?.sync_error && (
          <div className="rounded-md bg-red-50 p-3 text-sm text-red-700" role="alert">
            {t('settings.integrations.common.lastSyncError', { error: status.data.sync_error })}
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
              {connectAirbnb.isPending
                ? t('settings.integrations.airbnb.redirecting')
                : t('settings.integrations.airbnb.connect')}
            </button>
          ) : (
            <>
              <button
                type="button"
                onClick={handleSync}
                disabled={syncAirbnb.isPending}
                className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
              >
                {syncAirbnb.isPending
                  ? t('settings.integrations.common.syncing')
                  : t('settings.integrations.common.syncNow')}
              </button>
              <button
                type="button"
                onClick={handleDisconnect}
                disabled={disconnectAirbnb.isPending}
                className="rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
              >
                {disconnectAirbnb.isPending
                  ? t('settings.integrations.common.disconnecting')
                  : t('settings.integrations.common.disconnect')}
              </button>
            </>
          )}
        </div>

        {connected && (
          <div className="pt-2">
            <h3 className="text-sm font-semibold text-gray-700">
              {t('settings.integrations.airbnb.mappings.title')}
            </h3>
            {listings.isLoading ? (
              <p className="text-sm text-gray-500">
                {t('settings.integrations.airbnb.mappings.loading')}
              </p>
            ) : (listings.data?.length ?? 0) === 0 ? (
              <p className="text-sm text-gray-500">
                {t('settings.integrations.airbnb.mappings.empty')}
              </p>
            ) : (
              <ul className="mt-2 divide-y divide-gray-100 rounded-md border border-gray-100">
                {listings.data?.map((mapping) => (
                  <li
                    key={mapping.id}
                    className="flex items-center justify-between px-3 py-2 text-sm"
                  >
                    <span className="font-medium text-gray-900">{mapping.unit_name}</span>
                    <span className={mapping.is_active ? 'text-green-600' : 'text-gray-400'}>
                      {mapping.is_active
                        ? t('settings.integrations.airbnb.mappings.active')
                        : t('settings.integrations.airbnb.mappings.inactive')}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </section>

      {/* Booking.com channel card (Gap 83-2) */}
      <BookingIntegrationCard organizationId={organizationId} />
    </div>
  );
};

export default IntegrationsPage;
