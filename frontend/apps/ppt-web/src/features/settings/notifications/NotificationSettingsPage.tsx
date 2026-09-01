/**
 * NotificationSettingsPage Component (Epic 8A, Story 8A.1)
 *
 * Settings page for managing notification channel preferences.
 * Auth is injected via the global token provider — no props required.
 */

import type { NotificationChannel } from '@ppt/api-client';
import {
  ConfirmationRequiredError,
  useNotificationPreferences,
  useUpdateNotificationPreference,
} from '@ppt/api-client';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ChannelToggle, DisableAllWarningDialog } from './components';

export function NotificationSettingsPage() {
  const { t } = useTranslation();
  const [showWarningDialog, setShowWarningDialog] = useState(false);
  const [pendingChannel, setPendingChannel] = useState<NotificationChannel | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);

  const { data, isLoading, error } = useNotificationPreferences();
  const updatePreference = useUpdateNotificationPreference();

  const handleToggle = useCallback(
    async (channel: NotificationChannel, enabled: boolean) => {
      setUpdateError(null);
      try {
        await updatePreference.mutateAsync({
          channel,
          request: { enabled },
        });
      } catch (err) {
        if (err instanceof ConfirmationRequiredError) {
          // Show confirmation dialog before disabling the last active channel.
          setPendingChannel(channel);
          setShowWarningDialog(true);
        } else {
          setUpdateError(t('settings.notifications.updateError'));
        }
      }
    },
    [updatePreference, t]
  );

  const handleConfirmDisableAll = useCallback(async () => {
    if (!pendingChannel) return;

    setUpdateError(null);
    try {
      await updatePreference.mutateAsync({
        channel: pendingChannel,
        request: { enabled: false, confirmDisableAll: true },
      });
    } catch (_err) {
      setUpdateError(t('settings.notifications.disableError'));
    } finally {
      setShowWarningDialog(false);
      setPendingChannel(null);
    }
  }, [pendingChannel, updatePreference, t]);

  const handleCancelDisableAll = useCallback(() => {
    setShowWarningDialog(false);
    setPendingChannel(null);
  }, []);

  if (isLoading) {
    return (
      <div className="max-w-2xl mx-auto p-6">
        <div className="animate-pulse">
          <div className="h-8 bg-gray-200 rounded w-1/3 mb-6" />
          <div className="space-y-4">
            <div className="h-16 bg-gray-100 rounded" />
            <div className="h-16 bg-gray-100 rounded" />
            <div className="h-16 bg-gray-100 rounded" />
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-2xl mx-auto p-6">
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-700">{t('settings.notifications.loadError')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-2xl mx-auto p-6">
      <h1 className="text-2xl font-bold text-gray-900 mb-2">{t('settings.notifications.title')}</h1>
      <p className="text-gray-600 mb-6">{t('settings.notifications.subtitle')}</p>

      {/* Update error alert */}
      {updateError && (
        <div className="mb-6 bg-red-50 border border-red-200 rounded-lg p-4">
          <div className="flex">
            <div className="flex-shrink-0">
              <svg
                className="h-5 w-5 text-red-400"
                viewBox="0 0 20 20"
                fill="currentColor"
                aria-hidden="true"
              >
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.28 7.22a.75.75 0 00-1.06 1.06L8.94 10l-1.72 1.72a.75.75 0 101.06 1.06L10 11.06l1.72 1.72a.75.75 0 101.06-1.06L11.06 10l1.72-1.72a.75.75 0 00-1.06-1.06L10 8.94 8.28 7.22z"
                  clipRule="evenodd"
                />
              </svg>
            </div>
            <div className="ml-3 flex-1">
              <p className="text-sm text-red-700">{updateError}</p>
            </div>
            <button
              type="button"
              onClick={() => setUpdateError(null)}
              className="ml-3 inline-flex text-red-400 hover:text-red-500"
            >
              <span className="sr-only">{t('settings.notifications.dismiss')}</span>
              <svg className="h-5 w-5" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
                <path d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z" />
              </svg>
            </button>
          </div>
        </div>
      )}

      {/* All disabled warning */}
      {data?.allDisabledWarning && (
        <div className="mb-6 bg-amber-50 border border-amber-200 rounded-lg p-4">
          <div className="flex">
            <div className="flex-shrink-0">
              <svg
                className="h-5 w-5 text-amber-400"
                viewBox="0 0 20 20"
                fill="currentColor"
                aria-hidden="true"
              >
                <path
                  fillRule="evenodd"
                  d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.17 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625L8.485 2.495zM10 5a.75.75 0 01.75.75v3.5a.75.75 0 01-1.5 0v-3.5A.75.75 0 0110 5zm0 9a1 1 0 100-2 1 1 0 000 2z"
                  clipRule="evenodd"
                />
              </svg>
            </div>
            <div className="ml-3">
              <p className="text-sm text-amber-700">{data.allDisabledWarning}</p>
            </div>
          </div>
        </div>
      )}

      {/* Channel toggles */}
      <div className="bg-white rounded-lg border border-gray-200 shadow-sm">
        <div className="px-4">
          {data?.preferences.map((pref) => (
            <ChannelToggle
              key={pref.channel}
              channel={pref.channel}
              enabled={pref.enabled}
              loading={updatePreference.isPending}
              onToggle={(enabled) => handleToggle(pref.channel, enabled)}
            />
          ))}
        </div>
      </div>

      {/* Last updated info */}
      {data?.preferences && data.preferences.length > 0 && (
        <p className="mt-4 text-xs text-gray-500">
          {t('settings.notifications.lastUpdated', {
            when: new Date(
              Math.max(...data.preferences.map((p) => new Date(p.updatedAt).getTime()))
            ).toLocaleString(),
          })}
        </p>
      )}

      {/* Confirmation dialog */}
      <DisableAllWarningDialog
        isOpen={showWarningDialog}
        onConfirm={handleConfirmDisableAll}
        onCancel={handleCancelDisableAll}
      />
    </div>
  );
}
