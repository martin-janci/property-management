/**
 * Threshold configuration page for a single sensor (Epic 14 — FR73).
 *
 * Lists existing threshold rules and provides an inline form to add new ones
 * or edit/delete existing ones. Presentational — route wrapper in
 * `routes/groups/iot.tsx` wires hooks and passes data + handlers in.
 */
import type { Sensor, SensorThreshold } from '@ppt/api-client';
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
  alertCooldownMinutes: string;
}

export interface SensorThresholdPageProps {
  sensor: Sensor | null;
  thresholds: SensorThreshold[];
  isLoading: boolean;
  isSaving: boolean;
  deletingThresholdId: string | null;
  onBack: () => void;
  onCreate: (values: ThresholdFormValues) => void;
  onUpdate: (id: string, values: Partial<ThresholdFormValues> & { enabled?: boolean }) => void;
  onDelete: (threshold: SensorThreshold) => void;
}

const COMPARISON_OPTIONS = ['>', '>=', '<', '<=', '==', '!='];

const EMPTY_FORM: ThresholdFormValues = {
  metric: '',
  comparison: '>',
  warningValue: '',
  warningHigh: '',
  criticalValue: '',
  criticalHigh: '',
  alertCooldownMinutes: '',
};

function formatNum(v: number | null): string {
  return v == null ? '—' : String(v);
}

export function SensorThresholdPage({
  sensor,
  thresholds,
  isLoading,
  isSaving,
  deletingThresholdId,
  onBack,
  onCreate,
  onUpdate,
  onDelete,
}: SensorThresholdPageProps): JSX.Element {
  const { t } = useTranslation();
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState<ThresholdFormValues>(EMPTY_FORM);
  const [editingId, setEditingId] = useState<string | null>(null);

  function handleField(key: keyof ThresholdFormValues, value: string) {
    setForm((prev) => ({ ...prev, [key]: value }));
  }

  function openCreate() {
    setEditingId(null);
    setForm(EMPTY_FORM);
    setShowForm(true);
  }

  function openEdit(t: SensorThreshold) {
    setEditingId(t.id);
    setForm({
      metric: t.metric,
      comparison: t.comparison,
      warningValue: t.warning_value != null ? String(t.warning_value) : '',
      warningHigh: t.warning_high != null ? String(t.warning_high) : '',
      criticalValue: t.critical_value != null ? String(t.critical_value) : '',
      criticalHigh: t.critical_high != null ? String(t.critical_high) : '',
      alertCooldownMinutes:
        t.alert_cooldown_minutes != null ? String(t.alert_cooldown_minutes) : '',
    });
    setShowForm(true);
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (editingId) {
      onUpdate(editingId, form);
    } else {
      onCreate(form);
    }
    setShowForm(false);
    setForm(EMPTY_FORM);
    setEditingId(null);
  }

  function handleCancel() {
    setShowForm(false);
    setForm(EMPTY_FORM);
    setEditingId(null);
  }

  const canSubmit = form.comparison.trim().length > 0;

  return (
    <div className="mx-auto max-w-4xl px-4 py-6">
      <header className="mb-6 flex flex-wrap items-start justify-between gap-3">
        <div>
          <button
            type="button"
            onClick={onBack}
            className="mb-2 text-sm text-blue-600 hover:underline"
          >
            ← {t('common.back', { defaultValue: 'Back' })}
          </button>
          <h1 className="text-2xl font-semibold text-gray-900">
            {t('iot.thresholdsTitle', { defaultValue: 'Thresholds' })}
            {sensor ? ` — ${sensor.name}` : ''}
          </h1>
          <p className="mt-1 text-sm text-gray-500">
            {t('iot.thresholdsSubtitle', {
              defaultValue: 'Configure alert rules for this sensor.',
            })}
          </p>
        </div>
        {!showForm && (
          <button
            type="button"
            onClick={openCreate}
            className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700"
          >
            {t('iot.addThreshold', { defaultValue: 'Add threshold' })}
          </button>
        )}
      </header>

      {showForm && (
        <form
          onSubmit={handleSubmit}
          className="mb-6 rounded-lg border border-gray-200 bg-gray-50 p-4"
        >
          <h2 className="mb-4 text-base font-semibold text-gray-800">
            {editingId
              ? t('iot.editThreshold', { defaultValue: 'Edit threshold' })
              : t('iot.newThreshold', { defaultValue: 'New threshold' })}
          </h2>
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
            <div>
              <label className="mb-1 block text-sm font-medium text-gray-700">
                {t('iot.metric', { defaultValue: 'Metric' })}
              </label>
              <input
                type="text"
                value={form.metric}
                onChange={(e) => handleField('metric', e.target.value)}
                placeholder={t('iot.metricPlaceholder', { defaultValue: 'e.g. temperature' })}
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-gray-700">
                {t('iot.comparison', { defaultValue: 'Comparison' })}
                <span className="text-red-500"> *</span>
              </label>
              <select
                value={form.comparison}
                onChange={(e) => handleField('comparison', e.target.value)}
                required
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                {COMPARISON_OPTIONS.map((op) => (
                  <option key={op} value={op}>
                    {op}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-gray-700">
                {t('iot.cooldown', { defaultValue: 'Cooldown (min)' })}
              </label>
              <input
                type="number"
                min="0"
                value={form.alertCooldownMinutes}
                onChange={(e) => handleField('alertCooldownMinutes', e.target.value)}
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-gray-700">
                {t('iot.warningValue', { defaultValue: 'Warning (low)' })}
              </label>
              <input
                type="number"
                step="any"
                value={form.warningValue}
                onChange={(e) => handleField('warningValue', e.target.value)}
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-gray-700">
                {t('iot.warningHigh', { defaultValue: 'Warning (high)' })}
              </label>
              <input
                type="number"
                step="any"
                value={form.warningHigh}
                onChange={(e) => handleField('warningHigh', e.target.value)}
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-gray-700">
                {t('iot.criticalValue', { defaultValue: 'Critical (low)' })}
              </label>
              <input
                type="number"
                step="any"
                value={form.criticalValue}
                onChange={(e) => handleField('criticalValue', e.target.value)}
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-gray-700">
                {t('iot.criticalHigh', { defaultValue: 'Critical (high)' })}
              </label>
              <input
                type="number"
                step="any"
                value={form.criticalHigh}
                onChange={(e) => handleField('criticalHigh', e.target.value)}
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>
          <div className="mt-4 flex items-center gap-3">
            <button
              type="submit"
              disabled={!canSubmit || isSaving}
              className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {isSaving
                ? t('common.saving', { defaultValue: 'Saving…' })
                : editingId
                  ? t('common.save', { defaultValue: 'Save' })
                  : t('iot.addThreshold', { defaultValue: 'Add threshold' })}
            </button>
            <button
              type="button"
              onClick={handleCancel}
              className="rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50"
            >
              {t('common.cancel', { defaultValue: 'Cancel' })}
            </button>
          </div>
        </form>
      )}

      {isLoading ? (
        <p className="py-8 text-center text-sm text-gray-400">
          {t('common.loading', { defaultValue: 'Loading…' })}
        </p>
      ) : thresholds.length === 0 ? (
        <div className="rounded-lg border border-dashed border-gray-300 py-12 text-center">
          <p className="text-sm text-gray-500">
            {t('iot.noThresholds', { defaultValue: 'No threshold rules yet.' })}
          </p>
          {!showForm && (
            <button
              type="button"
              onClick={openCreate}
              className="mt-3 text-sm font-medium text-blue-600 hover:underline"
            >
              {t('iot.addThreshold', { defaultValue: 'Add threshold' })}
            </button>
          )}
        </div>
      ) : (
        <div className="overflow-x-auto rounded-lg border border-gray-200">
          <table className="min-w-full divide-y divide-gray-200 text-sm">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-4 py-3 text-left font-medium text-gray-500">
                  {t('iot.metric', { defaultValue: 'Metric' })}
                </th>
                <th className="px-4 py-3 text-left font-medium text-gray-500">
                  {t('iot.comparison', { defaultValue: 'Comparison' })}
                </th>
                <th className="px-4 py-3 text-right font-medium text-gray-500">
                  {t('iot.warningValue', { defaultValue: 'Warn low' })}
                </th>
                <th className="px-4 py-3 text-right font-medium text-gray-500">
                  {t('iot.warningHigh', { defaultValue: 'Warn high' })}
                </th>
                <th className="px-4 py-3 text-right font-medium text-gray-500">
                  {t('iot.criticalValue', { defaultValue: 'Crit low' })}
                </th>
                <th className="px-4 py-3 text-right font-medium text-gray-500">
                  {t('iot.criticalHigh', { defaultValue: 'Crit high' })}
                </th>
                <th className="px-4 py-3 text-center font-medium text-gray-500">
                  {t('iot.enabled', { defaultValue: 'Enabled' })}
                </th>
                <th className="px-4 py-3 text-right font-medium text-gray-500">
                  {t('common.actions', { defaultValue: 'Actions' })}
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 bg-white">
              {thresholds.map((th) => (
                <tr key={th.id} className="hover:bg-gray-50">
                  <td className="px-4 py-3 font-mono text-gray-900">{th.metric || '—'}</td>
                  <td className="px-4 py-3 font-mono text-gray-700">{th.comparison}</td>
                  <td className="px-4 py-3 text-right text-gray-700">
                    {formatNum(th.warning_value)}
                  </td>
                  <td className="px-4 py-3 text-right text-gray-700">
                    {formatNum(th.warning_high)}
                  </td>
                  <td className="px-4 py-3 text-right text-gray-700">
                    {formatNum(th.critical_value)}
                  </td>
                  <td className="px-4 py-3 text-right text-gray-700">
                    {formatNum(th.critical_high)}
                  </td>
                  <td className="px-4 py-3 text-center">
                    <button
                      type="button"
                      onClick={() => onUpdate(th.id, { enabled: !th.enabled })}
                      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${
                        th.enabled
                          ? 'bg-green-100 text-green-700 hover:bg-green-200'
                          : 'bg-gray-100 text-gray-500 hover:bg-gray-200'
                      }`}
                    >
                      {th.enabled
                        ? t('common.on', { defaultValue: 'On' })
                        : t('common.off', { defaultValue: 'Off' })}
                    </button>
                  </td>
                  <td className="px-4 py-3 text-right">
                    <div className="flex justify-end gap-2">
                      <button
                        type="button"
                        onClick={() => openEdit(th)}
                        className="text-blue-600 hover:underline"
                      >
                        {t('common.edit', { defaultValue: 'Edit' })}
                      </button>
                      <button
                        type="button"
                        disabled={deletingThresholdId === th.id}
                        onClick={() => onDelete(th)}
                        className="text-red-600 hover:underline disabled:opacity-50"
                      >
                        {deletingThresholdId === th.id
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
      )}
    </div>
  );
}
