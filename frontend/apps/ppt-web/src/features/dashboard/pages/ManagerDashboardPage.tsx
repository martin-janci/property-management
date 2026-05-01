/**
 * Manager dashboard page with action-first UX pattern.
 * Shows a prioritized queue of items needing manager attention.
 *
 * @module features/dashboard/pages/ManagerDashboardPage
 */

import { useTranslation } from 'react-i18next';
import { ActionQueue } from '../components/ActionQueue';
import type { ActionButton, ActionItem } from '../hooks/useActionQueue';
import './ManagerDashboardPage.css';

interface ManagerDashboardPageProps {
  onItemAction?: (itemId: string, action: ActionButton['action'], item: ActionItem) => void;
}

export function ManagerDashboardPage({ onItemAction }: ManagerDashboardPageProps) {
  const { t } = useTranslation();

  return (
    <div className="dashboard-page">
      <header className="dashboard-page__header">
        <h1 className="dashboard-page__title">{t('dashboard.managerDashboard')}</h1>
        <p className="dashboard-page__subtitle">{t('dashboard.managerWelcome')}</p>
      </header>

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

      <ActionQueue userRole="manager" onItemAction={onItemAction} />
    </div>
  );
}

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
        <span className="quick-stat__trend" aria-hidden="true">{trendIcons[trend]}</span>
      </div>
      <p className="quick-stat__label">{label}</p>
    </div>
  );
}
