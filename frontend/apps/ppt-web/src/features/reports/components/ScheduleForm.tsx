/**
 * ScheduleForm component - Story 53.2
 *
 * Form for creating and editing report schedules.
 */

import type { CreateReportSchedule, ReportFormat, ScheduleFrequency } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BUILTIN_REPORTS } from '../builtinReports';

interface ScheduleFormProps {
  initialData?: Partial<CreateReportSchedule>;
  onSubmit: (data: CreateReportSchedule) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

// Only the cadences the backend `create_schedule` handler accepts. Quarterly and
// yearly used to be offered here but the backend rejects them with 400
// INVALID_FREQUENCY (issue #2324, finding 2), so they are removed to keep the
// form in sync with the contract.
const FREQUENCIES: { value: ScheduleFrequency; label: string }[] = [
  { value: 'daily', label: 'Daily' },
  { value: 'weekly', label: 'Weekly' },
  { value: 'monthly', label: 'Monthly' },
];

/**
 * Derive a 5-field UNIX cron expression from the legacy form fields (issue
 * #2324, finding 3) so a freshly-created schedule is cron-canonical from birth
 * and round-trips losslessly through the cron-first edit modal. Returns
 * `undefined` when the time can't be parsed (the backend then falls back to the
 * legacy frequency/time cadence).
 */
function deriveCron(
  frequency: ScheduleFrequency,
  time: string,
  dayOfWeek: number,
  dayOfMonth: number
): string | undefined {
  const match = /^(\d{1,2}):(\d{1,2})$/.exec(time);
  if (!match) return undefined;
  const hour = Number(match[1]);
  const minute = Number(match[2]);
  if (hour > 23 || minute > 59) return undefined;
  switch (frequency) {
    case 'daily':
      return `${minute} ${hour} * * *`;
    case 'weekly':
      return `${minute} ${hour} * * ${dayOfWeek}`;
    case 'monthly':
      return `${minute} ${hour} ${dayOfMonth} * *`;
    default:
      return undefined;
  }
}

const FORMATS: { value: ReportFormat; label: string }[] = [
  { value: 'pdf', label: 'PDF' },
  { value: 'excel', label: 'Excel' },
  { value: 'csv', label: 'CSV' },
];

const DAYS_OF_WEEK = [
  { value: 0, label: 'Sunday' },
  { value: 1, label: 'Monday' },
  { value: 2, label: 'Tuesday' },
  { value: 3, label: 'Wednesday' },
  { value: 4, label: 'Thursday' },
  { value: 5, label: 'Friday' },
  { value: 6, label: 'Saturday' },
];

interface FormErrors {
  report_id?: string;
  name?: string;
  recipients?: string;
  time?: string;
}

export function ScheduleForm({ initialData, onSubmit, onCancel, isSubmitting }: ScheduleFormProps) {
  const { t } = useTranslation();
  const [reportId, setReportId] = useState(initialData?.report_id || '');
  const [name, setName] = useState(initialData?.name || '');
  const [frequency, setFrequency] = useState<ScheduleFrequency>(initialData?.frequency || 'weekly');
  const [dayOfWeek, setDayOfWeek] = useState(initialData?.day_of_week ?? 1);
  const [dayOfMonth, setDayOfMonth] = useState(initialData?.day_of_month ?? 1);
  const [time, setTime] = useState(initialData?.time || '09:00');
  const [timezone, setTimezone] = useState(
    initialData?.timezone || Intl.DateTimeFormat().resolvedOptions().timeZone
  );
  const [format, setFormat] = useState<ReportFormat>(initialData?.format || 'pdf');
  const [recipients, setRecipients] = useState(initialData?.recipients?.join(', ') || '');
  const [errors, setErrors] = useState<FormErrors>({});

  const validate = (): boolean => {
    const newErrors: FormErrors = {};

    if (!reportId) {
      newErrors.report_id = 'Please select a report';
    }

    if (!name.trim()) {
      newErrors.name = 'Schedule name is required';
    }

    if (!recipients.trim()) {
      newErrors.recipients = 'At least one recipient is required';
    } else {
      const emails = recipients.split(',').map((e) => e.trim());
      const invalidEmails = emails.filter(
        (e) => !e.match(/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/)
      );
      if (invalidEmails.length > 0) {
        newErrors.recipients = `Invalid email(s): ${invalidEmails.join(', ')}`;
      }
    }

    if (!time) {
      newErrors.time = 'Time is required';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) return;

    const dow = frequency === 'weekly' ? dayOfWeek : undefined;
    const dom = frequency === 'monthly' ? dayOfMonth : undefined;

    // Await + swallow: the parent's onSubmit rejects on a backend error (so it
    // can keep this form open) and surfaces the reason via a toast. Firing it
    // fire-and-forget left that rejection unhandled on every failed create
    // (issue #2370); awaiting and catching here handles it at the form level.
    try {
      await onSubmit({
        report_id: reportId,
        name,
        frequency,
        day_of_week: dow,
        day_of_month: dom,
        time,
        timezone,
        format,
        recipients: recipients.split(',').map((e) => e.trim()),
        // Cron-canonical from birth (issue #2324, finding 3): the backend prefers
        // this over the legacy frequency/time fields when computing next_run_at.
        cron_expression: deriveCron(frequency, time, dayOfWeek, dayOfMonth),
      });
    } catch {
      // Intentionally swallowed — the parent already reported the failure.
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Report Selection */}
      <div>
        <label htmlFor="report" className="block text-sm font-medium text-gray-700">
          Report *
        </label>
        <select
          id="report"
          value={reportId}
          onChange={(e) => {
            setReportId(e.target.value);
            setErrors({ ...errors, report_id: undefined });
          }}
          className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 ${
            errors.report_id ? 'border-red-300' : 'border-gray-300'
          }`}
        >
          <option value="">Select a report</option>
          {BUILTIN_REPORTS.map((report) => (
            <option key={report.id} value={report.id}>
              {t(report.labelKey, report.name)}
            </option>
          ))}
        </select>
        {errors.report_id && <p className="mt-1 text-sm text-red-600">{errors.report_id}</p>}
      </div>

      {/* Schedule Name */}
      <div>
        <label htmlFor="schedule-name" className="block text-sm font-medium text-gray-700">
          Schedule Name *
        </label>
        <input
          type="text"
          id="schedule-name"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setErrors({ ...errors, name: undefined });
          }}
          className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 ${
            errors.name ? 'border-red-300' : 'border-gray-300'
          }`}
          placeholder="Weekly Revenue Summary"
        />
        {errors.name && <p className="mt-1 text-sm text-red-600">{errors.name}</p>}
      </div>

      {/* Frequency */}
      <div>
        <label htmlFor="frequency" className="block text-sm font-medium text-gray-700">
          Frequency
        </label>
        <select
          id="frequency"
          value={frequency}
          onChange={(e) => setFrequency(e.target.value as ScheduleFrequency)}
          className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
        >
          {FREQUENCIES.map((freq) => (
            <option key={freq.value} value={freq.value}>
              {freq.label}
            </option>
          ))}
        </select>
      </div>

      {/* Day Selection */}
      {frequency === 'weekly' && (
        <div>
          <label htmlFor="day-of-week" className="block text-sm font-medium text-gray-700">
            Day of Week
          </label>
          <select
            id="day-of-week"
            value={dayOfWeek}
            onChange={(e) => setDayOfWeek(Number(e.target.value))}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
          >
            {DAYS_OF_WEEK.map((day) => (
              <option key={day.value} value={day.value}>
                {day.label}
              </option>
            ))}
          </select>
        </div>
      )}

      {frequency === 'monthly' && (
        <div>
          <label htmlFor="day-of-month" className="block text-sm font-medium text-gray-700">
            Day of Month
          </label>
          <input
            type="number"
            id="day-of-month"
            min="1"
            max="31"
            value={dayOfMonth}
            onChange={(e) => {
              const value = Number(e.target.value);
              if (value >= 1 && value <= 31) {
                setDayOfMonth(value);
              }
            }}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
          />
          <p className="mt-1 text-xs text-gray-500">
            Enter a day between 1-31. For months with fewer days, the schedule will run on the last
            day of the month.
          </p>
        </div>
      )}

      {/* Time and Timezone */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label htmlFor="time" className="block text-sm font-medium text-gray-700">
            Time *
          </label>
          <input
            type="time"
            id="time"
            value={time}
            onChange={(e) => {
              setTime(e.target.value);
              setErrors({ ...errors, time: undefined });
            }}
            className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 ${
              errors.time ? 'border-red-300' : 'border-gray-300'
            }`}
          />
          {errors.time && <p className="mt-1 text-sm text-red-600">{errors.time}</p>}
        </div>
        <div>
          <label htmlFor="timezone" className="block text-sm font-medium text-gray-700">
            Timezone
          </label>
          <select
            id="timezone"
            value={timezone}
            onChange={(e) => setTimezone(e.target.value)}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
          >
            <option value="Europe/Bratislava">Europe/Bratislava</option>
            <option value="Europe/Prague">Europe/Prague</option>
            <option value="Europe/Berlin">Europe/Berlin</option>
            <option value="Europe/London">Europe/London</option>
            <option value="America/New_York">America/New_York</option>
            <option value="UTC">UTC</option>
          </select>
        </div>
      </div>

      {/* Format */}
      <div>
        <label htmlFor="format" className="block text-sm font-medium text-gray-700">
          Export Format
        </label>
        <div className="mt-2 flex gap-4">
          {FORMATS.map((fmt) => (
            <label key={fmt.value} className="flex items-center gap-2 cursor-pointer">
              <input
                type="radio"
                name="format"
                value={fmt.value}
                checked={format === fmt.value}
                onChange={() => setFormat(fmt.value)}
                className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300"
              />
              <span className="text-sm text-gray-700">{fmt.label}</span>
            </label>
          ))}
        </div>
      </div>

      {/* Recipients */}
      <div>
        <label htmlFor="recipients" className="block text-sm font-medium text-gray-700">
          Recipients *
        </label>
        <textarea
          id="recipients"
          value={recipients}
          onChange={(e) => {
            setRecipients(e.target.value);
            setErrors({ ...errors, recipients: undefined });
          }}
          rows={2}
          className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 ${
            errors.recipients ? 'border-red-300' : 'border-gray-300'
          }`}
          placeholder="email1@example.com, email2@example.com"
        />
        <p className="mt-1 text-xs text-gray-500">Separate multiple emails with commas</p>
        {errors.recipients && <p className="mt-1 text-sm text-red-600">{errors.recipients}</p>}
      </div>

      {/* Actions */}
      <div className="flex justify-end gap-3 pt-4 border-t">
        <button
          type="button"
          onClick={onCancel}
          disabled={isSubmitting}
          className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
        >
          Cancel
        </button>
        <button
          type="submit"
          disabled={isSubmitting}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {isSubmitting ? 'Saving...' : 'Save Schedule'}
        </button>
      </div>
    </form>
  );
}
