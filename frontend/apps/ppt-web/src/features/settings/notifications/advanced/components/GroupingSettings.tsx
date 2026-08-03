/**
 * GroupingSettings Component (Epic 40, Story 40.4)
 *
 * Configuration UI for smart notification grouping in the notification center.
 */

import type { GroupingConfig } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface GroupingSettingsProps {
  config: GroupingConfig;
  loading?: boolean;
  onUpdate: (updates: Partial<GroupingConfig>) => void;
}

export function GroupingSettings({ config, loading, onUpdate }: GroupingSettingsProps) {
  const { t } = useTranslation();
  return (
    <div className="bg-white border border-gray-200 rounded-lg p-6">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-lg font-semibold text-gray-900">Smart Grouping</h3>
          <p className="text-sm text-gray-500 mt-0.5">
            Group similar notifications together to reduce clutter
          </p>
        </div>

        {/* Enable toggle */}
        <button
          type="button"
          role="switch"
          aria-checked={config.enabled}
          aria-label={t('aria.enableGrouping')}
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
          {/* Grouping options */}
          <div className="space-y-3">
            {/* Group by category */}
            <div className="flex items-center justify-between py-2">
              <div>
                <p className="text-sm font-medium text-gray-700">Group by Category</p>
                <p className="text-xs text-gray-500">
                  Combine notifications of the same type (e.g., "5 new fault updates")
                </p>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={config.groupByCategory}
                aria-label={t('aria.groupByCategory')}
                onClick={() => onUpdate({ groupByCategory: !config.groupByCategory })}
                disabled={loading}
                className={`
                  relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full
                  border-2 border-transparent transition-colors duration-200 ease-in-out
                  focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2
                  ${config.groupByCategory ? 'bg-blue-600' : 'bg-gray-200'}
                  ${loading ? 'opacity-50 cursor-not-allowed' : ''}
                `}
              >
                <span
                  className={`
                    pointer-events-none inline-block h-4 w-4 transform rounded-full
                    bg-white shadow ring-0 transition duration-200 ease-in-out
                    ${config.groupByCategory ? 'translate-x-4' : 'translate-x-0'}
                  `}
                />
              </button>
            </div>

            {/* Group by source */}
            <div className="flex items-center justify-between py-2">
              <div>
                <p className="text-sm font-medium text-gray-700">Group by Source</p>
                <p className="text-xs text-gray-500">
                  Combine notifications from the same building or issue
                </p>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={config.groupBySource}
                aria-label={t('aria.groupBySource')}
                onClick={() => onUpdate({ groupBySource: !config.groupBySource })}
                disabled={loading}
                className={`
                  relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full
                  border-2 border-transparent transition-colors duration-200 ease-in-out
                  focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2
                  ${config.groupBySource ? 'bg-blue-600' : 'bg-gray-200'}
                  ${loading ? 'opacity-50 cursor-not-allowed' : ''}
                `}
              >
                <span
                  className={`
                    pointer-events-none inline-block h-4 w-4 transform rounded-full
                    bg-white shadow ring-0 transition duration-200 ease-in-out
                    ${config.groupBySource ? 'translate-x-4' : 'translate-x-0'}
                  `}
                />
              </button>
            </div>
          </div>

          {/* Group size settings */}
          <div className="grid grid-cols-2 gap-4 pt-2">
            <div>
              <label
                htmlFor="max-group-size"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                Max Group Size
              </label>
              <select
                id="max-group-size"
                value={config.maxGroupSize}
                onChange={(e) => onUpdate({ maxGroupSize: Number.parseInt(e.target.value, 10) })}
                disabled={loading}
                className="block w-full rounded-md border-0 py-2 pl-3 pr-10 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 focus:ring-2 focus:ring-inset focus:ring-blue-600 sm:text-sm disabled:opacity-50"
              >
                <option value={3}>3 items</option>
                <option value={5}>5 items</option>
                <option value={10}>10 items</option>
                <option value={20}>20 items</option>
              </select>
              <p className="text-xs text-gray-500 mt-1">Show "and X more" after this many</p>
            </div>

            <div>
              <label htmlFor="auto-expand" className="block text-sm font-medium text-gray-700 mb-1">
                Auto-Expand Threshold
              </label>
              <select
                id="auto-expand"
                value={config.autoExpandThreshold}
                onChange={(e) =>
                  onUpdate({ autoExpandThreshold: Number.parseInt(e.target.value, 10) })
                }
                disabled={loading}
                className="block w-full rounded-md border-0 py-2 pl-3 pr-10 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 focus:ring-2 focus:ring-inset focus:ring-blue-600 sm:text-sm disabled:opacity-50"
              >
                <option value={0}>Never auto-expand</option>
                {config.maxGroupSize >= 2 && <option value={2}>2 or fewer</option>}
                {config.maxGroupSize >= 3 && <option value={3}>3 or fewer</option>}
                {config.maxGroupSize >= 5 && <option value={5}>5 or fewer</option>}
              </select>
              <p className="text-xs text-gray-500 mt-1">Expand groups with fewer items</p>
            </div>
          </div>

          {/* Preview */}
          <div className="bg-gray-50 rounded-lg p-4 mt-4">
            <p className="text-xs text-gray-500 mb-3 font-medium">Preview</p>
            <div className="space-y-2">
              <div className="bg-white rounded-md border border-gray-200 p-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <div className="w-2 h-2 bg-blue-500 rounded-full" />
                    <span className="text-sm font-medium text-gray-900">
                      {config.maxGroupSize} new comments
                    </span>
                  </div>
                  <span className="sr-only">Expand notification group</span>
                  <svg
                    className="h-4 w-4 text-gray-400"
                    fill="none"
                    viewBox="0 0 24 24"
                    strokeWidth={2}
                    stroke="currentColor"
                    aria-hidden="true"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      d="M19.5 8.25l-7.5 7.5-7.5-7.5"
                    />
                  </svg>
                </div>
              </div>
              <p className="text-xs text-gray-400 text-center">
                Click to expand and see individual items
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
