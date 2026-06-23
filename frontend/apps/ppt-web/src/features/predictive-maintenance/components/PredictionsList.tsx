import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import type { MaintenancePrediction } from '../types';

function RiskBadge({ score }: { score: number }): JSX.Element {
  const { t } = useTranslation();
  const pct = Math.round(score * 100);
  const cls =
    pct >= 75
      ? 'bg-red-100 text-red-700'
      : pct >= 50
        ? 'bg-orange-100 text-orange-700'
        : 'bg-yellow-100 text-yellow-700';
  return (
    <span className={`rounded-full px-2 py-0.5 text-xs font-semibold ${cls}`}>
      {t('predictive.riskBadge', { defaultValue: '{{pct}}% risk', pct })}
    </span>
  );
}

interface PredictionsListProps {
  predictions: MaintenancePrediction[];
  isLoading: boolean;
  pendingAcknowledgeId: string | null;
  onAcknowledge: (id: string) => void;
}

export function PredictionsList({
  predictions,
  isLoading,
  pendingAcknowledgeId,
  onAcknowledge,
}: PredictionsListProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <div className="rounded-lg border bg-white p-4 shadow-sm">
      <h2 className="mb-3 text-base font-semibold text-gray-900">
        {t('predictive.predictionsTitle', { defaultValue: 'Maintenance Predictions' })}
      </h2>

      {isLoading ? (
        <div className="space-y-2">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-14 animate-pulse rounded bg-gray-100" />
          ))}
        </div>
      ) : predictions.length === 0 ? (
        <p className="py-4 text-center text-sm text-gray-500">
          {t('predictive.noPredictions', { defaultValue: 'No high-risk predictions.' })}
        </p>
      ) : (
        <ul className="divide-y divide-gray-100">
          {predictions.map((p) => (
            <li key={p.id} className="flex items-start justify-between gap-4 py-3">
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <RiskBadge score={p.riskScore} />
                  {p.predictedFailureDate && (
                    <span className="text-xs text-gray-500">
                      {t('predictive.failureEta', {
                        defaultValue: 'Failure ~{{date}}',
                        date: new Date(p.predictedFailureDate).toLocaleDateString(),
                      })}
                    </span>
                  )}
                </div>
                <p className="mt-1 text-sm text-gray-700">{p.recommendation}</p>
                <p className="text-xs text-gray-400">
                  {t('predictive.confidenceMeta', {
                    defaultValue: 'Confidence {{pct}}% · {{date}}',
                    pct: Math.round(p.confidence * 100),
                    date: new Date(p.createdAt).toLocaleDateString(),
                  })}
                </p>
              </div>
              {!p.acknowledged ? (
                <button
                  type="button"
                  disabled={pendingAcknowledgeId === p.id}
                  onClick={() => onAcknowledge(p.id)}
                  className="shrink-0 rounded-md bg-indigo-600 px-3 py-1 text-xs font-medium text-white hover:bg-indigo-700 disabled:opacity-50"
                >
                  {pendingAcknowledgeId === p.id
                    ? t('common.saving', { defaultValue: 'Saving…' })
                    : t('predictive.acknowledge', { defaultValue: 'Acknowledge' })}
                </button>
              ) : (
                <span className="shrink-0 rounded-full bg-green-100 px-2 py-0.5 text-xs text-green-700">
                  {t('predictive.acknowledged', { defaultValue: 'Acknowledged' })}
                </span>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
