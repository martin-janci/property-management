/**
 * BulkEntryPage - bulk entry form for multiple units.
 * Allows entering person counts for all units in a building for a specific month.
 */

import { type FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { BulkEntryFormData, BulkEntrySummary, BulkEntryUnit } from '../types';

interface BulkEntryPageProps {
  buildingName: string;
  units: BulkEntryUnit[];
  initialYear?: number;
  initialMonth?: number;
  isSubmitting?: boolean;
  submitProgress?: number;
  submitResults?: BulkEntrySummary;
  onSubmit: (data: BulkEntryFormData) => void;
  onBack: () => void;
  onDismissResults?: () => void;
}

const currentYear = new Date().getFullYear();
const currentMonth = new Date().getMonth() + 1;

const monthValues = Array.from({ length: 12 }, (_, i) => i + 1);

/**
 * Localized month name (1-12) from `Intl`, so the dropdown follows the UI
 * language without per-locale month keys.
 */
export function monthName(month: number, locale: string): string {
  const name = new Intl.DateTimeFormat(locale, { month: 'long' }).format(
    new Date(2000, month - 1, 1)
  );
  return name.charAt(0).toLocaleUpperCase(locale) + name.slice(1);
}

const yearOptions = Array.from({ length: 5 }, (_, i) => currentYear - 2 + i);

export function BulkEntryPage({
  buildingName,
  units,
  initialYear,
  initialMonth,
  isSubmitting,
  submitProgress,
  submitResults,
  onSubmit,
  onBack,
  onDismissResults,
}: BulkEntryPageProps) {
  const { t, i18n } = useTranslation();
  const [year, setYear] = useState(initialYear ?? currentYear);
  const [month, setMonth] = useState(initialMonth ?? currentMonth);
  // Only the counts the user changed. Units (and their current counts) load
  // asynchronously, so anything not edited falls back to the unit's value —
  // seeding a map once at mount left late-loading units without a count and
  // flagged them as invalid.
  const [entries, setEntries] = useState<Map<string, number>>(() => new Map());
  const countFor = (unitId: string): number => {
    const edited = entries.get(unitId);
    if (edited !== undefined) return edited;
    const unit = units.find((u) => u.unitId === unitId);
    return unit?.currentPersonCount ?? unit?.personCount ?? 0;
  };
  const [errors, setErrors] = useState<Map<string, string>>(new Map());
  const [showConfirmation, setShowConfirmation] = useState(false);

  const handlePersonCountChange = (unitId: string, value: string) => {
    const count = Number.parseInt(value, 10);
    if (!Number.isNaN(count) && count >= 0 && count <= 100) {
      setEntries((prev) => new Map(prev).set(unitId, count));
      setErrors((prev) => {
        const next = new Map(prev);
        next.delete(unitId);
        return next;
      });
    }
  };

  const incrementCount = (unitId: string) => {
    const current = countFor(unitId);
    if (current < 100) {
      setEntries((prev) => new Map(prev).set(unitId, current + 1));
    }
  };

  const decrementCount = (unitId: string) => {
    const current = countFor(unitId);
    if (current > 0) {
      setEntries((prev) => new Map(prev).set(unitId, current - 1));
    }
  };

  const validate = (): boolean => {
    const newErrors = new Map<string, string>();

    for (const unit of units) {
      const count = countFor(unit.unitId);
      if (!Number.isFinite(count) || count < 0) {
        newErrors.set(unit.unitId, t('personMonths.errors.personCountInvalid'));
      } else if (count > 100) {
        newErrors.set(unit.unitId, t('personMonths.errors.personCountTooHigh'));
      }
    }

    setErrors(newErrors);
    return newErrors.size === 0;
  };

  const handleSubmitClick = (e: FormEvent) => {
    e.preventDefault();
    if (validate()) {
      setShowConfirmation(true);
    }
  };

  const handleConfirmSubmit = () => {
    setShowConfirmation(false);
    const data: BulkEntryFormData = {
      year,
      month,
      entries: units.map((unit) => ({
        unitId: unit.unitId,
        unitDesignation: unit.unitDesignation,
        personCount: countFor(unit.unitId),
      })),
    };
    onSubmit(data);
  };

  const handleCancelConfirmation = () => {
    setShowConfirmation(false);
  };

  const counts = units.map((unit) => countFor(unit.unitId));
  const totalPersons = counts.reduce((sum, count) => sum + count, 0);
  const unitsWithData = counts.filter((count) => count > 0).length;

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onBack}
          className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 mb-4"
        >
          &larr; {t('common.back')}
        </button>
        <h1 className="text-2xl font-bold text-gray-900">{t('personMonths.bulkEntry')}</h1>
        <p className="text-gray-600 mt-1">{buildingName}</p>
      </div>

      {/* Results Summary */}
      {submitResults && (
        <div className="mb-6 bg-white rounded-lg shadow p-6">
          <div className="flex items-start justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">
              {t('personMonths.bulkEntryResults')}
            </h2>
            {onDismissResults && (
              <button
                type="button"
                onClick={onDismissResults}
                className="text-gray-400 hover:text-gray-600"
              >
                <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                  <path
                    fillRule="evenodd"
                    d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                    clipRule="evenodd"
                  />
                </svg>
              </button>
            )}
          </div>

          {/* Summary Stats */}
          <div className="grid grid-cols-3 gap-4 mb-4">
            <div className="bg-gray-50 rounded p-3 text-center">
              <div className="text-2xl font-bold text-gray-900">{submitResults.total}</div>
              <div className="text-sm text-gray-600">{t('personMonths.bulkEntryTotal')}</div>
            </div>
            <div className="bg-green-50 rounded p-3 text-center">
              <div className="text-2xl font-bold text-green-600">{submitResults.successful}</div>
              <div className="text-sm text-gray-600">{t('personMonths.bulkEntrySuccessful')}</div>
            </div>
            <div className="bg-red-50 rounded p-3 text-center">
              <div className="text-2xl font-bold text-red-600">{submitResults.failed}</div>
              <div className="text-sm text-gray-600">{t('personMonths.bulkEntryFailed')}</div>
            </div>
          </div>

          {/* Detailed Results */}
          {submitResults.failed > 0 && (
            <div className="border-t pt-4">
              <h3 className="text-sm font-medium text-gray-700 mb-2">
                {t('personMonths.bulkEntryFailedItems')}
              </h3>
              <div className="max-h-40 overflow-y-auto space-y-2">
                {submitResults.results
                  .filter((r) => !r.success)
                  .map((result) => (
                    <div
                      key={result.unitId}
                      className="flex items-center justify-between text-sm bg-red-50 p-2 rounded"
                    >
                      <span className="font-medium">{result.unitDesignation}</span>
                      <span className="text-red-600">{result.error}</span>
                    </div>
                  ))}
              </div>
            </div>
          )}
        </div>
      )}

      {/* Description */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
        <p className="text-sm text-blue-800">{t('personMonths.bulkEntryDescription')}</p>
      </div>

      {/* Form */}
      <form onSubmit={handleSubmitClick}>
        {/* Month/Year Selector */}
        <div className="bg-white rounded-lg shadow p-4 mb-6">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label htmlFor="bulk-year" className="block text-sm font-medium text-gray-700 mb-1">
                {t('personMonths.form.year')} *
              </label>
              <select
                id="bulk-year"
                value={year}
                onChange={(e) => setYear(Number.parseInt(e.target.value, 10))}
                disabled={isSubmitting}
                className="block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 disabled:opacity-50"
              >
                {yearOptions.map((y) => (
                  <option key={y} value={y}>
                    {y}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label htmlFor="bulk-month" className="block text-sm font-medium text-gray-700 mb-1">
                {t('personMonths.form.month')} *
              </label>
              <select
                id="bulk-month"
                value={month}
                onChange={(e) => setMonth(Number.parseInt(e.target.value, 10))}
                disabled={isSubmitting}
                className="block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 disabled:opacity-50"
              >
                {monthValues.map((value) => (
                  <option key={value} value={value}>
                    {monthName(value, i18n.language)}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </div>

        {/* Units Table */}
        <div className="bg-white rounded-lg shadow overflow-hidden mb-6">
          <div className="px-4 py-3 bg-gray-50 border-b">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-medium text-gray-700">
                {t('personMonths.bulkEntryUnits')} ({units.length})
              </h2>
              <div className="text-sm text-gray-500">
                {t('personMonths.totalPersons')}: <span className="font-bold">{totalPersons}</span>
              </div>
            </div>
          </div>

          <div className="overflow-x-auto">
            <table className="min-w-full divide-y divide-gray-200">
              <thead className="bg-gray-50">
                <tr>
                  <th
                    scope="col"
                    className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
                  >
                    {t('personMonths.unit')}
                  </th>
                  <th
                    scope="col"
                    className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
                  >
                    {t('personMonths.currentValue')}
                  </th>
                  <th
                    scope="col"
                    className="px-6 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider"
                  >
                    {t('personMonths.form.personCount')}
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-gray-200">
                {units.map((unit) => {
                  const error = errors.get(unit.unitId);
                  const currentValue = countFor(unit.unitId);

                  return (
                    <tr key={unit.unitId} className={error ? 'bg-red-50' : ''}>
                      <td className="px-6 py-4 whitespace-nowrap">
                        <span className="text-sm font-medium text-gray-900">
                          {unit.unitDesignation}
                        </span>
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap">
                        <span className="text-sm text-gray-500">
                          {unit.currentPersonCount !== undefined
                            ? unit.currentPersonCount
                            : t('personMonths.noDataShort')}
                        </span>
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap">
                        <div className="flex items-center justify-center gap-2">
                          <button
                            type="button"
                            onClick={() => decrementCount(unit.unitId)}
                            disabled={isSubmitting || currentValue <= 0}
                            className="w-8 h-8 flex items-center justify-center rounded-full border border-gray-300 bg-white hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                          >
                            <span className="text-lg font-medium">-</span>
                          </button>
                          <input
                            type="number"
                            min={0}
                            max={100}
                            value={currentValue}
                            onChange={(e) => handlePersonCountChange(unit.unitId, e.target.value)}
                            disabled={isSubmitting}
                            className={`w-16 px-2 py-1 text-center text-sm border rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 disabled:opacity-50 ${
                              error ? 'border-red-300' : 'border-gray-300'
                            }`}
                          />
                          <button
                            type="button"
                            onClick={() => incrementCount(unit.unitId)}
                            disabled={isSubmitting || currentValue >= 100}
                            className="w-8 h-8 flex items-center justify-center rounded-full border border-gray-300 bg-white hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                          >
                            <span className="text-lg font-medium">+</span>
                          </button>
                        </div>
                        {error && <p className="mt-1 text-xs text-red-600 text-center">{error}</p>}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>

        {/* Progress indicator */}
        {isSubmitting && submitProgress !== undefined && (
          <div className="mb-6">
            <div className="flex items-center justify-between text-sm text-gray-600 mb-2">
              <span>{t('personMonths.bulkEntryProgress')}</span>
              <span>{submitProgress}%</span>
            </div>
            <div className="w-full bg-gray-200 rounded-full h-2">
              <div
                className="bg-blue-600 h-2 rounded-full transition-all duration-300"
                style={{ width: `${submitProgress}%` }}
              />
            </div>
          </div>
        )}

        {/* Summary */}
        <div className="bg-gray-50 rounded-lg p-4 mb-6">
          <div className="flex justify-between items-center text-sm">
            <span className="text-gray-600">
              {t('personMonths.bulkEntrySummary', { count: unitsWithData })}
            </span>
            <span className="font-medium text-gray-900">
              {t('personMonths.totalPersons')}: {totalPersons}
            </span>
          </div>
        </div>

        {/* Actions */}
        <div className="flex items-center justify-end gap-3">
          <button
            type="button"
            onClick={onBack}
            disabled={isSubmitting}
            className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
          >
            {t('common.cancel')}
          </button>
          <button
            type="submit"
            disabled={isSubmitting || units.length === 0}
            className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50 flex items-center gap-2"
          >
            {isSubmitting && (
              <span className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
            )}
            {t('personMonths.submitAll')}
          </button>
        </div>
      </form>

      {/* Confirmation Modal */}
      {showConfirmation && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg shadow-xl p-6 max-w-md mx-4">
            <h3 className="text-lg font-semibold text-gray-900 mb-2">
              {t('personMonths.bulkEntryConfirmTitle')}
            </h3>
            <p className="text-gray-600 mb-4">
              {t('personMonths.bulkEntryConfirmMessage', {
                count: units.length,
                month: monthName(month, i18n.language),
                year,
              })}
            </p>
            <div className="bg-gray-50 rounded p-3 mb-4">
              <div className="flex justify-between text-sm">
                <span className="text-gray-600">{t('personMonths.bulkEntryUnitsCount')}</span>
                <span className="font-medium">{units.length}</span>
              </div>
              <div className="flex justify-between text-sm mt-1">
                <span className="text-gray-600">{t('personMonths.totalPersons')}</span>
                <span className="font-medium">{totalPersons}</span>
              </div>
            </div>
            <div className="flex justify-end gap-3">
              <button
                type="button"
                onClick={handleCancelConfirmation}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                onClick={handleConfirmSubmit}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
              >
                {t('common.confirm')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
