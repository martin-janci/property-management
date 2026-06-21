/**
 * Threshold rule form (Epic 14 — FR73).
 *
 * One presentational form drives both the add (`mode="create"`) and edit
 * (`mode="edit"`) flows for a sensor's alert thresholds. The page in
 * {@link IotThresholdConfigPage} keeps the open/close state and maps the
 * emitted {@link ThresholdFormValues} onto the `@ppt/api-client`
 * create/update threshold request shapes.
 *
 * In edit mode the `metric` is rendered read-only because the api-server
 * `UpdateSensorThreshold` shape does not accept it.
 */
import type { JSX } from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface ThresholdFormValues {
  metric: string;
  comparison: string;
  warningValue: string;
  warningHigh: string;
  criticalValue: string;
  criticalHigh: string;
  cooldownMinutes: string;
  enabled: boolean;
}

export interface ThresholdFormProps {
  mode: 'create' | 'edit';
  initialValues?: Partial<ThresholdFormValues>;
  isSaving: boolean;
  onSubmit: (values: ThresholdFormValues) => void;
  onCancel: () => void;
}

const EMPTY_VALUES: ThresholdFormValues = {
  metric: '',
  comparison: 'gt',
  warningValue: '',
  warningHigh: '',
  criticalValue: '',
  criticalHigh: '',
  cooldownMinutes: '',
  enabled: true,
};

/** Common metrics — offered as suggestions, free text still allowed. */
const METRIC_OPTIONS = ['temperature', 'humidity', 'co2', 'pressure', 'voltage', 'value'];

/** Comparison operators understood by the alerting engine. */
const COMPARISON_OPTIONS = ['gt', 'lt', 'range'];

const fieldClass =
  'mt-1 block w-full rounded-lg border border-gray-300 px-3 py-2 text-sm shadow-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 disabled:bg-gray-100 disabled:text-gray-500';
const labelClass = 'block text-sm font-medium text-gray-700';

export function ThresholdForm({
  mode,
  initialValues,
  isSaving,
  onSubmit,
  onCancel,
}: ThresholdFormProps): JSX.Element {
  const { t } = useTranslation();
  const [values, setValues] = useState<ThresholdFormValues>({
    ...EMPTY_VALUES,
    ...initialValues,
  });

  const isEdit = mode === 'edit';

  const setField = <K extends keyof ThresholdFormValues>(key: K, value: ThresholdFormValues[K]) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  };

  // At least one warning/critical bound must be set for a meaningful rule.
  const hasBound =
    values.warningValue.trim() !== '' ||
    values.warningHigh.trim() !== '' ||
    values.criticalValue.trim() !== '' ||
    values.criticalHigh.trim() !== '';
  const canSubmit = values.comparison.trim().length > 0 && hasBound;

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    if (!canSubmit || isSaving) {
      return;
    }
    onSubmit(values);
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="space-y-5 rounded-lg border border-gray-200 bg-white p-5 shadow-sm"
    >
      <h3 className="text-base font-semibold text-gray-900">
        {isEdit
          ? t('iot.editThresholdTitle', { defaultValue: 'Edit threshold' })
          : t('iot.addThresholdTitle', { defaultValue: 'Add threshold' })}
      </h3>

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div>
          <label htmlFor="threshold-metric" className={labelClass}>
            {t('iot.thresholdMetric', { defaultValue: 'Metric' })}
          </label>
          <input
            id="threshold-metric"
            type="text"
            list="threshold-metric-options"
            className={fieldClass}
            value={values.metric}
            disabled={isEdit}
            placeholder={t('iot.thresholdMetricPlaceholder', {
              defaultValue: 'e.g. temperature (blank = default)',
            })}
            onChange={(e) => setField('metric', e.target.value)}
          />
          <datalist id="threshold-metric-options">
            {METRIC_OPTIONS.map((opt) => (
              <option key={opt} value={opt} />
            ))}
          </datalist>
        </div>

        <div>
          <label htmlFor="threshold-comparison" className={labelClass}>
            {t('iot.thresholdComparison', { defaultValue: 'Comparison' })}
            <span className="text-red-500"> *</span>
          </label>
          <select
            id="threshold-comparison"
            className={fieldClass}
            value={values.comparison}
            onChange={(e) => setField('comparison', e.target.value)}
            required
          >
            {COMPARISON_OPTIONS.map((opt) => (
              <option key={opt} value={opt}>
                {t(`iot.comparison_${opt}`, { defaultValue: opt })}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div>
          <label htmlFor="threshold-warning-low" className={labelClass}>
            {t('iot.thresholdWarningLow', { defaultValue: 'Warning (low)' })}
          </label>
          <input
            id="threshold-warning-low"
            type="number"
            step="any"
            className={fieldClass}
            value={values.warningValue}
            onChange={(e) => setField('warningValue', e.target.value)}
          />
        </div>
        <div>
          <label htmlFor="threshold-warning-high" className={labelClass}>
            {t('iot.thresholdWarningHigh', { defaultValue: 'Warning (high)' })}
          </label>
          <input
            id="threshold-warning-high"
            type="number"
            step="any"
            className={fieldClass}
            value={values.warningHigh}
            onChange={(e) => setField('warningHigh', e.target.value)}
          />
        </div>
        <div>
          <label htmlFor="threshold-critical-low" className={labelClass}>
            {t('iot.thresholdCriticalLow', { defaultValue: 'Critical (low)' })}
          </label>
          <input
            id="threshold-critical-low"
            type="number"
            step="any"
            className={fieldClass}
            value={values.criticalValue}
            onChange={(e) => setField('criticalValue', e.target.value)}
          />
        </div>
        <div>
          <label htmlFor="threshold-critical-high" className={labelClass}>
            {t('iot.thresholdCriticalHigh', { defaultValue: 'Critical (high)' })}
          </label>
          <input
            id="threshold-critical-high"
            type="number"
            step="any"
            className={fieldClass}
            value={values.criticalHigh}
            onChange={(e) => setField('criticalHigh', e.target.value)}
          />
        </div>
      </div>

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div>
          <label htmlFor="threshold-cooldown" className={labelClass}>
            {t('iot.thresholdCooldown', { defaultValue: 'Alert cooldown (minutes)' })}
          </label>
          <input
            id="threshold-cooldown"
            type="number"
            min="0"
            className={fieldClass}
            value={values.cooldownMinutes}
            onChange={(e) => setField('cooldownMinutes', e.target.value)}
          />
        </div>
        <div className="flex items-end">
          <label className="inline-flex items-center gap-2 text-sm font-medium text-gray-700">
            <input
              type="checkbox"
              className="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
              checked={values.enabled}
              onChange={(e) => setField('enabled', e.target.checked)}
            />
            {t('iot.thresholdEnabled', { defaultValue: 'Enabled' })}
          </label>
        </div>
      </div>

      {!hasBound && (
        <p className="text-xs text-gray-400">
          {t('iot.thresholdNeedsBound', {
            defaultValue: 'Set at least one warning or critical value.',
          })}
        </p>
      )}

      <div className="flex items-center justify-end gap-3 border-t border-gray-100 pt-4">
        <button
          type="button"
          onClick={onCancel}
          className="btn-outline-token rounded-lg px-4 py-2 text-sm font-medium"
        >
          {t('common.cancel', { defaultValue: 'Cancel' })}
        </button>
        <button
          type="submit"
          disabled={!canSubmit || isSaving}
          className="btn-primary-token rounded-lg px-4 py-2 text-sm font-medium disabled:opacity-50"
        >
          {isSaving
            ? t('common.saving', { defaultValue: 'Saving…' })
            : isEdit
              ? t('common.saveChanges', { defaultValue: 'Save changes' })
              : t('iot.addThreshold', { defaultValue: 'Add threshold' })}
        </button>
      </div>
    </form>
  );
}
