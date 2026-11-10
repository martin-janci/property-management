/**
 * DateRangeFilter component for filtering financial data by date range.
 */

import { useTranslation } from 'react-i18next';

interface DateRangeFilterProps {
  startDate?: string;
  endDate?: string;
  onStartDateChange: (date?: string) => void;
  onEndDateChange: (date?: string) => void;
  presets?: boolean;
}

type PresetKey = 'today' | 'week' | 'month' | 'quarter' | 'year';

/** Presets in display order (labels: `financial.dateRange.<key>`). */
const PRESETS: PresetKey[] = ['today', 'week', 'month', 'quarter', 'year'];

function getPresetDates(preset: PresetKey): { start: string; end: string } {
  const today = new Date();
  const year = today.getFullYear();
  const month = today.getMonth();

  switch (preset) {
    case 'today':
      return {
        start: today.toISOString().split('T')[0],
        end: today.toISOString().split('T')[0],
      };
    case 'week': {
      const weekStart = new Date(today);
      weekStart.setDate(today.getDate() - today.getDay());
      return {
        start: weekStart.toISOString().split('T')[0],
        end: today.toISOString().split('T')[0],
      };
    }
    case 'month':
      return {
        start: new Date(year, month, 1).toISOString().split('T')[0],
        end: today.toISOString().split('T')[0],
      };
    case 'quarter': {
      const quarterMonth = Math.floor(month / 3) * 3;
      return {
        start: new Date(year, quarterMonth, 1).toISOString().split('T')[0],
        end: today.toISOString().split('T')[0],
      };
    }
    case 'year':
      return {
        start: new Date(year, 0, 1).toISOString().split('T')[0],
        end: today.toISOString().split('T')[0],
      };
    default:
      return {
        start: today.toISOString().split('T')[0],
        end: today.toISOString().split('T')[0],
      };
  }
}

export function DateRangeFilter({
  startDate,
  endDate,
  onStartDateChange,
  onEndDateChange,
  presets = true,
}: DateRangeFilterProps) {
  const { t } = useTranslation();
  const handlePresetClick = (preset: PresetKey) => {
    const dates = getPresetDates(preset);
    onStartDateChange(dates.start);
    onEndDateChange(dates.end);
  };

  return (
    <div className="flex flex-col gap-3">
      {presets && (
        <div className="flex gap-2">
          {PRESETS.map((preset) => (
            <button
              key={preset}
              type="button"
              onClick={() => handlePresetClick(preset)}
              className="px-3 py-1 text-xs font-medium text-gray-600 bg-gray-100 rounded-full hover:bg-gray-200 transition-colors"
            >
              {t(`financial.dateRange.${preset}`)}
            </button>
          ))}
          <button
            type="button"
            onClick={() => {
              onStartDateChange(undefined);
              onEndDateChange(undefined);
            }}
            className="px-3 py-1 text-xs font-medium text-gray-500 hover:text-gray-700"
          >
            {t('financial.dateRange.clear')}
          </button>
        </div>
      )}

      <div className="flex items-center gap-2">
        <label htmlFor="start-date" className="text-sm text-gray-600">
          {t('financial.dateRange.from')}
        </label>
        <input
          id="start-date"
          type="date"
          value={startDate || ''}
          onChange={(e) => onStartDateChange(e.target.value || undefined)}
          className="px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
        />
        <label htmlFor="end-date" className="text-sm text-gray-600 ml-2">
          {t('financial.dateRange.to')}
        </label>
        <input
          id="end-date"
          type="date"
          value={endDate || ''}
          onChange={(e) => onEndDateChange(e.target.value || undefined)}
          className="px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
        />
      </div>
    </div>
  );
}
