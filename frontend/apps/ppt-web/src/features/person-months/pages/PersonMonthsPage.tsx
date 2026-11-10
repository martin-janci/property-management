/**
 * PersonMonthsPage - building-level summary with all units.
 * Shows person-month data for all units in a building for a selected month.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { type PersonMonth, PersonMonthCard } from '../components/PersonMonthCard';
import type { ExportFormat } from '../types';

export interface UnitPersonMonthSummary {
  unitId: string;
  unitDesignation: string;
  personCount: number;
  updatedAt?: string;
}

export interface BuildingPersonMonthSummary {
  buildingId: string;
  buildingName: string;
  year: number;
  month: number;
  totalPersons: number;
  units: UnitPersonMonthSummary[];
}

interface PersonMonthsPageProps {
  buildingName: string;
  summary: BuildingPersonMonthSummary;
  isLoading?: boolean;
  isExporting?: boolean;
  onYearChange: (year: number) => void;
  onMonthChange: (month: number) => void;
  onNavigateToUnit: (unitId: string) => void;
  /** Managers only; omitted for residents so the cards are read-only. */
  onNavigateToEdit?: (unitId: string, year: number, month: number) => void;
  onNavigateToBulkEntry?: () => void;
  onExport?: (format: ExportFormat, year: number, month: number) => void;
  onBack: () => void;
}

const currentYear = new Date().getFullYear();
const currentMonth = new Date().getMonth() + 1;

const monthOptions = [
  { value: 1, key: 'january' },
  { value: 2, key: 'february' },
  { value: 3, key: 'march' },
  { value: 4, key: 'april' },
  { value: 5, key: 'may' },
  { value: 6, key: 'june' },
  { value: 7, key: 'july' },
  { value: 8, key: 'august' },
  { value: 9, key: 'september' },
  { value: 10, key: 'october' },
  { value: 11, key: 'november' },
  { value: 12, key: 'december' },
];

const yearOptions = Array.from({ length: 5 }, (_, i) => currentYear - 2 + i);

export function PersonMonthsPage({
  buildingName,
  summary,
  isLoading,
  isExporting,
  onYearChange,
  onMonthChange,
  onNavigateToUnit,
  onNavigateToEdit,
  onNavigateToBulkEntry,
  onExport,
  onBack,
}: PersonMonthsPageProps) {
  const { t } = useTranslation();
  const [selectedYear, setSelectedYear] = useState(summary.year || currentYear);
  const [selectedMonth, setSelectedMonth] = useState(summary.month || currentMonth);
  const [showExportMenu, setShowExportMenu] = useState(false);

  const handleYearChange = (year: number) => {
    setSelectedYear(year);
    onYearChange(year);
  };

  const handleMonthChange = (month: number) => {
    setSelectedMonth(month);
    onMonthChange(month);
  };

  const handleExport = (format: ExportFormat) => {
    setShowExportMenu(false);
    onExport?.(format, selectedYear, selectedMonth);
  };

  const convertToPersonMonth = (unit: UnitPersonMonthSummary): PersonMonth => ({
    id: `${unit.unitId}-${selectedYear}-${selectedMonth}`,
    unitId: unit.unitId,
    unitDesignation: unit.unitDesignation,
    year: selectedYear,
    month: selectedMonth,
    personCount: unit.personCount,
    updatedAt: unit.updatedAt || new Date().toISOString(),
  });

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
        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">{t('personMonths.title')}</h1>
            <p className="text-gray-600 mt-1">{buildingName}</p>
          </div>
          <div className="flex items-center gap-2">
            {onNavigateToBulkEntry && (
              <button
                type="button"
                onClick={onNavigateToBulkEntry}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
              >
                {t('personMonths.bulkEntry')}
              </button>
            )}
            {onExport && (
              <div className="relative">
                <button
                  type="button"
                  onClick={() => setShowExportMenu(!showExportMenu)}
                  disabled={isExporting}
                  className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 flex items-center gap-2"
                >
                  {isExporting && (
                    <span className="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-600" />
                  )}
                  {t('personMonths.export')}
                </button>
                {showExportMenu && (
                  <div className="absolute right-0 mt-2 w-48 bg-white rounded-md shadow-lg ring-1 ring-black ring-opacity-5 z-10">
                    <div className="py-1">
                      <button
                        type="button"
                        onClick={() => handleExport('csv')}
                        className="block w-full px-4 py-2 text-sm text-gray-700 text-left hover:bg-gray-100"
                      >
                        {t('personMonths.exportCsv')}
                      </button>
                      <button
                        type="button"
                        onClick={() => handleExport('excel')}
                        className="block w-full px-4 py-2 text-sm text-gray-700 text-left hover:bg-gray-100"
                      >
                        {t('personMonths.exportExcel')}
                      </button>
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Filters */}
      <div className="bg-white rounded-lg shadow p-4 mb-6">
        <div className="flex flex-wrap items-center gap-4">
          <div>
            <label htmlFor="year-filter" className="block text-sm font-medium text-gray-700 mb-1">
              {t('personMonths.form.year')}
            </label>
            <select
              id="year-filter"
              value={selectedYear}
              onChange={(e) => handleYearChange(Number.parseInt(e.target.value, 10))}
              className="px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
            >
              {yearOptions.map((year) => (
                <option key={year} value={year}>
                  {year}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label htmlFor="month-filter" className="block text-sm font-medium text-gray-700 mb-1">
              {t('personMonths.form.month')}
            </label>
            <select
              id="month-filter"
              value={selectedMonth}
              onChange={(e) => handleMonthChange(Number.parseInt(e.target.value, 10))}
              className="px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
            >
              {monthOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {t(`personMonths.months.${option.key}`)}
                </option>
              ))}
            </select>
          </div>

          <div className="ml-auto text-right">
            <div className="text-sm text-gray-500">{t('personMonths.totalPersons')}</div>
            <div className="text-2xl font-bold text-blue-600">{summary.totalPersons}</div>
          </div>
        </div>
      </div>

      {/* Loading state */}
      {isLoading && (
        <div className="flex justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      )}

      {/* Units list */}
      {!isLoading &&
        (summary.units.length === 0 ? (
          <div className="bg-white rounded-lg shadow p-8 text-center">
            <p className="text-gray-500">{t('personMonths.noData')}</p>
            <p className="text-sm text-gray-400 mt-2">{t('personMonths.noDataDescription')}</p>
          </div>
        ) : (
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {summary.units.map((unit) => (
              <PersonMonthCard
                key={unit.unitId}
                personMonth={convertToPersonMonth(unit)}
                onView={() => onNavigateToUnit(unit.unitId)}
                onEdit={
                  onNavigateToEdit
                    ? () => onNavigateToEdit(unit.unitId, selectedYear, selectedMonth)
                    : undefined
                }
              />
            ))}
          </div>
        ))}

      {/* Summary footer */}
      {!isLoading && summary.units.length > 0 && (
        <div className="mt-6 bg-gray-50 rounded-lg p-4 text-sm text-gray-600">
          <div className="flex justify-between items-center">
            <span>{t('personMonths.unitsWithData', { count: summary.units.length })}</span>
            <span>
              {t('personMonths.averagePerUnit', {
                average:
                  summary.units.length > 0
                    ? (summary.totalPersons / summary.units.length).toFixed(1)
                    : '0',
              })}
            </span>
          </div>
        </div>
      )}

      {/* Click outside to close export menu */}
      {showExportMenu && (
        <div
          className="fixed inset-0 z-0"
          onClick={() => setShowExportMenu(false)}
          onKeyDown={(e) => e.key === 'Escape' && setShowExportMenu(false)}
        />
      )}
    </div>
  );
}
