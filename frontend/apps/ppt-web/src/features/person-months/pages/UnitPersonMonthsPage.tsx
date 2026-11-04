/**
 * UnitPersonMonthsPage - unit-level person months history.
 * Shows all person-month entries for a specific unit with yearly summary.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { type PersonMonth, PersonMonthCard } from '../components/PersonMonthCard';
import { type YearlySummary, YearlySummaryChart } from '../components/YearlySummaryChart';
import type { ExportFormat } from '../types';

interface UnitPersonMonthsPageProps {
  buildingName: string;
  unitDesignation: string;
  personMonths: PersonMonth[];
  yearlySummary: YearlySummary;
  isLoading?: boolean;
  isExporting?: boolean;
  isDeleting?: boolean;
  selectedYear: number;
  onYearChange: (year: number) => void;
  /** Managers only; omitted for residents so the page is read-only. */
  onNavigateToEdit?: (year: number, month: number) => void;
  onDelete?: (personMonth: PersonMonth) => void;
  onExport?: (format: ExportFormat, year: number) => void;
  onBack: () => void;
}

const currentYear = new Date().getFullYear();
const yearOptions = Array.from({ length: 5 }, (_, i) => currentYear - 2 + i);

export function UnitPersonMonthsPage({
  buildingName,
  unitDesignation,
  personMonths,
  yearlySummary,
  isLoading,
  isExporting,
  isDeleting,
  selectedYear,
  onYearChange,
  onNavigateToEdit,
  onDelete,
  onExport,
  onBack,
}: UnitPersonMonthsPageProps) {
  const { t } = useTranslation();
  const [year, setYear] = useState(selectedYear);
  const [showExportMenu, setShowExportMenu] = useState(false);

  const handleYearChange = (newYear: number) => {
    setYear(newYear);
    onYearChange(newYear);
  };

  const handleMonthClick = (month: number) => {
    onNavigateToEdit?.(year, month);
  };

  const handleExport = (format: ExportFormat) => {
    setShowExportMenu(false);
    onExport?.(format, year);
  };

  const handleDelete = (personMonth: PersonMonth) => {
    onDelete?.(personMonth);
  };

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onBack}
          className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 mb-4"
        >
          &larr; {t('personMonths.backToBuilding')}
        </button>
        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">
              {t('personMonths.unitHistory')} - {unitDesignation}
            </h1>
            <p className="text-gray-600 mt-1">{buildingName}</p>
          </div>
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

      {/* Year selector */}
      <div className="bg-white rounded-lg shadow p-4 mb-6">
        <div className="flex items-center gap-4">
          <label htmlFor="year-select" className="text-sm font-medium text-gray-700">
            {t('personMonths.form.year')}:
          </label>
          <select
            id="year-select"
            value={year}
            onChange={(e) => handleYearChange(Number.parseInt(e.target.value, 10))}
            className="px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
          >
            {yearOptions.map((y) => (
              <option key={y} value={y}>
                {y}
              </option>
            ))}
          </select>
          {onNavigateToEdit && (
            <button
              type="button"
              onClick={() => onNavigateToEdit(year, new Date().getMonth() + 1)}
              className="ml-auto px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
            >
              {t('personMonths.addEntry')}
            </button>
          )}
        </div>
      </div>

      {/* Loading state */}
      {isLoading && (
        <div className="flex justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      )}

      {/* Deleting state overlay */}
      {isDeleting && (
        <div className="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-40">
          <div className="bg-white rounded-lg shadow-xl p-6 flex items-center gap-3">
            <span className="animate-spin rounded-full h-5 w-5 border-b-2 border-blue-600" />
            <span className="text-gray-700">{t('personMonths.deleting')}</span>
          </div>
        </div>
      )}

      {/* Yearly summary chart */}
      {!isLoading && yearlySummary && (
        <div className="mb-6">
          <YearlySummaryChart
            summary={yearlySummary}
            onMonthClick={onNavigateToEdit ? handleMonthClick : undefined}
          />
        </div>
      )}

      {/* Monthly entries list */}
      {!isLoading && (
        <>
          <h2 className="text-lg font-medium text-gray-900 mb-4">
            {t('personMonths.monthlyEntries')}
          </h2>
          {personMonths.length === 0 ? (
            <div className="bg-white rounded-lg shadow p-8 text-center">
              <p className="text-gray-500">{t('personMonths.noEntriesForYear')}</p>
              {onNavigateToEdit && (
                <p className="text-sm text-gray-400 mt-2">{t('personMonths.clickAddToCreate')}</p>
              )}
            </div>
          ) : (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {personMonths.map((pm) => (
                <PersonMonthCard
                  key={pm.id}
                  personMonth={pm}
                  onEdit={onNavigateToEdit ? () => onNavigateToEdit(pm.year, pm.month) : undefined}
                  onDelete={onDelete ? () => handleDelete(pm) : undefined}
                />
              ))}
            </div>
          )}
        </>
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
