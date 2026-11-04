/**
 * ChannelToggle Component (Epic 8A, Story 8A.1)
 *
 * A toggle switch for a notification channel with label and description.
 */

import type { NotificationChannel } from '@ppt/api-client';
import { CHANNEL_DESCRIPTIONS, CHANNEL_LABELS } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface ChannelToggleProps {
  channel: NotificationChannel;
  enabled: boolean;
  loading?: boolean;
  onToggle: (enabled: boolean) => void;
}

export function ChannelToggle({ channel, enabled, loading = false, onToggle }: ChannelToggleProps) {
  const { t } = useTranslation();
  const label = t(`settings.notifications.channels.${channel}.label`, {
    defaultValue: CHANNEL_LABELS[channel],
  });
  const handleChange = () => {
    if (!loading) {
      onToggle(!enabled);
    }
  };

  return (
    <div className="flex items-center justify-between py-4 border-b border-gray-100 last:border-b-0">
      <div className="flex-1 min-w-0">
        <label htmlFor={`toggle-${channel}`} className="block text-sm font-medium text-gray-900">
          {label}
        </label>
        <p className="mt-1 text-sm text-gray-500">
          {t(`settings.notifications.channels.${channel}.description`, {
            defaultValue: CHANNEL_DESCRIPTIONS[channel],
          })}
        </p>
      </div>
      <div className="ml-4 flex-shrink-0">
        <button
          type="button"
          id={`toggle-${channel}`}
          role="switch"
          aria-checked={enabled}
          disabled={loading}
          onClick={handleChange}
          className={`
            relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent
            transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2
            ${enabled ? 'bg-blue-600' : 'bg-gray-200'}
            ${loading ? 'opacity-50 cursor-not-allowed' : ''}
          `}
        >
          <span className="sr-only">
            {t('settings.notifications.toggleChannel', { channel: label })}
          </span>
          <span
            aria-hidden="true"
            className={`
              pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0
              transition duration-200 ease-in-out
              ${enabled ? 'translate-x-5' : 'translate-x-0'}
            `}
          />
        </button>
      </div>
    </div>
  );
}
