/**
 * Resident dashboard page with action-first UX pattern.
 * Shows a prioritized queue of items needing resident attention.
 * Designed for 60-second task completion. Recent activity comes from real
 * announcements and the user's own fault reports (see RecentActivity).
 *
 * @module features/dashboard/pages/ResidentDashboardPage
 */

import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { ActionQueue } from '../components/ActionQueue';
import { RecentActivity } from '../components/RecentActivity';
import type { ActionButton, ActionItem } from '../hooks/useActionQueue';
import './ResidentDashboardPage.css';

interface ResidentDashboardPageProps {
  onItemAction?: (itemId: string, action: ActionButton['action'], item: ActionItem) => void;
}

export function ResidentDashboardPage({ onItemAction }: ResidentDashboardPageProps) {
  const { t } = useTranslation();

  return (
    <div className="resident-page">
      <header className="resident-page__header">
        <h1 className="resident-page__title">{t('dashboard.residentDashboard')}</h1>
        <p className="resident-page__subtitle">{t('dashboard.residentWelcome')}</p>
      </header>

      <div className="resident-page__banner">
        <span className="resident-page__banner-icon" aria-hidden="true">
          ⚡
        </span>
        <div>
          <h2 className="resident-page__banner-title">{t('dashboard.quickTasks')}</h2>
          <p className="resident-page__banner-subtitle">{t('dashboard.completeIn60Seconds')}</p>
        </div>
      </div>

      <nav className="resident-page__quick-links">
        <Link to="/my-unit" className="resident-page__quick-link">
          {t('dashboard.viewMyUnit')}
        </Link>
      </nav>

      <ActionQueue userRole="resident" onItemAction={onItemAction} />

      <RecentActivity />
    </div>
  );
}
