/**
 * Per-sensor threshold configuration page (Epic 14 — FR73).
 *
 * Lists a sensor's configured alert thresholds and lets the operator add,
 * edit, delete and toggle them, plus apply a reusable template scoped to the
 * sensor's type. Presentational: the route wrapper in
 * `routes/groups/iot.tsx` wires the `@ppt/api-client` threshold hooks and
 * passes the data + mutation callbacks in.
 */
import type { Sensor, SensorThreshold, ThresholdTemplate } from '@ppt/api-client';
import type { JSX } from 'react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ThresholdForm, type ThresholdFormValues } from '../components/ThresholdForm';
import { ThresholdList } from '../components/ThresholdList';

export interface IotThresholdConfigPageProps {
  sensor?: Sensor | null;
  sensorLoading?: boolean;
  thresholds: SensorThreshold[];
  thresholdsLoading: boolean;
  templates: ThresholdTemplate[];
  templatesLoading: boolean;
  isSaving: boolean;
  togglingId: string | null;
  deletingId: string | null;
  applyingTemplate: boolean;
  /** Resolves on success so the page can close the form; rejects on error. */
  onCreate: (values: ThresholdFormValues) => Promise<void>;
  onUpdate: (id: string, values: ThresholdFormValues) => Promise<void>;
  onDelete: (threshold: SensorThreshold) => void;
  onToggleEnabled: (threshold: SensorThreshold) => void;
  onApplyTemplate: (templateId: string) => void;
  onBack: () => void;
}

/** Map a loaded threshold onto the editable form value shape. */
function thresholdToFormValues(th: SensorThreshold): Partial<ThresholdFormValues> {
  const numOrEmpty = (v: number | null) => (v == null ? '' : String(v));
  return {
    metric: th.metric,
    comparison: th.comparison,
    warningValue: numOrEmpty(th.warning_value),
    warningHigh: numOrEmpty(th.warning_high),
    criticalValue: numOrEmpty(th.critical_value),
    criticalHigh: numOrEmpty(th.critical_high),
    cooldownMinutes: numOrEmpty(th.alert_cooldown_minutes),
    enabled: th.enabled,
  };
}

type FormState = { mode: 'create' } | { mode: 'edit'; threshold: SensorThreshold } | null;

export function IotThresholdConfigPage({
  sensor,
  sensorLoading = false,
  thresholds,
  thresholdsLoading,
  templates,
  templatesLoading,
  isSaving,
  togglingId,
  deletingId,
  applyingTemplate,
  onCreate,
  onUpdate,
  onDelete,
  onToggleEnabled,
  onApplyTemplate,
  onBack,
}: IotThresholdConfigPageProps): JSX.Element {
  const { t } = useTranslation();
  const [formState, setFormState] = useState<FormState>(null);
  const [selectedTemplateId, setSelectedTemplateId] = useState('');

  // Only templates matching the sensor's type are applicable.
  const applicableTemplates = useMemo(() => {
    if (!sensor) {
      return templates;
    }
    return templates.filter((tpl) => tpl.sensor_type === sensor.sensor_type);
  }, [templates, sensor]);

  const handleSubmit = async (values: ThresholdFormValues) => {
    try {
      if (formState?.mode === 'edit') {
        await onUpdate(formState.threshold.id, values);
      } else {
        await onCreate(values);
      }
      setFormState(null);
    } catch {
      // Error is surfaced via toast by the route wrapper; keep the form open.
    }
  };

  const handleApplyTemplate = () => {
    if (!selectedTemplateId) {
      return;
    }
    onApplyTemplate(selectedTemplateId);
    setSelectedTemplateId('');
  };

  return (
    <div className="mx-auto max-w-4xl px-4 py-6">
      <header className="mb-6">
        <button
          type="button"
          onClick={onBack}
          className="mb-2 text-sm font-medium text-blue-600 hover:underline"
        >
          ← {t('iot.backToSensors', { defaultValue: 'Back to sensors' })}
        </button>
        <h1 className="text-2xl font-semibold text-gray-900">
          {t('iot.thresholdConfigTitle', { defaultValue: 'Threshold configuration' })}
        </h1>
        <p className="mt-1 text-sm text-gray-500">
          {sensorLoading
            ? t('common.loading', { defaultValue: 'Loading…' })
            : sensor
              ? t('iot.thresholdConfigSubtitle', {
                  defaultValue: 'Alert rules for {{name}} ({{type}}).',
                  name: sensor.name,
                  type: sensor.sensor_type,
                })
              : t('iot.thresholdConfigSubtitleGeneric', {
                  defaultValue: 'Configure alert rules for this sensor.',
                })}
        </p>
      </header>

      {/* Apply template */}
      <section className="mb-6 rounded-lg border border-gray-200 bg-white p-5 shadow-sm">
        <h2 className="text-base font-semibold text-gray-900">
          {t('iot.applyTemplateTitle', { defaultValue: 'Apply template' })}
        </h2>
        <p className="mt-1 text-sm text-gray-500">
          {t('iot.applyTemplateHint', {
            defaultValue: 'Create a threshold from a predefined template for this sensor type.',
          })}
        </p>
        <div className="mt-3 flex flex-wrap items-center gap-3">
          <select
            aria-label={t('iot.selectTemplate', { defaultValue: 'Select a template' })}
            className="block w-full max-w-md rounded-lg border border-gray-300 px-3 py-2 text-sm shadow-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 disabled:bg-gray-100"
            value={selectedTemplateId}
            disabled={templatesLoading || applyingTemplate}
            onChange={(e) => setSelectedTemplateId(e.target.value)}
          >
            <option value="">
              {templatesLoading
                ? t('common.loading', { defaultValue: 'Loading…' })
                : applicableTemplates.length === 0
                  ? t('iot.noTemplates', { defaultValue: 'No templates for this sensor type' })
                  : t('iot.selectTemplate', { defaultValue: 'Select a template' })}
            </option>
            {applicableTemplates.map((tpl) => (
              <option key={tpl.id} value={tpl.id}>
                {tpl.name}
                {tpl.is_default
                  ? ` (${t('iot.templateDefault', { defaultValue: 'default' })})`
                  : ''}
              </option>
            ))}
          </select>
          <button
            type="button"
            onClick={handleApplyTemplate}
            disabled={!selectedTemplateId || applyingTemplate}
            className="btn-primary-token rounded-lg px-4 py-2 text-sm font-medium disabled:opacity-50"
          >
            {applyingTemplate
              ? t('iot.applying', { defaultValue: 'Applying…' })
              : t('iot.applyTemplate', { defaultValue: 'Apply' })}
          </button>
        </div>
      </section>

      {/* Threshold list + add/edit */}
      <section>
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-base font-semibold text-gray-900">
            {t('iot.thresholdsTitle', { defaultValue: 'Thresholds' })}
          </h2>
          {formState === null && (
            <button
              type="button"
              onClick={() => setFormState({ mode: 'create' })}
              className="btn-primary-token rounded-lg px-4 py-2 text-sm font-medium"
            >
              {t('iot.addThreshold', { defaultValue: 'Add threshold' })}
            </button>
          )}
        </div>

        {formState !== null && (
          <div className="mb-6">
            <ThresholdForm
              mode={formState.mode}
              initialValues={
                formState.mode === 'edit' ? thresholdToFormValues(formState.threshold) : undefined
              }
              isSaving={isSaving}
              onSubmit={handleSubmit}
              onCancel={() => setFormState(null)}
            />
          </div>
        )}

        <ThresholdList
          thresholds={thresholds}
          isLoading={thresholdsLoading}
          togglingId={togglingId}
          deletingId={deletingId}
          onEdit={(th) => setFormState({ mode: 'edit', threshold: th })}
          onDelete={onDelete}
          onToggleEnabled={onToggleEnabled}
        />
      </section>
    </div>
  );
}
