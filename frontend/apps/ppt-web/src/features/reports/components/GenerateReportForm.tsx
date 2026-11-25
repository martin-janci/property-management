/**
 * GenerateReportForm - on-demand report generation (gap audit 3.1).
 *
 * Generates one of the built-in reports (faults, voting, occupancy,
 * consumption) for a date range in CSV, Excel or PDF via
 * POST /api/v1/reports/export. The parent owns the request (and the
 * download); this component only collects and validates the inputs.
 */

import { type FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BUILTIN_REPORTS, type BuiltinReport } from '../builtinReports';

export type GenerateReportFormat = 'pdf' | 'excel' | 'csv';

export interface GenerateReportValues {
  reportType: BuiltinReport['type'];
  format: GenerateReportFormat;
  fromDate: string;
  toDate: string;
}

interface GenerateReportFormProps {
  isGenerating?: boolean;
  onGenerate: (values: GenerateReportValues) => Promise<void> | void;
}

const FORMATS: GenerateReportFormat[] = ['pdf', 'excel', 'csv'];

function isoDate(date: Date): string {
  return date.toISOString().slice(0, 10);
}

function defaultRange(): { fromDate: string; toDate: string } {
  const to = new Date();
  const from = new Date(to);
  from.setDate(from.getDate() - 30);
  return { fromDate: isoDate(from), toDate: isoDate(to) };
}

export function GenerateReportForm({ isGenerating, onGenerate }: GenerateReportFormProps) {
  const { t } = useTranslation();
  const [values, setValues] = useState<GenerateReportValues>(() => ({
    reportType: 'faults',
    format: 'pdf',
    ...defaultRange(),
  }));
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (!values.fromDate || !values.toDate || values.fromDate > values.toDate) {
      setError(t('reports.page.generate.invalidRange'));
      return;
    }
    setError(null);
    await onGenerate(values);
  };

  return (
    <form onSubmit={handleSubmit} className="bg-white rounded-lg shadow p-6 space-y-6">
      <div>
        <h2 className="text-lg font-medium text-gray-900">{t('reports.page.generate.title')}</h2>
        <p className="text-sm text-gray-500 mt-1">{t('reports.page.generate.description')}</p>
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <label className="block">
          <span className="block text-sm font-medium text-gray-700">
            {t('reports.page.generate.report')}
          </span>
          <select
            value={values.reportType}
            onChange={(e) =>
              setValues((v) => ({ ...v, reportType: e.target.value as BuiltinReport['type'] }))
            }
            className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
          >
            {BUILTIN_REPORTS.map((report) => (
              <option key={report.type} value={report.type}>
                {t(report.labelKey, report.name)}
              </option>
            ))}
          </select>
        </label>

        <fieldset>
          <legend className="block text-sm font-medium text-gray-700">
            {t('reports.page.generate.format')}
          </legend>
          <div className="mt-2 flex gap-4">
            {FORMATS.map((format) => (
              <label key={format} className="inline-flex items-center gap-2 text-sm">
                <input
                  type="radio"
                  name="report-format"
                  value={format}
                  checked={values.format === format}
                  onChange={() => setValues((v) => ({ ...v, format }))}
                />
                {t(`reports.schedule.form.formats.${format}`)}
              </label>
            ))}
          </div>
        </fieldset>

        <label className="block">
          <span className="block text-sm font-medium text-gray-700">
            {t('reports.page.generate.from')}
          </span>
          <input
            type="date"
            value={values.fromDate}
            max={values.toDate || undefined}
            onChange={(e) => setValues((v) => ({ ...v, fromDate: e.target.value }))}
            className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
          />
        </label>

        <label className="block">
          <span className="block text-sm font-medium text-gray-700">
            {t('reports.page.generate.to')}
          </span>
          <input
            type="date"
            value={values.toDate}
            min={values.fromDate || undefined}
            onChange={(e) => setValues((v) => ({ ...v, toDate: e.target.value }))}
            className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
          />
        </label>
      </div>

      {error && (
        <p role="alert" className="text-sm text-red-600">
          {error}
        </p>
      )}

      <div className="flex justify-end">
        <button
          type="submit"
          disabled={isGenerating}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {isGenerating ? t('reports.page.generate.generating') : t('reports.page.generate.submit')}
        </button>
      </div>
    </form>
  );
}
