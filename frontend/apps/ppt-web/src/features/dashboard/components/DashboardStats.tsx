/**
 * DashboardStats section — the four QuickStat tiles extracted from
 * ManagerDashboardPage so that the layout registry can render them as a
 * discrete section.
 *
 * @module features/dashboard/components/DashboardStats
 */

import { useTranslation } from 'react-i18next';
import type { SectionProps } from '../../layout/registry';

interface QuickStatProps {
  label: string;
  value: string;
  trend: 'up' | 'down' | 'neutral';
  color: 'red' | 'orange' | 'blue' | 'green' | 'gray';
}

const trendIcons: Record<QuickStatProps['trend'], string> = {
  up: '↑',
  down: '↓',
  neutral: '→',
};

function QuickStat({ label, value, trend, color }: QuickStatProps) {
  return (
    <div className={`quick-stat quick-stat--${color}`}>
      <div className="quick-stat__row">
        <span className="quick-stat__value">{value}</span>
        <span className="quick-stat__trend" aria-hidden="true">
          {trendIcons[trend]}
        </span>
      </div>
      <p className="quick-stat__label">{label}</p>
    </div>
  );
}

/** Renders the four dashboard summary stats. Implements SectionProps so it can
 *  be mounted directly by LayoutRenderer. */
// biome-ignore lint/correctness/noUnusedVariables: mode/props are part of SectionProps contract
export function DashboardStats({ mode: _mode, props: _props }: SectionProps) {
  const { t } = useTranslation();

  return (
    <div className="dashboard-page__stats">
      <QuickStat label={t('dashboard.stats.pendingFaults')} value="3" trend="up" color="red" />
      <QuickStat
        label={t('dashboard.stats.pendingApprovals')}
        value="2"
        trend="neutral"
        color="orange"
      />
      <QuickStat label={t('dashboard.stats.activeVotes')} value="1" trend="down" color="blue" />
      <QuickStat label={t('dashboard.stats.unreadMessages')} value="5" trend="up" color="gray" />
    </div>
  );
}
