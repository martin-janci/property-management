/**
 * Sentiment Dashboard (Epic 13, Story 13.2)
 *
 * Presentational component — the route wrapper in routes/groups/core.tsx
 * wires API hooks and passes data + handlers in.
 */
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import { SentimentAlertsPanel, SentimentStatCard, SentimentTrendChart } from '../components';
import type { SentimentAlert, SentimentDashboard, SentimentTrend } from '../types';

export interface SentimentDashboardPageProps {
  dashboard: SentimentDashboard | undefined;
  dashboardLoading: boolean;
  trends: SentimentTrend[];
  trendsLoading: boolean;
  alerts: SentimentAlert[];
  alertsLoading: boolean;
  pendingAcknowledgeId: string | null;
  onAcknowledge: (alertId: string) => void;
}

export function SentimentDashboardPage({
  dashboard,
  dashboardLoading,
  trends,
  trendsLoading,
  alerts,
  alertsLoading,
  pendingAcknowledgeId,
  onAcknowledge,
}: SentimentDashboardPageProps): JSX.Element {
  const { t } = useTranslation();

  const avgScore = dashboard?.organization_avg ?? null;
  const unacknowledged = alerts.filter((a) => !a.acknowledged).length;

  const scoreTone =
    avgScore === null
      ? 'default'
      : avgScore >= 0.6
        ? 'success'
        : avgScore >= 0.4
          ? 'warning'
          : 'danger';

  return (
    <div className="mx-auto max-w-7xl px-4 py-6">
      <header className="mb-6">
        <h1 className="text-2xl font-semibold text-gray-900">
          {t('sentiment.pageTitle', { defaultValue: 'Tenant Sentiment' })}
        </h1>
        <p className="mt-1 text-sm text-gray-500">
          {t('sentiment.pageSubtitle', {
            defaultValue: 'AI-derived sentiment trends and alerts from tenant communications.',
          })}
        </p>
      </header>

      {/* KPI row */}
      <div className="mb-6 grid grid-cols-2 gap-4 md:grid-cols-4">
        <SentimentStatCard
          label={t('sentiment.orgAvg', { defaultValue: 'Org avg (30d)' })}
          value={dashboardLoading ? '—' : avgScore !== null ? avgScore.toFixed(2) : 'N/A'}
          tone={scoreTone}
        />
        <SentimentStatCard
          label={t('sentiment.totalTrends', { defaultValue: 'Data points' })}
          value={dashboardLoading ? '—' : (dashboard?.trends.length ?? 0)}
        />
        <SentimentStatCard
          label={t('sentiment.openAlerts', { defaultValue: 'Open alerts' })}
          value={alertsLoading ? '—' : unacknowledged}
          tone={unacknowledged > 0 ? 'danger' : 'default'}
        />
        <SentimentStatCard
          label={t('sentiment.totalAlerts', { defaultValue: 'Total alerts' })}
          value={alertsLoading ? '—' : alerts.length}
        />
      </div>

      <div className="space-y-6">
        <SentimentTrendChart trends={trends} isLoading={trendsLoading} />
        <SentimentAlertsPanel
          alerts={alerts}
          isLoading={alertsLoading}
          pendingAcknowledgeId={pendingAcknowledgeId}
          onAcknowledge={onAcknowledge}
        />
      </div>
    </div>
  );
}
