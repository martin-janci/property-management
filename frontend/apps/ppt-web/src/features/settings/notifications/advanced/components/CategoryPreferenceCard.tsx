/**
 * CategoryPreferenceCard Component (Epic 40, Story 40.1)
 *
 * Displays a notification category with toggles for each channel.
 */

import type { CategoryPreference, NotificationChannel } from '@ppt/api-client';
import { ALL_CHANNELS, CATEGORY_DESCRIPTIONS, CATEGORY_LABELS } from '@ppt/api-client';

interface CategoryPreferenceCardProps {
  preference: CategoryPreference;
  loading?: boolean;
  onToggleChannel: (channel: NotificationChannel, enabled: boolean) => void;
}

const CHANNEL_ICONS: Record<NotificationChannel, React.JSX.Element> = {
  push: (
    <svg
      className="h-5 w-5"
      fill="none"
      viewBox="0 0 24 24"
      strokeWidth={1.5}
      stroke="currentColor"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M10.5 1.5H8.25A2.25 2.25 0 006 3.75v16.5a2.25 2.25 0 002.25 2.25h7.5A2.25 2.25 0 0018 20.25V3.75a2.25 2.25 0 00-2.25-2.25H13.5m-3 0V3h3V1.5m-3 0h3m-3 18.75h3"
      />
    </svg>
  ),
  email: (
    <svg
      className="h-5 w-5"
      fill="none"
      viewBox="0 0 24 24"
      strokeWidth={1.5}
      stroke="currentColor"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M21.75 6.75v10.5a2.25 2.25 0 01-2.25 2.25h-15a2.25 2.25 0 01-2.25-2.25V6.75m19.5 0A2.25 2.25 0 0019.5 4.5h-15a2.25 2.25 0 00-2.25 2.25m19.5 0v.243a2.25 2.25 0 01-1.07 1.916l-7.5 4.615a2.25 2.25 0 01-2.36 0L3.32 8.91a2.25 2.25 0 01-1.07-1.916V6.75"
      />
    </svg>
  ),
  in_app: (
    <svg
      className="h-5 w-5"
      fill="none"
      viewBox="0 0 24 24"
      strokeWidth={1.5}
      stroke="currentColor"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M14.857 17.082a23.848 23.848 0 005.454-1.31A8.967 8.967 0 0118 9.75v-.7V9A6 6 0 006 9v.75a8.967 8.967 0 01-2.312 6.022c1.733.64 3.56 1.085 5.455 1.31m5.714 0a24.255 24.255 0 01-5.714 0m5.714 0a3 3 0 11-5.714 0"
      />
    </svg>
  ),
};

const CHANNEL_LABELS: Record<NotificationChannel, string> = {
  push: 'Push',
  email: 'Email',
  in_app: 'In-App',
};

export function CategoryPreferenceCard({
  preference,
  loading,
  onToggleChannel,
}: CategoryPreferenceCardProps) {
  const allEnabled = ALL_CHANNELS.every((ch) => preference.channels[ch]);
  const allDisabled = ALL_CHANNELS.every((ch) => !preference.channels[ch]);

  return (
    <div className="bg-white border border-gray-200 rounded-lg p-4 hover:shadow-sm transition-shadow">
      <div className="flex items-start justify-between mb-3">
        <div>
          <h3 className="text-sm font-semibold text-gray-900">
            {CATEGORY_LABELS[preference.category]}
          </h3>
          <p className="text-xs text-gray-500 mt-0.5">
            {CATEGORY_DESCRIPTIONS[preference.category]}
          </p>
        </div>
        {allDisabled && (
          <span className="inline-flex items-center rounded-full bg-gray-100 px-2 py-1 text-xs font-medium text-gray-600">
            Off
          </span>
        )}
        {allEnabled && (
          <span className="inline-flex items-center rounded-full bg-green-100 px-2 py-1 text-xs font-medium text-green-700">
            All On
          </span>
        )}
      </div>

      <div className="flex items-center gap-2">
        {ALL_CHANNELS.map((channel) => {
          const enabled = preference.channels[channel];
          return (
            <button
              key={channel}
              type="button"
              aria-pressed={enabled}
              aria-label={`${CHANNEL_LABELS[channel]} notifications for ${CATEGORY_LABELS[preference.category]}`}
              onClick={() => onToggleChannel(channel, !enabled)}
              disabled={loading}
              className={`
                flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium
                transition-colors duration-150
                ${
                  enabled
                    ? 'bg-blue-100 text-blue-700 hover:bg-blue-200'
                    : 'bg-gray-100 text-gray-500 hover:bg-gray-200'
                }
                ${loading ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
              `}
              title={`${enabled ? 'Disable' : 'Enable'} ${CHANNEL_LABELS[channel]} for ${CATEGORY_LABELS[preference.category]}`}
            >
              <span className={enabled ? 'text-blue-600' : 'text-gray-400'}>
                {CHANNEL_ICONS[channel]}
              </span>
              {CHANNEL_LABELS[channel]}
            </button>
          );
        })}
      </div>
    </div>
  );
}
