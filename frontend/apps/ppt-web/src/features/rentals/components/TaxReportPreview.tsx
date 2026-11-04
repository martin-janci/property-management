/**
 * TaxReportPreview - Daňový výkaz preview: guests and reservations for one
 * building and period, broken down by nationality.
 * Epic 18: Short-Term Rental Integration - UC-29 (Story 18.4)
 *
 * Data comes from `POST /rentals/reports/preview`. Only guests with a
 * completed registration whose stay lies fully inside the period are counted.
 */

import { useTranslation } from 'react-i18next';
import { localizedCountryName, parseIsoDate } from '../mappers';
import type { GuestReportPreviewData } from '../types';

interface TaxReportPreviewProps {
  report: GuestReportPreviewData;
  onSave?: () => void;
  onClose?: () => void;
  isSaving?: boolean;
}

export function formatReportPeriod(start: string, end: string, language: string): string {
  const fmt = new Intl.DateTimeFormat(language, {
    day: 'numeric',
    month: 'numeric',
    year: 'numeric',
  });
  return `${fmt.format(parseIsoDate(start))} – ${fmt.format(parseIsoDate(end))}`;
}

export function TaxReportPreview({ report, onSave, onClose, isSaving }: TaxReportPreviewProps) {
  const { t, i18n } = useTranslation();
  const language = i18n.language || 'en';
  const numberFormat = new Intl.NumberFormat(language);
  const percentFormat = new Intl.NumberFormat(language, {
    style: 'percent',
    maximumFractionDigits: 1,
  });
  const countries = report.byNationality.filter((r) => r.nationality.toUpperCase() !== 'UNK');

  return (
    <div className="bg-white rounded-lg shadow" data-testid="tax-report-preview">
      <div className="px-6 py-4 border-b flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold text-gray-900">{t('rentals.tax.previewTitle')}</h2>
          <p className="mt-1 text-sm text-gray-600">
            {report.buildingName} ·{' '}
            {formatReportPeriod(report.periodStart, report.periodEnd, language)}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {onSave && (
            <button
              type="button"
              onClick={onSave}
              disabled={isSaving}
              className="px-4 py-2 text-sm font-medium text-white bg-indigo-600 rounded-md hover:bg-indigo-700 disabled:opacity-50"
            >
              {isSaving ? t('rentals.tax.saving') : t('rentals.tax.saveReport')}
            </button>
          )}
          {onClose && (
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
            >
              {t('common.close')}
            </button>
          )}
        </div>
      </div>

      <div className="p-6">
        <dl className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div className="p-4 bg-indigo-50 rounded-lg">
            <dt className="text-sm text-indigo-700">{t('rentals.tax.totalGuests')}</dt>
            <dd className="mt-1 text-2xl font-semibold text-indigo-900" data-testid="total-guests">
              {numberFormat.format(report.totalGuests)}
            </dd>
          </div>
          <div className="p-4 bg-gray-50 rounded-lg">
            <dt className="text-sm text-gray-600">{t('rentals.tax.totalBookings')}</dt>
            <dd className="mt-1 text-2xl font-semibold text-gray-900" data-testid="total-bookings">
              {numberFormat.format(report.bookingsCount)}
            </dd>
          </div>
          <div className="p-4 bg-gray-50 rounded-lg">
            <dt className="text-sm text-gray-600">{t('rentals.tax.countries')}</dt>
            <dd className="mt-1 text-2xl font-semibold text-gray-900">
              {numberFormat.format(countries.length)}
            </dd>
          </div>
        </dl>

        <h3 className="mt-8 text-sm font-semibold text-gray-900">
          {t('rentals.tax.byNationality')}
        </h3>
        {report.byNationality.length === 0 ? (
          <p className="mt-3 text-sm text-gray-500" data-testid="no-guests">
            {t('rentals.tax.noGuestsInPeriod')}
          </p>
        ) : (
          <div className="mt-3 overflow-x-auto">
            <table className="min-w-full text-sm">
              <thead>
                <tr className="text-left text-gray-500 border-b">
                  <th className="py-2 pr-4 font-medium">{t('rentals.tax.nationality')}</th>
                  <th className="py-2 pr-4 font-medium text-right">{t('rentals.tax.guests')}</th>
                  <th className="py-2 font-medium text-right">{t('rentals.tax.share')}</th>
                </tr>
              </thead>
              <tbody>
                {report.byNationality.map((row) => (
                  <tr key={row.nationality} className="border-b last:border-0">
                    <td className="py-2 pr-4 text-gray-900">
                      {localizedCountryName(row.nationality, row.countryName, language) ??
                        t('rentals.tax.unknownNationality')}
                    </td>
                    <td className="py-2 pr-4 text-right tabular-nums">
                      {numberFormat.format(row.count)}
                    </td>
                    <td className="py-2 text-right tabular-nums">
                      {percentFormat.format(row.percentage / 100)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        <p className="mt-6 text-xs text-gray-500">{t('rentals.tax.countingRule')}</p>
      </div>
    </div>
  );
}
