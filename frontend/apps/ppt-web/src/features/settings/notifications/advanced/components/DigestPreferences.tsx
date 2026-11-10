/**
 * DigestPreferences Component (Epic 40, Story 40.3)
 *
 * Configuration UI for notification digests - bundled notification summaries.
 */

import type { DayOfWeek, DigestConfig } from '@ppt/api-client';
import {
  ALL_DAYS,
  DAY_FULL_LABELS,
  DIGEST_OPTIONS,
  FREQUENCY_DESCRIPTIONS,
  FREQUENCY_LABELS,
} from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface DigestPreferencesProps {
  config: DigestConfig;
  loading?: boolean;
  onUpdate: (updates: Partial<DigestConfig>) => void;
}

export function DigestPreferences({ config, loading, onUpdate }: DigestPreferencesProps) {
  const { t } = useTranslation();
  const dayLabel = (day: DayOfWeek) =>
    t(`settings.notifications.advanced.days.${day}`, { defaultValue: DAY_FULL_LABELS[day] });

  const getNextDeliveryText = (): string => {
    if (!config.enabled || config.frequency === 'disabled') {
      return t('settings.notifications.advanced.digest.nextDisabled');
    }

    const time = config.deliveryTime || '09:00';

    // Note: 'disabled' is already handled by the guard above, so TypeScript
    // knows config.frequency can only be 'hourly' | 'daily' | 'weekly' here
    switch (config.frequency) {
      case 'hourly':
        return t('settings.notifications.advanced.digest.nextHourly');
      case 'daily':
        return t('settings.notifications.advanced.digest.nextDaily', { time });
      case 'weekly':
        return t('settings.notifications.advanced.digest.nextWeekly', {
          day: dayLabel(config.deliveryDay || 'monday'),
          time,
        });
    }
  };

  return (
    <div className="bg-white border border-gray-200 rounded-lg p-6">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-lg font-semibold text-gray-900">
            {t('settings.notifications.advanced.digest.title')}
          </h3>
          <p className="text-sm text-gray-500 mt-0.5">
            {t('settings.notifications.advanced.digest.subtitle')}
          </p>
        </div>

        {/* Enable toggle */}
        <button
          type="button"
          role="switch"
          aria-checked={config.enabled}
          aria-label={t('aria.enableDigest')}
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
        <div className="space-y-5 pt-4 border-t border-gray-100">
          {/* Frequency selector */}
          <fieldset className="border-0 p-0 m-0">
            <legend className="block text-sm font-medium text-gray-700 mb-2">
              {t('settings.notifications.advanced.digest.frequency')}
            </legend>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
              {DIGEST_OPTIONS.map((freq) => (
                <button
                  key={freq}
                  type="button"
                  aria-pressed={config.frequency === freq}
                  onClick={() => onUpdate({ frequency: freq })}
                  disabled={loading}
                  className={`
                    px-3 py-2 rounded-lg text-sm font-medium text-center transition-colors
                    ${
                      config.frequency === freq
                        ? 'bg-blue-600 text-white'
                        : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                    }
                    ${loading ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
                  `}
                >
                  {t(`settings.notifications.advanced.digest.frequencies.${freq}.label`, {
                    defaultValue: FREQUENCY_LABELS[freq],
                  })}
                </button>
              ))}
            </div>
            <p className="text-xs text-gray-500 mt-2">
              {t(
                `settings.notifications.advanced.digest.frequencies.${config.frequency}.description`,
                {
                  defaultValue: FREQUENCY_DESCRIPTIONS[config.frequency],
                }
              )}
            </p>
          </fieldset>

          {/* Delivery time (for daily/weekly) */}
          {(config.frequency === 'daily' || config.frequency === 'weekly') && (
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label
                  htmlFor="delivery-time"
                  className="block text-sm font-medium text-gray-700 mb-1"
                >
                  {t('settings.notifications.advanced.digest.deliveryTime')}
                </label>
                <input
                  type="time"
                  id="delivery-time"
                  value={config.deliveryTime}
                  onChange={(e) => onUpdate({ deliveryTime: e.target.value })}
                  disabled={loading}
                  className="block w-full rounded-md border-0 py-2 px-3 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 focus:ring-2 focus:ring-inset focus:ring-blue-600 sm:text-sm disabled:opacity-50"
                />
              </div>

              {config.frequency === 'weekly' && (
                <div>
                  <label
                    htmlFor="delivery-day"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    {t('settings.notifications.advanced.digest.deliveryDay')}
                  </label>
                  <select
                    id="delivery-day"
                    value={config.deliveryDay || 'monday'}
                    onChange={(e) => onUpdate({ deliveryDay: e.target.value as DayOfWeek })}
                    disabled={loading}
                    className="block w-full rounded-md border-0 py-2 pl-3 pr-10 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 focus:ring-2 focus:ring-inset focus:ring-blue-600 sm:text-sm disabled:opacity-50"
                  >
                    {ALL_DAYS.map((day) => (
                      <option key={day} value={day}>
                        {dayLabel(day)}
                      </option>
                    ))}
                  </select>
                </div>
              )}
            </div>
          )}

          {/* Preview */}
          <div className="bg-blue-50 rounded-lg p-3">
            <p className="text-xs text-blue-600 mb-1 font-medium">
              {t('settings.notifications.advanced.digest.nextDelivery')}
            </p>
            <p className="text-sm text-blue-800">{getNextDeliveryText()}</p>
          </div>

          <p className="text-xs text-gray-500">
            {t('settings.notifications.advanced.digest.allCategoriesNote')}
          </p>
        </div>
      )}
    </div>
  );
}
