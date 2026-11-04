/**
 * UnreadBadge Component (Story 6.2)
 *
 * Displays the count of unread announcements as a badge.
 */

import { useTranslation } from 'react-i18next';

interface UnreadBadgeProps {
  count: number;
  className?: string;
}

export function UnreadBadge({ count, className = '' }: UnreadBadgeProps) {
  const { t } = useTranslation();
  if (count <= 0) {
    return null;
  }

  const displayCount = count > 99 ? '99+' : count.toString();

  return (
    <span
      className={`inline-flex items-center justify-center min-w-[1.25rem] h-5 px-1.5 text-xs font-medium text-white bg-red-500 rounded-full ${className}`}
      aria-label={t('announcements.unreadCount', { count })}
    >
      {displayCount}
    </span>
  );
}
