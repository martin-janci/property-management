/**
 * PlatformConnectionCard Component
 *
 * Displays a platform connection with its sync status. Active connections
 * offer sync / settings / disconnect; a pending one links to the account
 * authorization; a disconnected (inactive) one can be reactivated.
 * Epic 18: Short-Term Rental Integration (Story 18.1)
 */

import { useTranslation } from 'react-i18next';
import type { ConnectionStatus, PlatformConnection, PlatformType } from '../types';

interface PlatformConnectionCardProps {
  connection: PlatformConnection;
  onConnect?: (id: string) => void;
  onDisconnect?: (id: string) => void;
  onSync?: (id: string) => void;
  onSettings?: (id: string) => void;
  isSyncing?: boolean;
}

const platformConfig: Record<
  PlatformType,
  { name: string; logo: string; color: string; bgColor: string; buttonColor: string }
> = {
  airbnb: {
    name: 'Airbnb',
    logo: 'A',
    color: 'text-rose-500',
    bgColor: 'bg-rose-100',
    buttonColor: 'bg-rose-600',
  },
  booking: {
    name: 'Booking.com',
    logo: 'B',
    color: 'text-blue-600',
    bgColor: 'bg-blue-100',
    buttonColor: 'bg-blue-600',
  },
};

const statusConfig: Record<ConnectionStatus, { label: string; color: string; bgColor: string }> = {
  connected: { label: 'connected', color: 'text-green-700', bgColor: 'bg-green-100' },
  disconnected: { label: 'disconnected', color: 'text-gray-700', bgColor: 'bg-gray-100' },
  pending: { label: 'pending', color: 'text-yellow-700', bgColor: 'bg-yellow-100' },
  error: { label: 'error', color: 'text-red-700', bgColor: 'bg-red-100' },
};

/**
 * Relative "last sync" label in the UI language ("5 minutes ago", "pred 5
 * minútami", …) via `Intl.RelativeTimeFormat`; `justNow` covers < 1 minute.
 */
export function formatLastSync(
  dateString: string | undefined,
  language: string,
  justNow: string,
  now: Date = new Date()
): string {
  if (!dateString) return '-';
  const diff = now.getTime() - new Date(dateString).getTime();
  if (!Number.isFinite(diff)) return '-';
  const minutes = Math.floor(diff / (1000 * 60));
  if (minutes < 1) return justNow;
  const rtf = new Intl.RelativeTimeFormat(language, { numeric: 'auto' });
  if (minutes < 60) return rtf.format(-minutes, 'minute');
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return rtf.format(-hours, 'hour');
  return rtf.format(-Math.floor(hours / 24), 'day');
}

export function PlatformConnectionCard({
  connection,
  onConnect,
  onDisconnect,
  onSync,
  onSettings,
  isSyncing,
}: PlatformConnectionCardProps) {
  const { t, i18n } = useTranslation();
  const platform = platformConfig[connection.platform];
  const status = statusConfig[connection.status];
  const isConnected = connection.status === 'connected';
  const hasError = connection.status === 'error';
  const isInactive = connection.status === 'disconnected';
  const isPending = connection.status === 'pending';

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b">
        <div className="flex items-center gap-3">
          <div
            className={`w-10 h-10 rounded-lg flex items-center justify-center text-xl font-bold ${platform.bgColor} ${platform.color}`}
          >
            {platform.logo}
          </div>
          <div>
            <h3 className="font-semibold text-gray-900">{platform.name}</h3>
            {connection.unitName && <p className="text-sm text-gray-500">{connection.unitName}</p>}
          </div>
        </div>
        <span className={`px-2 py-1 text-xs font-medium rounded ${status.bgColor} ${status.color}`}>
          {t(`rentals.connection.status.${status.label}`)}
        </span>
      </div>

      {/* Content */}
      <div className="p-4">
        {/* Connection Details */}
        <div className="space-y-2 text-sm">
          {connection.platformPropertyId && (
            <div className="flex items-center justify-between">
              <span className="text-gray-500">{t('rentals.connection.propertyId')}</span>
              <span className="text-gray-900 font-mono text-xs">
                {connection.platformPropertyId}
              </span>
            </div>
          )}
          <div className="flex items-center justify-between">
            <span className="text-gray-500">{t('rentals.connection.lastSync')}</span>
            <span className="text-gray-900">
              {formatLastSync(
                connection.lastSyncAt,
                i18n.language,
                t('rentals.connection.justNow')
              )}
            </span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-gray-500">{t('rentals.connection.autoSync')}</span>
            <span
              className={`${connection.isAutoSyncEnabled ? 'text-green-600' : 'text-gray-400'}`}
            >
              {connection.isAutoSyncEnabled
                ? t('rentals.connection.enabled')
                : t('rentals.connection.disabled')}
            </span>
          </div>
        </div>

        {/* Error Message */}
        {hasError && connection.syncErrorMessage && (
          <div className="mt-4 p-3 bg-red-50 rounded-md">
            <div className="flex items-start gap-2">
              <svg
                className="w-5 h-5 text-red-400 flex-shrink-0 mt-0.5"
                fill="currentColor"
                viewBox="0 0 20 20"
                aria-hidden="true"
              >
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                  clipRule="evenodd"
                />
              </svg>
              <p className="text-sm text-red-700">{connection.syncErrorMessage}</p>
            </div>
          </div>
        )}

        {isPending && (
          <p className="mt-4 text-xs text-gray-500">{t('rentals.connection.pendingHint')}</p>
        )}

        {/* Sync Status */}
        {isConnected && (
          <div className="mt-4 flex items-center gap-2 text-xs text-gray-500">
            {isSyncing ? (
              <>
                <div className="animate-spin rounded-full h-3 w-3 border-b-2 border-blue-600" />
                <span>{t('rentals.connection.syncing')}</span>
              </>
            ) : (
              <>
                <svg
                  className="w-3 h-3 text-green-500"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                  aria-hidden="true"
                >
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                    clipRule="evenodd"
                  />
                </svg>
                <span>{t('rentals.connection.syncedSuccessfully')}</span>
              </>
            )}
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="px-4 py-3 bg-gray-50 border-t flex items-center justify-between gap-2">
        {isInactive ? (
          <button
            type="button"
            onClick={() => onConnect?.(connection.id)}
            className={`flex-1 px-4 py-2 text-sm font-medium text-white rounded-md ${platform.buttonColor} hover:opacity-90`}
          >
            {t('rentals.connection.reconnect')}
          </button>
        ) : isPending ? (
          <button
            type="button"
            onClick={() => onConnect?.(connection.id)}
            className={`px-3 py-1.5 text-sm font-medium text-white rounded-md ${platform.buttonColor} hover:opacity-90`}
          >
            {t('rentals.connection.connectPlatform', { platform: platform.name })}
          </button>
        ) : (
          <button
            type="button"
            onClick={() => onSync?.(connection.id)}
            disabled={isSyncing}
            className="flex items-center gap-1 text-sm text-blue-600 hover:text-blue-800 disabled:opacity-50"
          >
            <svg
              className={`w-4 h-4 ${isSyncing ? 'animate-spin' : ''}`}
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
              />
            </svg>
            {t('rentals.connection.syncNow')}
          </button>
        )}
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => onSettings?.(connection.id)}
            className="p-2 text-gray-400 hover:text-gray-600 rounded"
            title={t('rentals.connection.settings')}
            aria-label={t('rentals.connection.settingsFor', { platform: platform.name })}
          >
            <svg
              className="w-5 h-5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
              />
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
              />
            </svg>
          </button>
          {!isInactive && (
            <button
              type="button"
              onClick={() => onDisconnect?.(connection.id)}
              className="px-3 py-1 text-sm text-red-600 hover:text-red-800 border border-red-200 rounded-md hover:bg-red-50"
            >
              {t('rentals.connection.disconnect')}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
