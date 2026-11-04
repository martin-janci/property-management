/**
 * TaxReportPage - Daňový výkaz (authority guest report) for short-term rentals.
 * Epic 18: Short-Term Rental Integration - UC-29 (Story 18.4)
 *
 * The manager picks a building, the authority and a period, previews the guest
 * counts by nationality (`POST /rentals/reports/preview`), stores the report
 * (`POST /rentals/reports`) and later marks it as submitted
 * (`POST /rentals/reports/{id}/submit`). Stored reports are listed from
 * `GET /rentals/reports`.
 *
 * The api-server does not compute rental income, expenses or income tax and
 * does not render report files yet, so the page offers no income figures and
 * no export; a download link appears only when a stored report has a file.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { formatReportPeriod, TaxReportPreview } from '../components/TaxReportPreview';
import { computeReportPeriod } from '../mappers';
import type {
  GuestReportAuthority,
  GuestReportParams,
  GuestReportPeriodType,
  GuestReportPreviewData,
  GuestReportSummaryItem,
  RentalBuildingOption,
} from '../types';

interface TaxReportPageProps {
  buildings: RentalBuildingOption[];
  isLoadingBuildings?: boolean;
  isBuildingsError?: boolean;
  reports: GuestReportSummaryItem[];
  isLoadingReports?: boolean;
  submittingReportId?: string;
  onPreview: (params: GuestReportParams) => Promise<GuestReportPreviewData>;
  onSaveReport: (params: GuestReportParams) => Promise<void>;
  onSubmitReport: (reportId: string) => void;
  onBack?: () => void;
  /** Override "now" (tests). */
  now?: Date;
}

const AUTHORITIES: GuestReportAuthority[] = ['SK_UHUL', 'CZ_CIZPOL', 'AT_ZMR', 'DE_MELDEWESEN'];
const PERIOD_TYPES: GuestReportPeriodType[] = ['monthly', 'quarterly', 'annual'];

const STATUS_STYLES: Record<string, string> = {
  draft: 'bg-gray-100 text-gray-700',
  generated: 'bg-yellow-100 text-yellow-800',
  submitted: 'bg-blue-100 text-blue-800',
  confirmed: 'bg-green-100 text-green-800',
};

export function TaxReportPage({
  buildings,
  isLoadingBuildings,
  isBuildingsError,
  reports,
  isLoadingReports,
  submittingReportId,
  onPreview,
  onSaveReport,
  onSubmitReport,
  onBack,
  now = new Date(),
}: TaxReportPageProps) {
  const { t, i18n } = useTranslation();
  const language = i18n.language || 'en';

  // Default to the previous month — the usual filing period.
  const lastMonth = new Date(now.getFullYear(), now.getMonth() - 1, 1);
  const yearOptions = [0, 1, 2, 3].map((n) => now.getFullYear() - n);

  const [buildingId, setBuildingId] = useState('');
  const [authorityCode, setAuthorityCode] = useState<GuestReportAuthority>('SK_UHUL');
  const [periodType, setPeriodType] = useState<GuestReportPeriodType>('monthly');
  const [year, setYear] = useState(lastMonth.getFullYear());
  const [monthIndex, setMonthIndex] = useState(lastMonth.getMonth() + 1);
  const [quarterIndex, setQuarterIndex] = useState(Math.floor(lastMonth.getMonth() / 3) + 1);

  const [preview, setPreview] = useState<GuestReportPreviewData | null>(null);
  const [previewParams, setPreviewParams] = useState<GuestReportParams | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [savedNotice, setSavedNotice] = useState(false);

  const monthFormat = new Intl.DateTimeFormat(language, { month: 'long' });
  const dateTimeFormat = new Intl.DateTimeFormat(language, { dateStyle: 'medium' });
  const numberFormat = new Intl.NumberFormat(language);

  const buildParams = (): GuestReportParams => {
    const index = periodType === 'monthly' ? monthIndex : quarterIndex;
    return {
      buildingId,
      authorityCode,
      periodType,
      ...computeReportPeriod(periodType, year, index),
    };
  };

  const handlePreview = async () => {
    if (!buildingId) return;
    setError(null);
    setSavedNotice(false);
    setIsGenerating(true);
    const params = buildParams();
    try {
      setPreview(await onPreview(params));
      setPreviewParams(params);
    } catch {
      setError(t('rentals.tax.errors.previewFailed'));
    } finally {
      setIsGenerating(false);
    }
  };

  const handleSave = async () => {
    if (!previewParams) return;
    setError(null);
    setIsSaving(true);
    try {
      await onSaveReport(previewParams);
      setSavedNotice(true);
    } catch {
      setError(t('rentals.tax.errors.saveFailed'));
    } finally {
      setIsSaving(false);
    }
  };

  const selectClass =
    'w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500';

  const renderConfig = () => {
    if (isLoadingBuildings) {
      return (
        <div className="p-6 flex justify-center" data-testid="buildings-loading">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600" />
        </div>
      );
    }
    if (isBuildingsError) {
      return <p className="p-6 text-sm text-red-700">{t('rentals.tax.errors.buildingsFailed')}</p>;
    }
    if (buildings.length === 0) {
      return (
        <div className="p-6 text-center">
          <p className="text-sm font-medium text-gray-900">{t('rentals.tax.noBuildings')}</p>
          <p className="mt-1 text-sm text-gray-500">{t('rentals.tax.noBuildingsDesc')}</p>
        </div>
      );
    }
    return (
      <div className="p-6 space-y-5">
        <div>
          <label htmlFor="report-building" className="block text-sm font-medium text-gray-700 mb-2">
            {t('rentals.tax.building')}
          </label>
          <select
            id="report-building"
            value={buildingId}
            onChange={(e) => setBuildingId(e.target.value)}
            className={selectClass}
          >
            <option value="">{t('rentals.tax.selectBuilding')}</option>
            {buildings.map((b) => (
              <option key={b.id} value={b.id}>
                {b.name}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label
            htmlFor="report-authority"
            className="block text-sm font-medium text-gray-700 mb-2"
          >
            {t('rentals.tax.authority')}
          </label>
          <select
            id="report-authority"
            value={authorityCode}
            onChange={(e) => setAuthorityCode(e.target.value as GuestReportAuthority)}
            className={selectClass}
          >
            {AUTHORITIES.map((a) => (
              <option key={a} value={a}>
                {t(`rentals.tax.authorities.${a}`)}
              </option>
            ))}
          </select>
        </div>

        <fieldset>
          <legend className="block text-sm font-medium text-gray-700 mb-2">
            {t('rentals.tax.periodType')}
          </legend>
          <div className="flex flex-wrap gap-2">
            {PERIOD_TYPES.map((p) => (
              <label
                key={p}
                className={`px-3 py-2 border rounded-md text-sm cursor-pointer ${
                  periodType === p
                    ? 'border-indigo-500 bg-indigo-50 text-indigo-800'
                    : 'border-gray-200 text-gray-700 hover:border-gray-300'
                }`}
              >
                <input
                  type="radio"
                  name="periodType"
                  value={p}
                  checked={periodType === p}
                  onChange={() => setPeriodType(p)}
                  className="sr-only"
                />
                {t(`rentals.tax.periodTypes.${p}`)}
              </label>
            ))}
          </div>
        </fieldset>

        <div className="grid grid-cols-2 gap-3">
          <div>
            <label htmlFor="report-year" className="block text-sm font-medium text-gray-700 mb-2">
              {t('rentals.tax.yearLabel')}
            </label>
            <select
              id="report-year"
              value={year}
              onChange={(e) => setYear(Number(e.target.value))}
              className={selectClass}
            >
              {yearOptions.map((y) => (
                <option key={y} value={y}>
                  {y}
                </option>
              ))}
            </select>
          </div>
          {periodType === 'monthly' && (
            <div>
              <label
                htmlFor="report-month"
                className="block text-sm font-medium text-gray-700 mb-2"
              >
                {t('rentals.tax.month')}
              </label>
              <select
                id="report-month"
                value={monthIndex}
                onChange={(e) => setMonthIndex(Number(e.target.value))}
                className={selectClass}
              >
                {Array.from({ length: 12 }, (_, i) => i + 1).map((m) => (
                  <option key={m} value={m}>
                    {monthFormat.format(new Date(2000, m - 1, 1))}
                  </option>
                ))}
              </select>
            </div>
          )}
          {periodType === 'quarterly' && (
            <div>
              <label
                htmlFor="report-quarter"
                className="block text-sm font-medium text-gray-700 mb-2"
              >
                {t('rentals.tax.quarter')}
              </label>
              <select
                id="report-quarter"
                value={quarterIndex}
                onChange={(e) => setQuarterIndex(Number(e.target.value))}
                className={selectClass}
              >
                {[1, 2, 3, 4].map((q) => (
                  <option key={q} value={q}>
                    {t('rentals.tax.quarterLabel', { quarter: q })}
                  </option>
                ))}
              </select>
            </div>
          )}
        </div>

        <button
          type="button"
          onClick={handlePreview}
          disabled={isGenerating || !buildingId}
          className="w-full px-4 py-3 text-sm font-medium text-white bg-indigo-600 rounded-md hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isGenerating ? t('rentals.tax.generating') : t('rentals.tax.showPreview')}
        </button>
      </div>
    );
  };

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex items-center gap-4">
            {onBack && (
              <button
                type="button"
                onClick={onBack}
                className="p-2 text-gray-400 hover:text-gray-600 rounded-full"
                aria-label={t('common.back')}
              >
                <svg
                  className="w-6 h-6"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M10 19l-7-7m0 0l7-7m-7 7h18"
                  />
                </svg>
              </button>
            )}
            <div>
              <h1 className="text-2xl font-bold text-gray-900">{t('rentals.tax.pageTitle')}</h1>
              <p className="mt-1 text-sm text-gray-500">{t('rentals.tax.pageSubtitle')}</p>
            </div>
          </div>
        </div>
      </div>

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {error && (
          <div
            className="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg flex items-center gap-3"
            role="alert"
          >
            <span className="text-sm text-red-700">{error}</span>
            <button
              type="button"
              onClick={() => setError(null)}
              className="ml-auto text-sm text-red-500 hover:text-red-700"
            >
              {t('common.close')}
            </button>
          </div>
        )}
        {savedNotice && (
          <div
            className="mb-6 p-4 bg-green-50 border border-green-200 rounded-lg text-sm text-green-800"
            role="status"
          >
            {t('rentals.tax.saved')}
          </div>
        )}

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          <div className="lg:col-span-1">
            <div className="bg-white rounded-lg shadow">
              <div className="px-6 py-4 border-b">
                <h2 className="text-lg font-semibold text-gray-900">
                  {t('rentals.tax.configureReport')}
                </h2>
              </div>
              {renderConfig()}
            </div>
            <p className="mt-4 text-xs text-gray-500">{t('rentals.tax.incomeNotAvailable')}</p>
          </div>

          <div className="lg:col-span-2 space-y-8">
            {preview ? (
              <TaxReportPreview
                report={preview}
                onSave={handleSave}
                onClose={() => {
                  setPreview(null);
                  setSavedNotice(false);
                }}
                isSaving={isSaving}
              />
            ) : (
              <div className="bg-white rounded-lg shadow p-12 text-center">
                <h3 className="text-lg font-medium text-gray-900 mb-2">
                  {t('rentals.tax.noReportGenerated')}
                </h3>
                <p className="text-sm text-gray-500 max-w-sm mx-auto">
                  {t('rentals.tax.noReportGeneratedDesc')}
                </p>
              </div>
            )}

            {/* Stored reports */}
            <div className="bg-white rounded-lg shadow">
              <div className="px-6 py-4 border-b">
                <h2 className="text-lg font-semibold text-gray-900">
                  {t('rentals.tax.savedReports')}
                </h2>
              </div>
              {isLoadingReports ? (
                <div className="p-6 flex justify-center" data-testid="reports-loading">
                  <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-indigo-600" />
                </div>
              ) : reports.length === 0 ? (
                <p className="p-6 text-sm text-gray-500" data-testid="no-saved-reports">
                  {t('rentals.tax.noSavedReports')}
                </p>
              ) : (
                <div className="overflow-x-auto">
                  <table className="min-w-full text-sm">
                    <thead>
                      <tr className="text-left text-gray-500 border-b">
                        <th className="px-6 py-2 font-medium">{t('rentals.tax.period')}</th>
                        <th className="px-3 py-2 font-medium">{t('rentals.tax.building')}</th>
                        <th className="px-3 py-2 font-medium">{t('rentals.tax.authority')}</th>
                        <th className="px-3 py-2 font-medium text-right">
                          {t('rentals.tax.guests')}
                        </th>
                        <th className="px-3 py-2 font-medium">{t('rentals.tax.status')}</th>
                        <th className="px-6 py-2 font-medium text-right">
                          <span className="sr-only">{t('rentals.tax.actions')}</span>
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      {reports.map((r) => {
                        const canSubmit = r.status === 'generated' || r.status === 'draft';
                        return (
                          <tr
                            key={r.id}
                            className="border-b last:border-0"
                            data-testid="saved-report-row"
                          >
                            <td className="px-6 py-3 whitespace-nowrap">
                              {formatReportPeriod(r.periodStart, r.periodEnd, language)}
                            </td>
                            <td className="px-3 py-3">{r.buildingName}</td>
                            <td className="px-3 py-3">
                              {t(`rentals.tax.authorities.${r.authorityCode}`, {
                                defaultValue: r.authorityName,
                              })}
                            </td>
                            <td className="px-3 py-3 text-right tabular-nums">
                              {numberFormat.format(r.totalGuests)}
                            </td>
                            <td className="px-3 py-3">
                              <span
                                className={`inline-flex px-2 py-0.5 rounded-full text-xs font-medium ${
                                  STATUS_STYLES[r.status] ?? STATUS_STYLES.draft
                                }`}
                              >
                                {t(`rentals.tax.statuses.${r.status}`, { defaultValue: r.status })}
                              </span>
                              {r.submittedAt && (
                                <span className="block mt-1 text-xs text-gray-500">
                                  {dateTimeFormat.format(new Date(r.submittedAt))}
                                </span>
                              )}
                            </td>
                            <td className="px-6 py-3 text-right whitespace-nowrap space-x-3">
                              {r.fileUrl && (
                                <a
                                  href={r.fileUrl}
                                  target="_blank"
                                  rel="noopener noreferrer"
                                  className="text-indigo-600 hover:text-indigo-800"
                                >
                                  {t('rentals.tax.download')}
                                </a>
                              )}
                              {canSubmit && (
                                <button
                                  type="button"
                                  onClick={() => onSubmitReport(r.id)}
                                  disabled={submittingReportId === r.id}
                                  className="text-indigo-600 hover:text-indigo-800 disabled:opacity-50"
                                >
                                  {t('rentals.tax.markSubmitted')}
                                </button>
                              )}
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
