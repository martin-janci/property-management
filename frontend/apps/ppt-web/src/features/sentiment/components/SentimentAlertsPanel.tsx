import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import type { SentimentAlert } from '../types';

const ALERT_TYPE_LABELS: Record<string, string> = {
  spike_negative: 'Negative spike',
  sustained_decline: 'Sustained decline',
  anomaly: 'Anomaly',
};

interface SentimentAlertsPanelProps {
  alerts: SentimentAlert[];
  isLoading: boolean;
  pendingAcknowledgeId: string | null;
  onAcknowledge: (alertId: string) => void;
}

export function SentimentAlertsPanel({
  alerts,
  isLoading,
  pendingAcknowledgeId,
  onAcknowledge,
}: SentimentAlertsPanelProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <div className="rounded-lg border bg-white p-4 shadow-sm">
      <h2 className="mb-3 text-base font-semibold text-gray-900">
        {t('sentiment.alertsTitle', { defaultValue: 'Sentiment Alerts' })}
      </h2>

      {isLoading ? (
        <div className="space-y-2">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-12 animate-pulse rounded bg-gray-100" />
          ))}
        </div>
      ) : alerts.length === 0 ? (
        <p className="py-4 text-center text-sm text-gray-500">
          {t('sentiment.noAlerts', { defaultValue: 'No active alerts.' })}
        </p>
      ) : (
        <ul className="divide-y divide-gray-100">
          {alerts.map((alert) => (
            <li key={alert.id} className="flex items-center justify-between gap-4 py-3">
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium text-gray-900">
                  {ALERT_TYPE_LABELS[alert.alertType] ?? alert.alertType}
                </p>
                <p className="text-xs text-gray-500">
                  Score {alert.currentSentiment.toFixed(2)} · threshold{' '}
                  {alert.thresholdBreached.toFixed(2)} ·{' '}
                  {new Date(alert.createdAt).toLocaleDateString()}
                </p>
              </div>
              {!alert.acknowledged && (
                <button
                  type="button"
                  disabled={pendingAcknowledgeId === alert.id}
                  onClick={() => onAcknowledge(alert.id)}
                  className="shrink-0 rounded-md bg-indigo-600 px-3 py-1 text-xs font-medium text-white hover:bg-indigo-700 disabled:opacity-50"
                >
                  {pendingAcknowledgeId === alert.id
                    ? t('common.saving', { defaultValue: 'Saving…' })
                    : t('sentiment.acknowledge', { defaultValue: 'Acknowledge' })}
                </button>
              )}
              {alert.acknowledged && (
                <span className="shrink-0 rounded-full bg-green-100 px-2 py-0.5 text-xs text-green-700">
                  {t('sentiment.acknowledged', { defaultValue: 'Acknowledged' })}
                </span>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
