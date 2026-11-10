/**
 * NotificationInboxPage — the signed-in user's in-app notifications
 * (gap audit R6 P1-8).
 *
 * In-app notifications have always been persisted by the backend
 * NotificationService into notification groups, but nothing in ppt-web read
 * them, so a notification missed in the toast was gone. This page lists the
 * groups (newest first) with their latest entries and lets the user mark one
 * or all as read, or remove a group.
 */
import type { InboxGroupWithNotifications } from '@ppt/api-client';
import {
  useDeleteInboxGroup,
  useMarkAllInboxRead,
  useMarkInboxGroupRead,
  useNotificationInbox,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

function formatWhen(iso: string, locale: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return d.toLocaleString(locale, { dateStyle: 'medium', timeStyle: 'short' });
}

function InboxGroupCard({
  item,
  locale,
  onRead,
  onDelete,
  busy,
}: {
  item: InboxGroupWithNotifications;
  locale: string;
  onRead: (id: string) => void;
  onDelete: (id: string) => void;
  busy: boolean;
}) {
  const { t } = useTranslation();
  const { group, notifications } = item;
  const unread = !group.isRead;
  return (
    <li
      className={`border rounded-lg p-4 ${unread ? 'border-blue-200 bg-blue-50' : 'border-gray-200 bg-white'}`}
      data-testid="inbox-group"
    >
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <h2 className="font-semibold text-gray-900 break-words">
            {unread && (
              <span className="sr-only">
                {t('shell.inbox.unreadMarker', { defaultValue: 'Unread:' })}{' '}
              </span>
            )}
            {group.title}
          </h2>
          <p className="text-xs text-gray-500">
            {formatWhen(group.latestNotificationAt, locale)}
            {group.notificationCount > 1 &&
              ` · ${t('shell.inbox.count', {
                count: group.notificationCount,
                defaultValue: '{{count}} notifications',
              })}`}
          </p>
        </div>
        <div className="flex gap-2">
          {unread && (
            <button
              type="button"
              disabled={busy}
              onClick={() => onRead(group.id)}
              className="px-2 py-1 text-sm border border-gray-300 rounded hover:bg-gray-50 disabled:opacity-50"
            >
              {t('shell.inbox.markRead', { defaultValue: 'Mark as read' })}
            </button>
          )}
          <button
            type="button"
            disabled={busy}
            onClick={() => onDelete(group.id)}
            className="px-2 py-1 text-sm border border-gray-300 rounded hover:bg-gray-50 disabled:opacity-50"
          >
            {t('shell.inbox.remove', { defaultValue: 'Remove' })}
          </button>
        </div>
      </div>
      {notifications.length > 0 && (
        <ul className="mt-3 space-y-2">
          {notifications.map((n) => (
            <li key={n.id} className="text-sm text-gray-700">
              {n.title !== group.title && <span className="font-medium">{n.title}</span>}
              {n.body && <p className="whitespace-pre-line">{n.body}</p>}
            </li>
          ))}
        </ul>
      )}
    </li>
  );
}

export function NotificationInboxPage() {
  const { t, i18n } = useTranslation();
  const [includeRead, setIncludeRead] = useState(false);
  const { data, isLoading, error, refetch } = useNotificationInbox({ includeRead, limit: 50 });
  const markRead = useMarkInboxGroupRead();
  const markAll = useMarkAllInboxRead();
  const remove = useDeleteInboxGroup();
  const busy = markRead.isPending || markAll.isPending || remove.isPending;
  const groups = data?.groups ?? [];

  return (
    <div className="max-w-3xl mx-auto px-4 py-8">
      <header className="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">
            {t('shell.inbox.title', { defaultValue: 'Notifications' })}
          </h1>
          <p className="text-sm text-gray-500">
            {t('shell.inbox.unreadSummary', {
              count: data?.totalUnread ?? 0,
              defaultValue: '{{count}} unread',
            })}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <label className="flex items-center gap-2 text-sm text-gray-700">
            <input
              type="checkbox"
              checked={includeRead}
              onChange={(e) => setIncludeRead(e.target.checked)}
            />
            {t('shell.inbox.showRead', { defaultValue: 'Show read' })}
          </label>
          <button
            type="button"
            disabled={busy || !data || data.totalUnread === 0}
            onClick={() => markAll.mutate()}
            className="px-3 py-2 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50"
          >
            {t('shell.inbox.markAllRead', { defaultValue: 'Mark all as read' })}
          </button>
        </div>
      </header>

      {isLoading && (
        <div className="flex justify-center py-16" aria-live="polite" aria-busy="true">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      )}

      {!isLoading && error && (
        <div
          role="alert"
          className="text-center py-8 border border-red-200 bg-red-50 rounded-lg text-red-700"
        >
          <p className="font-medium">
            {t('shell.inbox.failedToLoad', { defaultValue: 'Failed to load notifications' })}
          </p>
          <button
            type="button"
            onClick={() => void refetch()}
            className="mt-3 px-3 py-1 border border-red-300 rounded hover:bg-red-100"
          >
            {t('common.retry', { defaultValue: 'Retry' })}
          </button>
        </div>
      )}

      {!isLoading && !error && groups.length === 0 && (
        <p className="text-center py-12 text-gray-500">
          {includeRead
            ? t('shell.inbox.emptyAll', { defaultValue: 'You have no notifications.' })
            : t('shell.inbox.empty', { defaultValue: 'You are all caught up.' })}
        </p>
      )}

      {!isLoading && !error && groups.length > 0 && (
        <ul className="space-y-3">
          {groups.map((item) => (
            <InboxGroupCard
              key={item.group.id}
              item={item}
              locale={i18n.language}
              busy={busy}
              onRead={(id) => markRead.mutate(id)}
              onDelete={(id) => remove.mutate(id)}
            />
          ))}
        </ul>
      )}
    </div>
  );
}
