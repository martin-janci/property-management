/**
 * PersonMonthCard component - displays a person-month entry.
 * Shows the number of persons for a specific unit and month.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';

export interface PersonMonth {
  id: string;
  unitId: string;
  unitDesignation: string;
  year: number;
  month: number;
  personCount: number;
  updatedAt: string;
  updatedBy?: string;
}

interface PersonMonthCardProps {
  personMonth: PersonMonth;
  onEdit?: (personMonth: PersonMonth) => void;
  onView?: (personMonth: PersonMonth) => void;
  onDelete?: (personMonth: PersonMonth) => void;
}

const monthNames = [
  'january',
  'february',
  'march',
  'april',
  'may',
  'june',
  'july',
  'august',
  'september',
  'october',
  'november',
  'december',
];

export function PersonMonthCard({ personMonth, onEdit, onView, onDelete }: PersonMonthCardProps) {
  const { t } = useTranslation();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const monthKey = monthNames[personMonth.month - 1];
  const monthLabel = t(`personMonths.months.${monthKey}`);

  const handleDeleteClick = () => {
    setShowDeleteConfirm(true);
  };

  const handleConfirmDelete = () => {
    setShowDeleteConfirm(false);
    onDelete?.(personMonth);
  };

  const handleCancelDelete = () => {
    setShowDeleteConfirm(false);
  };

  return (
    <>
      <div className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
        <div className="flex items-start justify-between">
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <span className="text-2xl font-bold text-blue-600">{personMonth.personCount}</span>
              <span className="text-gray-600">
                {personMonth.personCount === 1
                  ? t('personMonths.person')
                  : t('personMonths.persons')}
              </span>
            </div>
            <div className="mt-2 text-sm text-gray-700">
              <span className="font-medium">{monthLabel}</span>
              <span className="mx-1">{personMonth.year}</span>
            </div>
            {personMonth.unitDesignation && (
              <div className="mt-1 text-sm text-gray-500">
                {t('personMonths.unit')}: {personMonth.unitDesignation}
              </div>
            )}
            <p className="mt-2 text-xs text-gray-400">
              {t('personMonths.lastUpdated')}:{' '}
              {new Date(personMonth.updatedAt).toLocaleDateString(getActiveLocale())}
            </p>
          </div>
        </div>

        <div className="mt-4 flex items-center gap-2 border-t pt-3">
          {onView && (
            <button
              type="button"
              onClick={() => onView(personMonth)}
              className="text-sm text-blue-600 hover:text-blue-800"
            >
              {t('common.view')}
            </button>
          )}
          {onEdit && (
            <button
              type="button"
              onClick={() => onEdit(personMonth)}
              className="text-sm text-gray-600 hover:text-gray-800"
            >
              {t('common.edit')}
            </button>
          )}
          {onDelete && (
            <button
              type="button"
              onClick={handleDeleteClick}
              className="text-sm text-red-600 hover:text-red-800 ml-auto"
            >
              {t('common.delete')}
            </button>
          )}
        </div>
      </div>

      {/* Delete Confirmation Modal */}
      {showDeleteConfirm && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg shadow-xl p-6 max-w-md mx-4">
            <h3 className="text-lg font-semibold text-gray-900 mb-2">
              {t('personMonths.deleteConfirmTitle')}
            </h3>
            <p className="text-gray-600 mb-4">
              {t('personMonths.deleteConfirmMessage', {
                month: monthLabel,
                year: personMonth.year,
                unit: personMonth.unitDesignation,
              })}
            </p>
            <div className="flex justify-end gap-3">
              <button
                type="button"
                onClick={handleCancelDelete}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                onClick={handleConfirmDelete}
                className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700"
              >
                {t('common.delete')}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
