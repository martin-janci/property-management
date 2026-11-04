/**
 * Recent activity feed for the resident dashboard.
 *
 * Built from real data only: the newest published announcements and the faults
 * the signed-in user reported. There is no dedicated activity-feed endpoint, so
 * the two existing list endpoints are merged client-side and sorted by date.
 *
 * @module features/dashboard/components/RecentActivity
 */

import { useAnnouncements, useFaults } from '@ppt/api-client';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { useAuth } from '../../../contexts';

const MAX_ITEMS = 5;

interface ActivityEntry {
  id: string;
  icon: string;
  text: string;
  timestamp: string;
  href: string;
}

const RELATIVE_UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['year', 365 * 24 * 60 * 60],
  ['month', 30 * 24 * 60 * 60],
  ['week', 7 * 24 * 60 * 60],
  ['day', 24 * 60 * 60],
  ['hour', 60 * 60],
  ['minute', 60],
];

/** Formats an ISO timestamp as "pred 2 hodinami" / "2 hours ago" in `locale`. */
export function formatRelativeTime(iso: string, locale: string, now: number = Date.now()): string {
  const diffSeconds = (new Date(iso).getTime() - now) / 1000;
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });
  for (const [unit, seconds] of RELATIVE_UNITS) {
    if (Math.abs(diffSeconds) >= seconds) {
      return rtf.format(Math.round(diffSeconds / seconds), unit);
    }
  }
  return rtf.format(0, 'minute');
}

export function RecentActivity() {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();

  const announcements = useAnnouncements({ status: 'published', pageSize: MAX_ITEMS });
  const faults = useFaults({ limit: 20 });

  const entries = useMemo<ActivityEntry[]>(() => {
    const items: ActivityEntry[] = [];
    for (const a of announcements.data?.items ?? []) {
      if (!a.publishedAt) continue;
      items.push({
        id: `announcement-${a.id}`,
        icon: '📢',
        text: t('dashboard.activity.announcementPublished', { title: a.title }),
        timestamp: a.publishedAt,
        href: `/announcements/${a.id}`,
      });
    }
    for (const f of faults.data?.faults ?? []) {
      if (!user || f.reporter_id !== user.id) continue;
      items.push({
        id: `fault-${f.id}`,
        icon: '🔧',
        text: t('dashboard.activity.faultReportedByYou', { title: f.title }),
        timestamp: f.created_at,
        href: `/faults/${f.id}`,
      });
    }
    return items
      .sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
      .slice(0, MAX_ITEMS);
  }, [announcements.data, faults.data, user, t]);

  const isLoading = announcements.isLoading || faults.isLoading;
  const isError = announcements.isError && faults.isError;

  return (
    <section className="resident-page__activity" aria-busy={isLoading}>
      <h2 className="resident-page__activity-title">{t('dashboard.recentActivity')}</h2>
      {isLoading ? (
        <p className="resident-page__activity-empty">{t('dashboard.activity.loading')}</p>
      ) : isError ? (
        <p className="resident-page__activity-empty" role="alert">
          {t('dashboard.activity.loadFailed')}
        </p>
      ) : entries.length === 0 ? (
        <p className="resident-page__activity-empty">{t('dashboard.activity.empty')}</p>
      ) : (
        <ul className="resident-page__activity-list">
          {entries.map((entry) => (
            <li key={entry.id}>
              <Link to={entry.href} className="activity-item">
                <span className="activity-item__icon" aria-hidden="true">
                  {entry.icon}
                </span>
                <div>
                  <p className="activity-item__text">{entry.text}</p>
                  <p className="activity-item__time">
                    <time dateTime={entry.timestamp}>
                      {formatRelativeTime(entry.timestamp, i18n.language)}
                    </time>
                  </p>
                </div>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
