/**
 * ConnectionSettingsDialog - edit the sync settings of one platform connection.
 * Epic 18: Short-Term Rental Integration (Story 18.1)
 *
 * Loads `GET /rentals/connections/{id}` (through `onLoad`), edits what
 * `PUT /rentals/connections/{id}` accepts (listing id + URL, calendar sync,
 * sync interval, blocking the other platforms) and offers removing the
 * connection (`DELETE`) behind the app's ConfirmationDialog.
 */

import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ConfirmationDialog } from '../../../components/ConfirmationDialog';
import { isValidListingUrl, SYNC_INTERVAL_OPTIONS } from '../mappers';
import type { ConnectionSettings, PlatformConnection } from '../types';

export interface ConnectionSettingsDialogProps {
  connection: PlatformConnection;
  platformName: string;
  onLoad: (id: string) => Promise<ConnectionSettings>;
  onSave: (id: string, next: ConnectionSettings, previous: ConnectionSettings) => Promise<void>;
  onRemove: (id: string) => Promise<void>;
  onClose: () => void;
}

type LoadState =
  | { kind: 'loading' }
  | { kind: 'error' }
  | { kind: 'ready'; initial: ConnectionSettings };

export function ConnectionSettingsDialog({
  connection,
  platformName,
  onLoad,
  onSave,
  onRemove,
  onClose,
}: ConnectionSettingsDialogProps) {
  const { t } = useTranslation();
  const [state, setState] = useState<LoadState>({ kind: 'loading' });
  const [form, setForm] = useState<ConnectionSettings | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const [isRemoving, setIsRemoving] = useState(false);
  const [reloadKey, setReloadKey] = useState(0);
  // Keep the latest loader without re-running the load on every parent render.
  const onLoadRef = useRef(onLoad);
  onLoadRef.current = onLoad;

  // biome-ignore lint/correctness/useExhaustiveDependencies: reloadKey re-runs the load on retry
  useEffect(() => {
    let cancelled = false;
    setState({ kind: 'loading' });
    onLoadRef.current(connection.id).then(
      (settings) => {
        if (cancelled) return;
        setState({ kind: 'ready', initial: settings });
        setForm(settings);
      },
      () => {
        if (!cancelled) setState({ kind: 'error' });
      }
    );
    return () => {
      cancelled = true;
    };
  }, [connection.id, reloadKey]);

  const urlInvalid = form ? !isValidListingUrl(form.externalListingUrl) : false;
  const intervals =
    form && !SYNC_INTERVAL_OPTIONS.includes(form.syncIntervalMinutes as never)
      ? [...SYNC_INTERVAL_OPTIONS, form.syncIntervalMinutes].sort((a, b) => a - b)
      : [...SYNC_INTERVAL_OPTIONS];

  const handleSave = async () => {
    if (!form || state.kind !== 'ready' || urlInvalid) return;
    setIsSaving(true);
    setSaveError(false);
    try {
      await onSave(connection.id, form, state.initial);
      onClose();
    } catch {
      setSaveError(true);
    } finally {
      setIsSaving(false);
    }
  };

  const handleRemove = async () => {
    setIsRemoving(true);
    try {
      await onRemove(connection.id);
      setConfirmRemove(false);
      onClose();
    } catch {
      setConfirmRemove(false);
    } finally {
      setIsRemoving(false);
    }
  };

  const intervalLabel = (minutes: number) =>
    minutes % 1440 === 0
      ? t('rentals.connections.settings.intervalDays', { count: minutes / 1440 })
      : minutes % 60 === 0
        ? t('rentals.connections.settings.intervalHours', { count: minutes / 60 })
        : t('rentals.connections.settings.intervalMinutes', { count: minutes });

  if (confirmRemove) {
    return (
      <ConfirmationDialog
        isOpen
        title={t('rentals.connections.removeTitle')}
        message={t('rentals.connections.removeMessage', {
          platform: platformName,
          unit: connection.unitName ?? '',
        })}
        confirmLabel={t('rentals.connections.removeConfirm')}
        cancelLabel={t('common.cancel')}
        variant="danger"
        isLoading={isRemoving}
        onConfirm={handleRemove}
        onCancel={() => setConfirmRemove(false)}
      />
    );
  }

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="fixed inset-0 bg-black/25" aria-hidden="true" onClick={onClose} />
        <div
          className="relative bg-white rounded-lg shadow-xl max-w-lg w-full p-6"
          role="dialog"
          aria-modal="true"
          aria-labelledby="connection-settings-title"
          onKeyDown={(e) => {
            if (e.key === 'Escape' && !isSaving) onClose();
          }}
        >
          <h3 id="connection-settings-title" className="text-lg font-semibold text-gray-900">
            {t('rentals.connections.settings.title', { platform: platformName })}
          </h3>
          {connection.unitName && (
            <p className="mt-1 text-sm text-gray-500">{connection.unitName}</p>
          )}

          {state.kind === 'loading' && (
            <div className="flex items-center justify-center h-40" data-testid="settings-loading">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
            </div>
          )}

          {state.kind === 'error' && (
            <div className="mt-6 p-4 bg-red-50 rounded-md text-sm text-red-700" role="alert">
              <p>{t('rentals.connections.settings.loadFailed')}</p>
              <button
                type="button"
                onClick={() => setReloadKey((k) => k + 1)}
                className="mt-2 font-medium underline"
              >
                {t('common.retry')}
              </button>
            </div>
          )}

          {state.kind === 'ready' && form && (
            <div className="mt-6 space-y-4">
              <div>
                <label
                  htmlFor="conn-property-id"
                  className="block text-sm font-medium text-gray-700"
                >
                  {t('rentals.connections.settings.propertyId')}
                </label>
                <input
                  id="conn-property-id"
                  type="text"
                  value={form.externalPropertyId}
                  onChange={(e) => setForm({ ...form, externalPropertyId: e.target.value })}
                  className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md text-sm font-mono"
                />
                <p className="mt-1 text-xs text-gray-500">
                  {t('rentals.connections.settings.propertyIdHelp', { platform: platformName })}
                </p>
              </div>

              <div>
                <label
                  htmlFor="conn-listing-url"
                  className="block text-sm font-medium text-gray-700"
                >
                  {t('rentals.connections.settings.listingUrl')}
                </label>
                <input
                  id="conn-listing-url"
                  type="url"
                  inputMode="url"
                  placeholder="https://"
                  value={form.externalListingUrl}
                  onChange={(e) => setForm({ ...form, externalListingUrl: e.target.value })}
                  aria-invalid={urlInvalid}
                  className={`mt-1 block w-full px-3 py-2 border rounded-md text-sm ${
                    urlInvalid ? 'border-red-400' : 'border-gray-300'
                  }`}
                />
                {urlInvalid && (
                  <p className="mt-1 text-xs text-red-600">
                    {t('rentals.connections.settings.invalidUrl')}
                  </p>
                )}
              </div>

              <label className="flex items-start gap-3">
                <input
                  type="checkbox"
                  checked={form.syncCalendar}
                  onChange={(e) => setForm({ ...form, syncCalendar: e.target.checked })}
                  className="mt-1 h-4 w-4 rounded border-gray-300 text-blue-600"
                />
                <span>
                  <span className="block text-sm font-medium text-gray-700">
                    {t('rentals.connections.settings.syncCalendar')}
                  </span>
                  <span className="block text-xs text-gray-500">
                    {t('rentals.connections.settings.syncCalendarHelp')}
                  </span>
                </span>
              </label>

              <div>
                <label
                  htmlFor="conn-sync-interval"
                  className="block text-sm font-medium text-gray-700"
                >
                  {t('rentals.connections.settings.syncInterval')}
                </label>
                <select
                  id="conn-sync-interval"
                  value={form.syncIntervalMinutes}
                  disabled={!form.syncCalendar}
                  onChange={(e) =>
                    setForm({ ...form, syncIntervalMinutes: Number(e.target.value) })
                  }
                  className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md text-sm disabled:bg-gray-100"
                >
                  {intervals.map((m) => (
                    <option key={m} value={m}>
                      {intervalLabel(m)}
                    </option>
                  ))}
                </select>
              </div>

              <label className="flex items-start gap-3">
                <input
                  type="checkbox"
                  checked={form.blockOtherPlatforms}
                  onChange={(e) => setForm({ ...form, blockOtherPlatforms: e.target.checked })}
                  className="mt-1 h-4 w-4 rounded border-gray-300 text-blue-600"
                />
                <span>
                  <span className="block text-sm font-medium text-gray-700">
                    {t('rentals.connections.settings.blockOtherPlatforms')}
                  </span>
                  <span className="block text-xs text-gray-500">
                    {t('rentals.connections.settings.blockOtherPlatformsHelp')}
                  </span>
                </span>
              </label>

              {saveError && (
                <p className="text-sm text-red-600" role="alert">
                  {t('rentals.connections.settings.saveFailed')}
                </p>
              )}
            </div>
          )}

          <div className="mt-6 flex flex-wrap items-center justify-between gap-3">
            <button
              type="button"
              onClick={() => setConfirmRemove(true)}
              disabled={isSaving}
              className="px-3 py-2 text-sm font-medium text-red-600 hover:text-red-800 disabled:opacity-50"
            >
              {t('rentals.connections.removeConnection')}
            </button>
            <div className="flex items-center gap-3">
              <button
                type="button"
                onClick={onClose}
                disabled={isSaving}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                onClick={handleSave}
                disabled={state.kind !== 'ready' || isSaving || urlInvalid}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSaving ? t('common.saving') : t('common.save')}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
