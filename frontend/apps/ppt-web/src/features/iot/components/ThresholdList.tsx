/**
 * Threshold rule list (Epic 14 — FR73).
 *
 * Presentational table of a sensor's configured thresholds with per-row
 * enable toggle, edit and delete actions. All data operations are delegated
 * to the parent via callbacks.
 */
import type { SensorThreshold } from '@ppt/api-client';
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';

export interface ThresholdListProps {
  thresholds: SensorThreshold[];
  isLoading?: boolean;
  togglingId?: string | null;
  deletingId?: string | null;
  onEdit: (threshold: SensorThreshold) => void;
  onDelete: (threshold: SensorThreshold) => void;
  onToggleEnabled: (threshold: SensorThreshold) => void;
}

/** Render a nullable numeric bound as text, falling back to an em-dash. */
function fmt(value: number | null): string {
  return value == null ? '—' : String(value);
}

export function ThresholdList({
  thresholds,
  isLoading = false,
  togglingId = null,
  deletingId = null,
  onEdit,
  onDelete,
  onToggleEnabled,
}: ThresholdListProps): JSX.Element {
  const { t } = useTranslation();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="h-6 w-6 animate-spin rounded-full border-b-2 border-blue-600" />
      </div>
    );
  }

  if (thresholds.length === 0) {
    return (
      <div className="rounded-lg border border-dashed border-gray-300 bg-gray-50 px-4 py-10 text-center text-sm text-gray-500">
        {t('iot.noThresholds', {
          defaultValue: 'No thresholds configured yet. Add one or apply a template.',
        })}
      </div>
    );
  }

  return (
    <div className="overflow-x-auto rounded-lg border border-gray-200 bg-white shadow-sm">
      <table className="min-w-full divide-y divide-gray-200 text-sm">
        <thead className="bg-gray-50">
          <tr className="text-left text-xs font-medium uppercase tracking-wide text-gray-500">
            <th className="px-4 py-3">{t('iot.thresholdMetric', { defaultValue: 'Metric' })}</th>
            <th className="px-4 py-3">
              {t('iot.thresholdComparison', { defaultValue: 'Comparison' })}
            </th>
            <th className="px-4 py-3">{t('iot.thresholdWarning', { defaultValue: 'Warning' })}</th>
            <th className="px-4 py-3">
              {t('iot.thresholdCritical', { defaultValue: 'Critical' })}
            </th>
            <th className="px-4 py-3">
              {t('iot.thresholdCooldownShort', { defaultValue: 'Cooldown' })}
            </th>
            <th className="px-4 py-3">{t('iot.thresholdEnabled', { defaultValue: 'Enabled' })}</th>
            <th className="px-4 py-3 text-right">
              {t('common.actions', { defaultValue: 'Actions' })}
            </th>
          </tr>
        </thead>
        <tbody className="divide-y divide-gray-100">
          {thresholds.map((th) => (
            <tr key={th.id} className="text-gray-700">
              <td className="px-4 py-3 font-medium text-gray-900">{th.metric || '—'}</td>
              <td className="px-4 py-3">
                {t(`iot.comparison_${th.comparison}`, { defaultValue: th.comparison })}
              </td>
              <td className="px-4 py-3">
                {fmt(th.warning_value)} / {fmt(th.warning_high)}
              </td>
              <td className="px-4 py-3">
                {fmt(th.critical_value)} / {fmt(th.critical_high)}
              </td>
              <td className="px-4 py-3">
                {th.alert_cooldown_minutes == null
                  ? '—'
                  : t('iot.minutesShort', {
                      defaultValue: '{{count}} min',
                      count: th.alert_cooldown_minutes,
                    })}
              </td>
              <td className="px-4 py-3">
                <button
                  type="button"
                  onClick={() => onToggleEnabled(th)}
                  disabled={togglingId === th.id}
                  className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium disabled:opacity-50 ${
                    th.enabled ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-600'
                  }`}
                >
                  {th.enabled
                    ? t('common.enabled', { defaultValue: 'Enabled' })
                    : t('common.disabled', { defaultValue: 'Disabled' })}
                </button>
              </td>
              <td className="px-4 py-3 text-right">
                <div className="flex justify-end gap-2">
                  <button
                    type="button"
                    onClick={() => onEdit(th)}
                    className="rounded-md px-2 py-1 text-xs font-medium text-blue-600 hover:bg-blue-50"
                  >
                    {t('common.edit', { defaultValue: 'Edit' })}
                  </button>
                  <button
                    type="button"
                    onClick={() => onDelete(th)}
                    disabled={deletingId === th.id}
                    className="rounded-md px-2 py-1 text-xs font-medium text-red-600 hover:bg-red-50 disabled:opacity-50"
                  >
                    {deletingId === th.id
                      ? t('common.deleting', { defaultValue: 'Deleting…' })
                      : t('common.delete', { defaultValue: 'Delete' })}
                  </button>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
