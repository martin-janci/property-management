/**
 * AdvancedNotificationSettingsPage Component (Epic 40)
 *
 * Settings page for advanced notification preferences:
 * - Story 40.1: Granular category preferences
 * - Story 40.2: Quiet hours configuration
 * - Story 40.3: Digest preferences
 * - Story 40.4: Smart notification grouping
 *
 * Auth is injected via the global token provider — no props required.
 *
 * @module features/settings/notifications/advanced
 */

import type { NotificationCategory, NotificationChannel } from '@ppt/api-client';
import {
  useCategoryPreferences,
  useDigestPreferences,
  useGroupingPreferences,
  useQuietHours,
  useUpdateCategoryPreference,
  useUpdateDigestPreferences,
  useUpdateGroupingPreferences,
  useUpdateQuietHours,
} from '@ppt/api-client';
import type * as React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import {
  CategoryPreferenceCard,
  DigestPreferences,
  GroupingSettings,
  QuietHoursConfig,
} from './components';

type ActiveTab = 'categories' | 'schedule' | 'grouping';

export function AdvancedNotificationSettingsPage() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<ActiveTab>('categories');
  const [updateError, setUpdateError] = useState<string | null>(null);

  // Data hooks
  const categoryQuery = useCategoryPreferences();
  const quietHoursQuery = useQuietHours();
  const digestQuery = useDigestPreferences();
  const groupingQuery = useGroupingPreferences();

  // Mutation hooks
  const updateCategory = useUpdateCategoryPreference();
  const updateQuietHours = useUpdateQuietHours();
  const updateDigest = useUpdateDigestPreferences();
  const updateGrouping = useUpdateGroupingPreferences();

  const isLoading =
    categoryQuery.isLoading ||
    quietHoursQuery.isLoading ||
    digestQuery.isLoading ||
    groupingQuery.isLoading;

  const hasError =
    categoryQuery.error || quietHoursQuery.error || digestQuery.error || groupingQuery.error;

  const handleCategoryChannelToggle = async (
    category: NotificationCategory,
    channel: NotificationChannel,
    enabled: boolean
  ) => {
    setUpdateError(null);
    try {
      await updateCategory.mutateAsync({
        category,
        request: { channels: { [channel]: enabled } },
      });
    } catch (error) {
      console.error('Failed to update category notification preference', {
        category,
        channel,
        enabled,
        error,
      });
      setUpdateError('Failed to update category preference. Please try again.');
    }
  };

  if (isLoading) {
    return (
      <div className="max-w-3xl mx-auto p-6">
        <div className="animate-pulse">
          <div className="h-8 bg-gray-200 rounded w-1/2 mb-2" />
          <div className="h-4 bg-gray-100 rounded w-2/3 mb-6" />
          <div className="h-12 bg-gray-100 rounded mb-6" />
          <div className="space-y-4">
            <div className="h-24 bg-gray-100 rounded" />
            <div className="h-24 bg-gray-100 rounded" />
            <div className="h-24 bg-gray-100 rounded" />
          </div>
        </div>
      </div>
    );
  }

  if (hasError) {
    return (
      <div className="max-w-3xl mx-auto p-6">
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <div className="flex">
            <svg
              className="h-5 w-5 text-red-400 flex-shrink-0"
              viewBox="0 0 20 20"
              fill="currentColor"
              aria-hidden="true"
            >
              <path
                fillRule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.28 7.22a.75.75 0 00-1.06 1.06L8.94 10l-1.72 1.72a.75.75 0 101.06 1.06L10 11.06l1.72 1.72a.75.75 0 101.06-1.06L11.06 10l1.72-1.72a.75.75 0 00-1.06-1.06L10 8.94 8.28 7.22z"
                clipRule="evenodd"
              />
            </svg>
            <p className="ml-3 text-sm text-red-700">
              Failed to load notification preferences. Please try again later.
            </p>
          </div>
        </div>
      </div>
    );
  }

  const tabs: { id: ActiveTab; label: string; icon: React.JSX.Element }[] = [
    {
      id: 'categories',
      label: 'Categories',
      icon: (
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
            d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3z"
          />
          <path strokeLinecap="round" strokeLinejoin="round" d="M6 6h.008v.008H6V6z" />
        </svg>
      ),
    },
    {
      id: 'schedule',
      label: 'Schedule',
      icon: (
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
            d="M12 6v6h4.5m4.5 0a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
      ),
    },
    {
      id: 'grouping',
      label: 'Grouping',
      icon: (
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
            d="M6 6.878V6a2.25 2.25 0 012.25-2.25h7.5A2.25 2.25 0 0118 6v.878m-12 0c.235-.083.487-.128.75-.128h10.5c.263 0 .515.045.75.128m-12 0A2.25 2.25 0 004.5 9v.878m13.5-3A2.25 2.25 0 0119.5 9v.878m0 0a2.246 2.246 0 00-.75-.128H5.25c-.263 0-.515.045-.75.128m15 0A2.25 2.25 0 0121 12v6a2.25 2.25 0 01-2.25 2.25H5.25A2.25 2.25 0 013 18v-6c0-.98.626-1.813 1.5-2.122"
          />
        </svg>
      ),
    },
  ];

  return (
    <div className="max-w-3xl mx-auto p-6">
      <h1 className="text-2xl font-bold text-gray-900 mb-2">Advanced Notification Settings</h1>
      <p className="text-gray-600 mb-2">Fine-tune how and when you receive notifications.</p>
      {/* Cross-link to the per-event trigger UI so the two preference surfaces
          (category-level here, per-event there) are discoverable from one place. */}
      <p className="mb-6">
        <Link
          to="/notifications/triggers"
          className="text-sm font-medium text-blue-600 hover:text-blue-500"
        >
          Per-event triggers →
        </Link>
      </p>

      {/* Update error alert */}
      {updateError && (
        <div className="mb-6 bg-red-50 border border-red-200 rounded-lg p-4">
          <div className="flex">
            <svg
              className="h-5 w-5 text-red-400 flex-shrink-0"
              viewBox="0 0 20 20"
              fill="currentColor"
              aria-hidden="true"
            >
              <path
                fillRule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.28 7.22a.75.75 0 00-1.06 1.06L8.94 10l-1.72 1.72a.75.75 0 101.06 1.06L10 11.06l1.72 1.72a.75.75 0 101.06-1.06L11.06 10l1.72-1.72a.75.75 0 00-1.06-1.06L10 8.94 8.28 7.22z"
                clipRule="evenodd"
              />
            </svg>
            <p className="ml-3 flex-1 text-sm text-red-700">{updateError}</p>
            <button
              type="button"
              onClick={() => setUpdateError(null)}
              className="ml-3 text-red-400 hover:text-red-500"
            >
              <span className="sr-only">Dismiss</span>
              <svg className="h-5 w-5" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
                <path d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z" />
              </svg>
            </button>
          </div>
        </div>
      )}

      {/* Tab navigation */}
      <div className="border-b border-gray-200 mb-6">
        <nav className="-mb-px flex space-x-8" aria-label={t('aria.tabs')}>
          {tabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => setActiveTab(tab.id)}
              className={`
                group inline-flex items-center gap-2 py-4 px-1 border-b-2 font-medium text-sm
                ${
                  activeTab === tab.id
                    ? 'border-blue-500 text-blue-600'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }
              `}
            >
              <span
                className={
                  activeTab === tab.id ? 'text-blue-500' : 'text-gray-400 group-hover:text-gray-500'
                }
              >
                {tab.icon}
              </span>
              {tab.label}
            </button>
          ))}
        </nav>
      </div>

      {/* Tab content */}
      {activeTab === 'categories' && categoryQuery.data && (
        <div className="space-y-4">
          <p className="text-sm text-gray-500 mb-4">
            Choose which types of notifications you want to receive and how.
          </p>
          {categoryQuery.data.categories.map((pref) => (
            <CategoryPreferenceCard
              key={pref.category}
              preference={pref}
              loading={updateCategory.isPending}
              onToggleChannel={(channel, enabled) =>
                handleCategoryChannelToggle(pref.category, channel, enabled)
              }
            />
          ))}
        </div>
      )}

      {activeTab === 'schedule' && (
        <div className="space-y-6">
          {/* onUpdate is async - error handling is done here in the parent component.
              Errors are caught and displayed via setUpdateError. */}
          {quietHoursQuery.data && (
            <QuietHoursConfig
              config={quietHoursQuery.data.quietHours}
              loading={updateQuietHours.isPending}
              onUpdate={async (updates) => {
                setUpdateError(null);
                try {
                  await updateQuietHours.mutateAsync(updates);
                } catch (error) {
                  console.error('Failed to update quiet hours', { updates, error });
                  setUpdateError('Failed to update quiet hours. Please try again.');
                }
              }}
            />
          )}

          {digestQuery.data && (
            <DigestPreferences
              config={digestQuery.data.digest}
              loading={updateDigest.isPending}
              onUpdate={async (updates) => {
                setUpdateError(null);
                try {
                  await updateDigest.mutateAsync(updates);
                } catch (error) {
                  console.error('Failed to update digest preferences', { updates, error });
                  setUpdateError('Failed to update digest preferences. Please try again.');
                }
              }}
            />
          )}
        </div>
      )}

      {activeTab === 'grouping' && groupingQuery.data && (
        <GroupingSettings
          config={groupingQuery.data.grouping}
          loading={updateGrouping.isPending}
          onUpdate={async (updates) => {
            setUpdateError(null);
            try {
              await updateGrouping.mutateAsync(updates);
            } catch (error) {
              console.error('Failed to update grouping settings', { updates, error });
              setUpdateError('Failed to update grouping settings. Please try again.');
            }
          }}
        />
      )}
    </div>
  );
}
