/**
 * PendingValidationsPage - manager view of pending meter readings for validation.
 * Meters feature: Self-readings and consumption tracking.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ReadingValidationCard } from '../components/ReadingValidationCard';
import type { MeterReading, MeterType, ValidationResult } from '../types';

interface PendingValidationsPageProps {
  readings: MeterReading[];
  previousReadings?: Record<string, number>;
  isLoading?: boolean;
  isProcessing?: boolean;
  onValidate: (result: ValidationResult) => void;
  onBack: () => void;
}

export function PendingValidationsPage({
  readings,
  previousReadings = {},
  isLoading,
  isProcessing,
  onValidate,
  onBack,
}: PendingValidationsPageProps) {
  const { t } = useTranslation();
  const [typeFilter, setTypeFilter] = useState<MeterType | ''>('');

  const pendingReadings = readings.filter((r) => r.status === 'pending');
  const filteredReadings = typeFilter
    ? pendingReadings.filter((r) => r.meterType === typeFilter)
    : pendingReadings;

  const meterTypes: MeterType[] = [
    'electricity',
    'gas',
    'water',
    'heat',
    'cold_water',
    'hot_water',
  ];

  // Group by building/unit for better organization
  const groupedReadings = filteredReadings.reduce(
    (acc, reading) => {
      const key = reading.meterId;
      if (!acc[key]) {
        acc[key] = [];
      }
      acc[key].push(reading);
      return acc;
    },
    {} as Record<string, MeterReading[]>
  );

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onBack}
          className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 mb-4"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
          {t('common.back')}
        </button>
        <h1 className="text-2xl font-bold text-gray-900">{t('meters.pendingValidations')}</h1>
        <p className="text-gray-600 mt-1">{t('meters.pendingValidationsDescription')}</p>
      </div>

      {/* Stats */}
      <div className="mb-6 grid grid-cols-3 gap-4">
        <div className="bg-yellow-50 rounded-lg p-4 border border-yellow-200">
          <p className="text-sm text-yellow-700">{t('meters.status.pending')}</p>
          <p className="text-2xl font-bold text-yellow-800">{pendingReadings.length}</p>
        </div>
        <div className="bg-green-50 rounded-lg p-4 border border-green-200">
          <p className="text-sm text-green-700">{t('meters.status.validated')}</p>
          <p className="text-2xl font-bold text-green-800">
            {readings.filter((r) => r.status === 'validated').length}
          </p>
        </div>
        <div className="bg-red-50 rounded-lg p-4 border border-red-200">
          <p className="text-sm text-red-700">{t('meters.status.rejected')}</p>
          <p className="text-2xl font-bold text-red-800">
            {readings.filter((r) => r.status === 'rejected').length}
          </p>
        </div>
      </div>

      {/* Filters */}
      <div className="mb-6 flex flex-wrap items-center gap-4">
        <div className="flex items-center gap-2">
          <label htmlFor="typeFilter" className="text-sm font-medium text-gray-700">
            {t('meters.type')}:
          </label>
          <select
            id="typeFilter"
            value={typeFilter}
            onChange={(e) => setTypeFilter(e.target.value as MeterType | '')}
            className="rounded-md border border-gray-300 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('common.all')}</option>
            {meterTypes.map((type) => (
              <option key={type} value={type}>
                {t(`meters.types.${type}`)}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Loading state */}
      {isLoading && (
        <div className="flex justify-center py-12">
          <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600" />
        </div>
      )}

      {/* Empty state */}
      {!isLoading && filteredReadings.length === 0 && (
        <div className="text-center py-12 bg-gray-50 rounded-lg">
          <svg
            className="mx-auto h-12 w-12 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <p className="mt-2 text-gray-500">{t('meters.noPendingValidations')}</p>
        </div>
      )}

      {/* Readings List */}
      {!isLoading && filteredReadings.length > 0 && (
        <div className="space-y-4">
          {Object.entries(groupedReadings).map(([meterId, meterReadings]) => (
            <div key={meterId}>
              {meterReadings.map((reading) => (
                <ReadingValidationCard
                  key={reading.id}
                  reading={reading}
                  previousReading={previousReadings[reading.meterId]}
                  onValidate={onValidate}
                  isProcessing={isProcessing}
                />
              ))}
            </div>
          ))}
        </div>
      )}

      {/* Summary */}
      {!isLoading && pendingReadings.length > 0 && (
        <div className="mt-6 text-sm text-gray-500">
          {t('meters.pendingSummary', {
            shown: filteredReadings.length,
            count: pendingReadings.length,
          })}
        </div>
      )}
    </div>
  );
}
