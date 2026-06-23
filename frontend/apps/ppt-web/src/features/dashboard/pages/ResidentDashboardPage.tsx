/**
 * Resident dashboard page with action-first UX pattern.
 * Shows a prioritized queue of items needing resident attention.
 * Designed for 60-second task completion.
 *
 * @module features/dashboard/pages/ResidentDashboardPage
 */

import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { ActionQueue } from '../components/ActionQueue';
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
          {t('dashboard.viewMyUnit', 'View my unit')}
        </Link>
      </nav>

      <ActionQueue userRole="resident" onItemAction={onItemAction} />

      <section className="resident-page__activity">
        <h2 className="resident-page__activity-title">{t('dashboard.recentActivity')}</h2>
        <div className="resident-page__activity-list">
          <ActivityItem
            icon="✅"
            text={t('dashboard.activity.meterReadingSubmitted')}
            time="2 hours ago"
          />
          <ActivityItem icon="🗳️" text={t('dashboard.activity.voteSubmitted')} time="Yesterday" />
          <ActivityItem icon="🔧" text={t('dashboard.activity.faultReported')} time="3 days ago" />
        </div>
      </section>
    </div>
  );
}

interface ActivityItemProps {
  icon: string;
  text: string;
  time: string;
}

function ActivityItem({ icon, text, time }: ActivityItemProps) {
  return (
    <div className="activity-item">
      <span className="activity-item__icon" aria-hidden="true">
        {icon}
      </span>
      <div>
        <p className="activity-item__text">{text}</p>
        <p className="activity-item__time">{time}</p>
      </div>
    </div>
  );
}
