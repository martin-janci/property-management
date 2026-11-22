/**
 * CriticalNotificationsPage - List view for critical notifications.
 * Epic 8A, Story 8A.2
 */

import type { CriticalNotificationResponse } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';

export interface CriticalNotificationsPageProps {
  notifications: CriticalNotificationResponse[];
  isLoading?: boolean;
  onNavigateToCreate: () => void;
  onNavigateToDetail: (id: string) => void;
  isAdmin?: boolean;
}

export function CriticalNotificationsPage({
  notifications,
  isLoading,
  onNavigateToCreate,
  onNavigateToDetail,
  isAdmin = false,
}: CriticalNotificationsPageProps) {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<'all' | 'acknowledged' | 'unacknowledged'>('all');

  const filteredNotifications = notifications.filter((n) => {
    if (filter === 'acknowledged') return n.isAcknowledged;
    if (filter === 'unacknowledged') return !n.isAcknowledged;
    return true;
  });

  const acknowledgedCount = notifications.filter((n) => n.isAcknowledged).length;
  const unacknowledgedCount = notifications.filter((n) => !n.isAcknowledged).length;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-red-600" />
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{t('criticalNotifications.title')}</h1>
          <p className="mt-1 text-sm text-gray-500">{t('criticalNotifications.subtitle')}</p>
        </div>
        {isAdmin && (
          <button
            type="button"
            onClick={onNavigateToCreate}
            className="px-4 py-2 text-sm font-medium text-white bg-red-600 hover:bg-red-700 rounded-md"
          >
            {t('criticalNotifications.createNew')}
          </button>
        )}
      </div>

      {/* Stats summary */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <div className="bg-white rounded-lg shadow p-4">
          <p className="text-sm text-gray-500">{t('criticalNotifications.stats.total')}</p>
          <p className="text-2xl font-semibold text-gray-900">{notifications.length}</p>
        </div>
        <div className="bg-white rounded-lg shadow p-4">
          <p className="text-sm text-gray-500">{t('criticalNotifications.stats.acknowledged')}</p>
          <p className="text-2xl font-semibold text-green-600">{acknowledgedCount}</p>
        </div>
        <div className="bg-white rounded-lg shadow p-4">
          <p className="text-sm text-gray-500">{t('criticalNotifications.stats.pending')}</p>
          <p className="text-2xl font-semibold text-red-600">{unacknowledgedCount}</p>
        </div>
      </div>

      {/* Filter tabs */}
      <div className="flex gap-2 mb-4">
        <button
          type="button"
          onClick={() => setFilter('all')}
          className={`px-3 py-1.5 text-sm font-medium rounded-md ${
            filter === 'all'
              ? 'bg-gray-900 text-white'
              : 'bg-white text-gray-700 border border-gray-300 hover:bg-gray-50'
          }`}
        >
          {t('criticalNotifications.filter.all')} ({notifications.length})
        </button>
        <button
          type="button"
          onClick={() => setFilter('unacknowledged')}
          className={`px-3 py-1.5 text-sm font-medium rounded-md ${
            filter === 'unacknowledged'
              ? 'bg-red-600 text-white'
              : 'bg-white text-gray-700 border border-gray-300 hover:bg-gray-50'
          }`}
        >
          {t('criticalNotifications.filter.pending')} ({unacknowledgedCount})
        </button>
        <button
          type="button"
          onClick={() => setFilter('acknowledged')}
          className={`px-3 py-1.5 text-sm font-medium rounded-md ${
            filter === 'acknowledged'
              ? 'bg-green-600 text-white'
              : 'bg-white text-gray-700 border border-gray-300 hover:bg-gray-50'
          }`}
        >
          {t('criticalNotifications.filter.acknowledged')} ({acknowledgedCount})
        </button>
      </div>

      {/* Empty state */}
      {filteredNotifications.length === 0 && (
        <div className="text-center py-12 bg-white rounded-lg shadow">
          <svg
            className="mx-auto h-12 w-12 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
            />
          </svg>
          <p className="mt-4 text-gray-500">{t('criticalNotifications.noResults')}</p>
        </div>
      )}

      {/* Notifications list */}
      {filteredNotifications.length > 0 && (
        <div className="space-y-3">
          {filteredNotifications.map((notification) => (
            <div
              key={notification.id}
              className={`bg-white rounded-lg shadow p-4 border-l-4 ${
                notification.isAcknowledged ? 'border-green-500' : 'border-red-500'
              }`}
            >
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <h3 className="font-semibold text-gray-900">{notification.title}</h3>
                    {!notification.isAcknowledged && (
                      <span className="px-2 py-0.5 text-xs font-medium bg-red-100 text-red-800 rounded">
                        {t('criticalNotifications.requiresAction')}
                      </span>
                    )}
                    {notification.isAcknowledged && (
                      <span className="px-2 py-0.5 text-xs font-medium bg-green-100 text-green-800 rounded">
                        {t('criticalNotifications.acknowledged')}
                      </span>
                    )}
                  </div>
                  <p className="mt-1 text-sm text-gray-600 line-clamp-2">{notification.message}</p>
                  <div className="mt-2 text-xs text-gray-500">
                    {new Date(notification.createdAt).toLocaleString(getActiveLocale())}
                    {notification.acknowledgedAt && (
                      <span className="ml-2">
                        {t('criticalNotifications.acknowledgedAt')}{' '}
                        {new Date(notification.acknowledgedAt).toLocaleString(getActiveLocale())}
                      </span>
                    )}
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => onNavigateToDetail(notification.id)}
                  className="ml-4 text-sm font-medium text-blue-600 hover:text-blue-800"
                >
                  {t('common.view')}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
