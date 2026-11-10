/**
 * Top-bar bell linking to the in-app notification inbox, with an unread badge
 * (gap audit R6 P1-8). The count is polled; it is a hint, not a source of truth.
 */
import { useUnreadNotificationCount } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';

export function NotificationBell() {
  const { t } = useTranslation();
  const { data: unread = 0 } = useUnreadNotificationCount();
  const label =
    unread > 0
      ? t('shell.inbox.bellUnread', {
          count: unread,
          defaultValue: 'Notifications ({{count}} unread)',
        })
      : t('shell.inbox.title', { defaultValue: 'Notifications' });

  return (
    <Link to="/notifications" className="shell-bell" aria-label={label} title={label}>
      <span aria-hidden="true">🔔</span>
      {unread > 0 && (
        <span className="shell-bell__badge" aria-hidden="true">
          {unread > 99 ? '99+' : unread}
        </span>
      )}
    </Link>
  );
}
