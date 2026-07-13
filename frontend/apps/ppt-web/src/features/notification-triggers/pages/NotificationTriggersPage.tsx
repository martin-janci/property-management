/**
 * NotificationTriggersPage — manage which notification triggers (event types)
 * fire, and on which channels (Story 84.4 — Notification Trigger System, PM gap 84-4).
 *
 * Presentational: query + mutation state is owned by the route wrapper
 * (NotificationTriggersPageRoute), which drives the granular-events hooks. Each
 * trigger row exposes push / email / in-app checkboxes; priority triggers
 * (e.g. emergency alerts) keep the in-app channel forced on server-side, so we
 * render it disabled here to set expectations.
 */

import { useTranslation } from 'react-i18next';
import type {
  CategorySummary,
  EventTriggerPreference,
  EventTriggersResponse,
  TriggerChannel,
} from '../types';

export interface NotificationTriggersPageProps {
  data?: EventTriggersResponse;
  isLoading?: boolean;
  isError?: boolean;
  /** True when the backend rejected the request with 401/403. */
  isForbidden?: boolean;
  onRetry?: () => void;
  /** Event type currently being persisted (disables its row's inputs). */
  savingEventType?: string | null;
  onToggle: (eventType: string, channel: TriggerChannel, enabled: boolean) => void;
  onReset?: () => void;
  isResetting?: boolean;
}

const CHANNELS: TriggerChannel[] = ['push', 'email', 'inApp'];

const CHANNEL_LABEL_KEY: Record<TriggerChannel, { key: string; def: string }> = {
  push: { key: 'notificationTriggers.channelPush', def: 'Push' },
  email: { key: 'notificationTriggers.channelEmail', def: 'Email' },
  inApp: { key: 'notificationTriggers.channelInApp', def: 'In app' },
};

/** Humanise a category/enum key (`in_app` → `In app`). */
function humanise(value: string): string {
  const s = value.replace(/_/g, ' ');
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function channelValue(pref: EventTriggerPreference, channel: TriggerChannel): boolean {
  switch (channel) {
    case 'push':
      return pref.pushEnabled;
    case 'email':
      return pref.emailEnabled;
    case 'inApp':
      return pref.inAppEnabled;
  }
}

export function NotificationTriggersPage({
  data,
  isLoading,
  isError,
  isForbidden,
  onRetry,
  savingEventType,
  onToggle,
  onReset,
  isResetting,
}: NotificationTriggersPageProps) {
  const { t } = useTranslation();

  if (isForbidden) {
    return (
      <div className="max-w-5xl mx-auto px-4 py-8">
        <div
          role="alert"
          className="border border-amber-200 bg-amber-50 text-amber-800 rounded-lg p-6 text-center"
        >
          <p className="font-medium">
            {t('notificationTriggers.forbidden', {
              defaultValue: 'You do not have permission to manage notification triggers.',
            })}
          </p>
        </div>
      </div>
    );
  }

  const preferences: EventTriggerPreference[] = data?.preferences ?? [];
  const categories: CategorySummary[] = data?.categories ?? [];

  // Group triggers by category, preserving the backend category ordering.
  const byCategory = new Map<string, EventTriggerPreference[]>();
  for (const pref of preferences) {
    const list = byCategory.get(pref.category) ?? [];
    list.push(pref);
    byCategory.set(pref.category, list);
  }
  const orderedCategories =
    categories.length > 0 ? categories.map((c) => c.category) : Array.from(byCategory.keys());

  return (
    <div className="max-w-5xl mx-auto px-4 py-8">
      <header className="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">
            {t('notificationTriggers.title', { defaultValue: 'Notification Triggers' })}
          </h1>
          <p className="text-sm text-gray-500">
            {t('notificationTriggers.subtitle', {
              defaultValue:
                'Choose which events notify you, and on which channels (push, email, in-app).',
            })}
          </p>
        </div>
        {onReset && (
          <button
            type="button"
            onClick={onReset}
            disabled={isResetting || isLoading}
            className="px-3 py-2 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50"
          >
            {isResetting
              ? t('notificationTriggers.resetting', { defaultValue: 'Resetting…' })
              : t('notificationTriggers.resetDefaults', { defaultValue: 'Reset to defaults' })}
          </button>
        )}
      </header>

      {/* Loading */}
      {isLoading && (
        <div className="flex justify-center py-16" aria-live="polite" aria-busy="true">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      )}

      {/* Error */}
      {!isLoading && isError && (
        <div
          role="alert"
          className="text-center py-8 border border-red-200 bg-red-50 rounded-lg text-red-700"
        >
          <p className="font-medium">
            {t('notificationTriggers.failedToLoad', {
              defaultValue: 'Failed to load notification triggers',
            })}
          </p>
          {onRetry && (
            <button
              type="button"
              onClick={onRetry}
              className="mt-3 px-3 py-1 border border-red-300 rounded hover:bg-red-100"
            >
              {t('common.retry', { defaultValue: 'Retry' })}
            </button>
          )}
        </div>
      )}

      {/* Content */}
      {!isLoading && !isError && data && (
        <>
          {preferences.length === 0 ? (
            <div className="text-center py-12 text-gray-500">
              {t('notificationTriggers.empty', {
                defaultValue: 'No notification triggers are available for your account.',
              })}
            </div>
          ) : (
            <div className="space-y-8">
              {orderedCategories.map((category) => {
                const rows = byCategory.get(category) ?? [];
                if (rows.length === 0) {
                  return null;
                }
                const summary = categories.find((c) => c.category === category);
                return (
                  <section
                    key={category}
                    className="border border-gray-200 rounded-lg overflow-hidden"
                  >
                    <div className="flex items-center justify-between bg-gray-50 px-4 py-3 border-b border-gray-200">
                      <h2 className="font-semibold text-gray-800">
                        {summary?.displayName ? humanise(summary.displayName) : humanise(category)}
                      </h2>
                      {summary && (
                        <span className="text-xs text-gray-500 tabular-nums">
                          {t('notificationTriggers.enabledCount', {
                            defaultValue: '{{enabled}} of {{total}} enabled',
                            enabled: summary.enabledEvents,
                            total: summary.totalEvents,
                          })}
                        </span>
                      )}
                    </div>
                    <div className="overflow-x-auto">
                      <table className="min-w-full text-sm">
                        <caption className="sr-only">
                          {t('notificationTriggers.tableCaption', {
                            defaultValue: 'Notification channels per trigger',
                          })}
                        </caption>
                        <thead className="text-gray-600">
                          <tr>
                            <th scope="col" className="px-4 py-2 text-left font-semibold">
                              {t('notificationTriggers.colTrigger', { defaultValue: 'Trigger' })}
                            </th>
                            {CHANNELS.map((channel) => (
                              <th
                                key={channel}
                                scope="col"
                                className="px-4 py-2 text-center font-semibold w-24"
                              >
                                {t(CHANNEL_LABEL_KEY[channel].key, {
                                  defaultValue: CHANNEL_LABEL_KEY[channel].def,
                                })}
                              </th>
                            ))}
                          </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-100">
                          {rows.map((pref) => {
                            const rowSaving = savingEventType === pref.eventType;
                            return (
                              <tr key={pref.eventType} className="text-gray-800">
                                <td className="px-4 py-3">
                                  <div className="font-medium flex items-center gap-2">
                                    {humanise(pref.displayName || pref.eventType)}
                                    {pref.isPriority && (
                                      <span className="inline-block text-[10px] uppercase tracking-wide font-semibold text-red-700 bg-red-50 border border-red-200 rounded px-1.5 py-0.5">
                                        {t('notificationTriggers.priorityBadge', {
                                          defaultValue: 'Priority',
                                        })}
                                      </span>
                                    )}
                                  </div>
                                  {pref.description && (
                                    <p className="text-xs text-gray-500 mt-0.5">
                                      {pref.description}
                                    </p>
                                  )}
                                </td>
                                {CHANNELS.map((channel) => {
                                  const checked = channelValue(pref, channel);
                                  // Priority triggers keep in-app forced on by the
                                  // backend; reflect that with a disabled, checked box.
                                  const forced = pref.isPriority && channel === 'inApp';
                                  const labelText = t(CHANNEL_LABEL_KEY[channel].key, {
                                    defaultValue: CHANNEL_LABEL_KEY[channel].def,
                                  });
                                  return (
                                    <td key={channel} className="px-4 py-3 text-center">
                                      <input
                                        type="checkbox"
                                        className="h-4 w-4 accent-blue-600 disabled:opacity-40"
                                        checked={forced ? true : checked}
                                        disabled={rowSaving || forced}
                                        aria-label={`${humanise(
                                          pref.displayName || pref.eventType
                                        )} — ${labelText}`}
                                        onChange={(e) =>
                                          onToggle(pref.eventType, channel, e.target.checked)
                                        }
                                      />
                                    </td>
                                  );
                                })}
                              </tr>
                            );
                          })}
                        </tbody>
                      </table>
                    </div>
                  </section>
                );
              })}
            </div>
          )}
        </>
      )}
    </div>
  );
}
