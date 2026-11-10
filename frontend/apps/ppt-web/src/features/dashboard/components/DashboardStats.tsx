/**
 * DashboardStats section — the summary tiles at the top of the manager
 * dashboard, mounted by the layout registry as a discrete section.
 *
 * Every figure comes from a live endpoint:
 *  - pending faults  — `new` bucket of GET /faults/statistics
 *  - active votes    — GET /votes?status=active
 *  - unread messages — GET /messages/unread-count
 *
 * A tile shows "–" while its query is loading or failed rather than a made-up
 * number. There is no org-wide "pending approvals" count in the API, so that
 * tile is not shown (the Action Queue lists the approvals themselves).
 *
 * @module features/dashboard/components/DashboardStats
 */

import { useFaultStatistics, useVotes } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import type { SectionProps } from '../../layout/registry';
import { useUnreadCount } from '../../messaging/hooks/useMessaging';

interface QuickStatProps {
  label: string;
  value: number | undefined;
  color: 'red' | 'orange' | 'blue' | 'green' | 'gray';
}

function QuickStat({ label, value, color }: QuickStatProps) {
  return (
    <div className={`quick-stat quick-stat--${color}`}>
      <div className="quick-stat__row">
        <span className="quick-stat__value">{value ?? '–'}</span>
      </div>
      <p className="quick-stat__label">{label}</p>
    </div>
  );
}

/** Renders the dashboard summary stats. Implements SectionProps so it can
 *  be mounted directly by LayoutRenderer. */
export function DashboardStats({ mode: _mode, props: _props }: SectionProps) {
  const { t } = useTranslation();
  const faultStats = useFaultStatistics();
  const activeVotes = useVotes({ status: 'active' });
  const unread = useUnreadCount();

  const pendingFaults = faultStats.data
    ? (faultStats.data.by_status.find((s) => s.status === 'new')?.count ?? 0)
    : undefined;

  return (
    <div className="dashboard-page__stats">
      <QuickStat label={t('dashboard.stats.pendingFaults')} value={pendingFaults} color="red" />
      <QuickStat
        label={t('dashboard.stats.activeVotes')}
        value={activeVotes.data?.length}
        color="blue"
      />
      <QuickStat
        label={t('dashboard.stats.unreadMessages')}
        value={unread.data?.unreadCount}
        color="gray"
      />
    </div>
  );
}
