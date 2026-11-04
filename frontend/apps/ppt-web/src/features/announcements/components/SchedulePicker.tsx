import { useTranslation } from 'react-i18next';

interface SchedulePickerProps {
  scheduledAt?: string;
  onScheduleChange: (date?: string) => void;
}

export function SchedulePicker({ scheduledAt, onScheduleChange }: SchedulePickerProps) {
  const { t } = useTranslation();
  const minDate = new Date();
  minDate.setMinutes(minDate.getMinutes() + 5); // At least 5 minutes in the future

  const handleDateChange = (value: string) => {
    if (!value) {
      onScheduleChange(undefined);
      return;
    }
    // Convert local datetime to ISO string
    const date = new Date(value);
    onScheduleChange(date.toISOString());
  };

  const formatForInput = (isoString?: string) => {
    if (!isoString) return '';
    const date = new Date(isoString);
    // Format as YYYY-MM-DDTHH:mm for datetime-local input
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    return `${year}-${month}-${day}T${hours}:${minutes}`;
  };

  const formatMinDate = () => {
    const year = minDate.getFullYear();
    const month = String(minDate.getMonth() + 1).padStart(2, '0');
    const day = String(minDate.getDate()).padStart(2, '0');
    const hours = String(minDate.getHours()).padStart(2, '0');
    const minutes = String(minDate.getMinutes()).padStart(2, '0');
    return `${year}-${month}-${day}T${hours}:${minutes}`;
  };

  return (
    <div>
      <label htmlFor="schedule-picker" className="block text-sm font-medium text-gray-700 mb-2">
        {t('announcements.schedule.label')}
      </label>
      <div className="flex items-center gap-3">
        <input
          id="schedule-picker"
          type="datetime-local"
          value={formatForInput(scheduledAt)}
          onChange={(e) => handleDateChange(e.target.value)}
          min={formatMinDate()}
          className="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        {scheduledAt && (
          <button
            type="button"
            onClick={() => onScheduleChange(undefined)}
            className="px-3 py-2 text-sm text-gray-600 hover:text-gray-800"
          >
            {t('announcements.schedule.clear')}
          </button>
        )}
      </div>
      <p className="mt-1 text-xs text-gray-500">{t('announcements.schedule.hint')}</p>
    </div>
  );
}
