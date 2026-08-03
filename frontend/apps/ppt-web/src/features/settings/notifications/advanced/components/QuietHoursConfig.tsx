/**
 * QuietHoursConfig Component (Epic 40, Story 40.2)
 *
 * Configuration UI for quiet hours - when notifications are silenced.
 */

import type { DayOfWeek, QuietHoursConfig as QuietHoursConfigType } from '@ppt/api-client';
import { ALL_DAYS, DAY_LABELS } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface QuietHoursConfigProps {
  config: QuietHoursConfigType;
  loading?: boolean;
  onUpdate: (updates: Partial<QuietHoursConfigType>) => void;
}

export function QuietHoursConfig({ config, loading, onUpdate }: QuietHoursConfigProps) {
  const { t } = useTranslation();
  const handleDayToggle = (day: DayOfWeek) => {
    const newDays = config.daysOfWeek.includes(day)
      ? config.daysOfWeek.filter((d) => d !== day)
      : [...config.daysOfWeek, day];
    onUpdate({ daysOfWeek: newDays });
  };

  return (
    <div className="bg-white border border-gray-200 rounded-lg p-6">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-lg font-semibold text-gray-900">Quiet Hours</h3>
          <p className="text-sm text-gray-500 mt-0.5">
            Silence non-emergency notifications during specific hours
          </p>
        </div>

        {/* Enable toggle */}
        <button
          type="button"
          role="switch"
          aria-checked={config.enabled}
          aria-label={t('aria.enableQuietHours')}
          onClick={() => onUpdate({ enabled: !config.enabled })}
          disabled={loading}
          className={`
            relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full
            border-2 border-transparent transition-colors duration-200 ease-in-out
            focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2
            ${config.enabled ? 'bg-blue-600' : 'bg-gray-200'}
            ${loading ? 'opacity-50 cursor-not-allowed' : ''}
          `}
        >
          <span
            className={`
              pointer-events-none inline-block h-5 w-5 transform rounded-full
              bg-white shadow ring-0 transition duration-200 ease-in-out
              ${config.enabled ? 'translate-x-5' : 'translate-x-0'}
            `}
          />
        </button>
      </div>

      {config.enabled && (
        <div className="space-y-4 pt-4 border-t border-gray-100">
          {/* Time range */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label htmlFor="start-time" className="block text-sm font-medium text-gray-700 mb-1">
                Start Time
              </label>
              <input
                type="time"
                id="start-time"
                value={config.startTime}
                onChange={(e) => onUpdate({ startTime: e.target.value })}
                disabled={loading}
                aria-describedby="time-format-hint"
                className="block w-full rounded-md border-0 py-2 px-3 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 focus:ring-2 focus:ring-inset focus:ring-blue-600 sm:text-sm disabled:opacity-50"
              />
            </div>
            <div>
              <label htmlFor="end-time" className="block text-sm font-medium text-gray-700 mb-1">
                End Time
              </label>
              <input
                type="time"
                id="end-time"
                value={config.endTime}
                onChange={(e) => onUpdate({ endTime: e.target.value })}
                disabled={loading}
                aria-describedby="time-format-hint"
                className="block w-full rounded-md border-0 py-2 px-3 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 focus:ring-2 focus:ring-inset focus:ring-blue-600 sm:text-sm disabled:opacity-50"
              />
            </div>
          </div>
          <p id="time-format-hint" className="text-xs text-gray-500 mt-1">
            Times are in 24-hour format (HH:MM) and use the timezone selected below.
          </p>

          {/* Visual schedule preview */}
          <div className="bg-gray-50 rounded-lg p-3">
            <p className="text-xs text-gray-500 mb-2">Schedule Preview</p>
            <div className="flex items-center gap-1">
              <span className="text-sm font-medium text-gray-700">🌙</span>
              <span className="text-sm text-gray-600">
                Quiet from <strong>{config.startTime}</strong> to <strong>{config.endTime}</strong>
              </span>
            </div>
          </div>

          {/* Days of week */}
          <fieldset className="border-0 p-0 m-0">
            <legend className="block text-sm font-medium text-gray-700 mb-2">Active Days</legend>
            <div className="flex flex-wrap gap-2">
              {ALL_DAYS.map((day) => {
                const isSelected = config.daysOfWeek.includes(day);
                return (
                  <button
                    key={day}
                    type="button"
                    aria-pressed={isSelected}
                    aria-label={`${DAY_LABELS[day]} - ${isSelected ? 'active' : 'inactive'}`}
                    onClick={() => handleDayToggle(day)}
                    disabled={loading}
                    className={`
                      px-3 py-1.5 rounded-md text-sm font-medium transition-colors
                      ${
                        isSelected
                          ? 'bg-blue-100 text-blue-700 hover:bg-blue-200'
                          : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                      }
                      ${loading ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
                    `}
                  >
                    {DAY_LABELS[day]}
                  </button>
                );
              })}
            </div>
          </fieldset>

          {/* Timezone */}
          <div>
            <label htmlFor="timezone" className="block text-sm font-medium text-gray-700 mb-1">
              Timezone
            </label>
            <select
              id="timezone"
              value={config.timezone}
              onChange={(e) => onUpdate({ timezone: e.target.value })}
              disabled={loading}
              className="block w-full rounded-md border-0 py-2 pl-3 pr-10 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 focus:ring-2 focus:ring-inset focus:ring-blue-600 sm:text-sm disabled:opacity-50"
            >
              <option value="Europe/Bratislava">Europe/Bratislava (CET)</option>
              <option value="Europe/Prague">Europe/Prague (CET)</option>
              <option value="Europe/Vienna">Europe/Vienna (CET)</option>
              <option value="Europe/Berlin">Europe/Berlin (CET)</option>
              <option value="Europe/London">Europe/London (GMT)</option>
              <option value="UTC">UTC</option>
            </select>
          </div>

          {/* Emergency override */}
          <div className="flex items-center justify-between pt-2">
            <div>
              <p className="text-sm font-medium text-gray-700">Allow Emergency Notifications</p>
              <p className="text-xs text-gray-500">
                Critical alerts will still come through during quiet hours
              </p>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={config.allowEmergency}
              aria-label={t('aria.allowEmergencyNotifications')}
              onClick={() => onUpdate({ allowEmergency: !config.allowEmergency })}
              disabled={loading}
              className={`
                relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full
                border-2 border-transparent transition-colors duration-200 ease-in-out
                focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2
                ${config.allowEmergency ? 'bg-blue-600' : 'bg-gray-200'}
                ${loading ? 'opacity-50 cursor-not-allowed' : ''}
              `}
            >
              <span
                className={`
                  pointer-events-none inline-block h-4 w-4 transform rounded-full
                  bg-white shadow ring-0 transition duration-200 ease-in-out
                  ${config.allowEmergency ? 'translate-x-4' : 'translate-x-0'}
                `}
              />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
