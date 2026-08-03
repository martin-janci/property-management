/**
 * EditScheduleModal - Modal for editing report schedules.
 *
 * Story 81.1 - Report Schedule Editing
 *
 * Three primary controls (gap-81-1 API contract):
 *   1. Cron picker  — builds / validates a 5-field UNIX cron expression
 *   2. Recipient list — managed by RecipientManager
 *   3. Enabled toggle — maps to the `enabled` flag on the backend
 */

import type { CronScheduleUpdateRequest, ReportSchedule } from '@ppt/api-client';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { CronPicker, isValidCron } from './CronPicker';
import { RecipientManager } from './RecipientManager';

interface EditScheduleModalProps {
  schedule: ReportSchedule;
  isOpen: boolean;
  isSubmitting?: boolean;
  onClose: () => void;
  /**
   * Called with the cron-based update payload.
   * The modal emits `CronScheduleUpdateRequest` to match the gap-81-1 endpoint.
   */
  onSave: (id: string, data: CronScheduleUpdateRequest) => Promise<void>;
}

interface FormErrors {
  cronExpression?: string;
  recipients?: string;
}

/**
 * Derive an initial cron expression from a schedule.
 *
 * Round-trip contract (gap-81-1 / issue #616): the backend now persists the
 * edited expression to a dedicated `cron_expression` column and returns it on
 * read, so when that field is present and valid we surface it verbatim — a
 * custom every-15-minutes-on-weekday-business-hours expression survives an
 * edit round-trip instead of being flattened back into the overloaded
 * `time`/`frequency` fields.
 *
 * Back-compat fallback: legacy rows predating the column have no
 * `cron_expression`. For those, the `time` field stores "HH:mm"; we convert
 * that + the frequency/day fields into a 5-field cron expression so the picker
 * can still display a sensible starting value.
 */
export function scheduleToInitialCron(schedule: ReportSchedule): string {
  // Prefer the dedicated cron column when the backend returned a usable one.
  const stored = schedule.cron_expression?.trim();
  if (stored && isValidCron(stored)) {
    return stored;
  }

  const [hh, mm] = (schedule.time || '08:00').split(':');
  const h = String(Number(hh) || 0);
  const m = String(Number(mm) || 0);

  switch (schedule.frequency) {
    case 'daily':
      return `${m} ${h} * * *`;
    case 'weekly': {
      const dow = schedule.day_of_week ?? 1;
      return `${m} ${h} * * ${dow}`;
    }
    case 'monthly':
    case 'quarterly': {
      const dom = schedule.day_of_month ?? 1;
      return `${m} ${h} ${dom} * *`;
    }
    case 'yearly':
      return `${m} ${h} 1 1 *`;
    default:
      return `0 ${h} * * *`;
  }
}

export function EditScheduleModal({
  schedule,
  isOpen,
  isSubmitting,
  onClose,
  onSave,
}: EditScheduleModalProps) {
  const { t } = useTranslation();
  const [cronExpression, setCronExpression] = useState(() => scheduleToInitialCron(schedule));
  const [recipients, setRecipients] = useState<string[]>(schedule.recipients);
  const [enabled, setEnabled] = useState(schedule.is_active);
  const [errors, setErrors] = useState<FormErrors>({});

  // Reset form when the schedule prop changes (e.g. user opens a different row)
  useEffect(() => {
    setCronExpression(scheduleToInitialCron(schedule));
    setRecipients(schedule.recipients);
    setEnabled(schedule.is_active);
    setErrors({});
  }, [schedule]);

  const validate = useCallback((): boolean => {
    const newErrors: FormErrors = {};

    // A cron expression must be present AND valid. The picker only calls
    // onChange with valid expressions in simple mode, but in custom mode
    // the user may type an invalid string; cronExpression retains the
    // previously-valid value while the input shows the invalid text.
    // isValidCron guards against that stale-value submission.
    if (!cronExpression.trim() || !isValidCron(cronExpression)) {
      newErrors.cronExpression = 'A valid cron expression is required';
    }

    if (recipients.length === 0) {
      newErrors.recipients = 'At least one recipient is required';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  }, [cronExpression, recipients]);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      if (!validate()) return;

      const payload: CronScheduleUpdateRequest = {
        cron_expression: cronExpression.trim(),
        recipients,
        enabled,
      };

      await onSave(schedule.id, payload);
    },
    [validate, cronExpression, recipients, enabled, onSave, schedule.id]
  );

  const handleRecipientsChange = useCallback((newRecipients: string[]) => {
    setRecipients(newRecipients);
    setErrors((prev) => ({ ...prev, recipients: undefined }));
  }, []);

  const handleCronChange = useCallback((cron: string) => {
    setCronExpression(cron);
    setErrors((prev) => ({ ...prev, cronExpression: undefined }));
  }, []);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      {/* Backdrop */}
      <div className="fixed inset-0 bg-black bg-opacity-50" onClick={onClose} aria-hidden="true" />

      {/* Modal panel */}
      <div className="flex min-h-full items-center justify-center p-4">
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="edit-schedule-title"
          className="relative bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-y-auto"
        >
          {/* ── Header ── */}
          <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200">
            <div>
              <h2 id="edit-schedule-title" className="text-lg font-semibold text-gray-900">
                Edit Schedule
              </h2>
              <p className="mt-0.5 text-sm text-gray-500 truncate max-w-xs">{schedule.name}</p>
            </div>
            <button
              type="button"
              onClick={onClose}
              className="text-gray-400 hover:text-gray-500"
              aria-label={t('aria.close')}
            >
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>

          {/* ── Form body ── */}
          <form onSubmit={handleSubmit} className="px-6 py-5 space-y-6">
            {/* ── 1. Enabled toggle ── */}
            <div className="flex items-center justify-between p-4 bg-gray-50 rounded-lg border border-gray-200">
              <div>
                <p className="text-sm font-medium text-gray-900">Schedule enabled</p>
                <p className="text-xs text-gray-500 mt-0.5">
                  {enabled
                    ? 'Reports will be delivered on the configured schedule.'
                    : 'Schedule is paused — no reports will be sent.'}
                </p>
              </div>
              {/* Toggle switch */}
              <button
                type="button"
                role="switch"
                aria-checked={enabled}
                onClick={() => setEnabled((v) => !v)}
                disabled={isSubmitting}
                className={`relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full
                  border-2 border-transparent transition-colors duration-200 ease-in-out
                  focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2
                  disabled:opacity-50 disabled:cursor-not-allowed
                  ${enabled ? 'bg-blue-600' : 'bg-gray-200'}`}
              >
                <span className="sr-only">{enabled ? 'Disable schedule' : 'Enable schedule'}</span>
                <span
                  className={`pointer-events-none inline-block h-5 w-5 transform rounded-full
                    bg-white shadow ring-0 transition duration-200 ease-in-out
                    ${enabled ? 'translate-x-5' : 'translate-x-0'}`}
                />
              </button>
            </div>

            {/* ── 2. Cron picker ── */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Schedule (cron)
                <span className="ml-1 text-red-500" aria-hidden="true">
                  *
                </span>
              </label>
              <CronPicker
                value={cronExpression}
                onChange={handleCronChange}
                error={errors.cronExpression}
                disabled={isSubmitting}
              />
            </div>

            {/* ── 3. Recipients ── */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Recipients
                <span className="ml-1 text-red-500" aria-hidden="true">
                  *
                </span>
              </label>
              <RecipientManager
                recipients={recipients}
                onChange={handleRecipientsChange}
                error={errors.recipients}
              />
            </div>
          </form>

          {/* ── Footer ── */}
          <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-gray-200 bg-gray-50">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300
                rounded-md hover:bg-gray-50 disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="submit"
              onClick={handleSubmit}
              disabled={isSubmitting}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md
                hover:bg-blue-700 disabled:opacity-50"
            >
              {isSubmitting ? 'Saving…' : 'Save changes'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
