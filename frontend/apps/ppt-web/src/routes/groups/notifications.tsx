/**
 * Notification analytics route group (Story 2B-C.3, PM #969 gap 4 / BIT-214).
 *
 * Operator dashboard for notification delivery metrics. Owns the filter state
 * (time window + channel) that drives `useNotificationAnalytics` and bridges it
 * to the presentational `NotificationAnalyticsPage`.
 *
 * The endpoint is capability-gated (`audit_read`) and not in the generated
 * client, so the hook calls the REST path directly. Page-level gating mirrors
 * `isManagerRole` (operator-equivalent); the backend remains the source of truth
 * and a 403 surfaces the forbidden notice.
 */
import { useState } from 'react';
import { Route } from 'react-router-dom';
import { useAuth } from '../../contexts';
import type { NotificationAnalyticsFilters } from '../../features/notification-analytics';
import { useNotificationAnalytics } from '../../features/notification-analytics';
import { NotificationAnalyticsPage } from '../lazyRoutes';
import { isManagerRole } from '../shared';

function NotificationAnalyticsPageRoute() {
  const { user } = useAuth();
  const isOperator = isManagerRole(user?.role);

  const [filters, setFilters] = useState<NotificationAnalyticsFilters>({ window: '24h' });

  const { data, isLoading, error, refetch } = useNotificationAnalytics(filters);

  // The handler 403s when the principal lacks `audit_read`; surface the same
  // forbidden notice as a non-operator role rather than a generic error.
  const status = (error as (Error & { status?: number }) | null)?.status;
  const isForbidden = status === 403;

  return (
    <NotificationAnalyticsPage
      isOperator={isOperator}
      data={isOperator ? data : undefined}
      isLoading={isOperator && isLoading}
      isError={!!error && !isForbidden}
      isForbidden={isForbidden}
      onRetry={() => {
        void refetch();
      }}
      filters={filters}
      onFiltersChange={setFilters}
    />
  );
}

/** Notification analytics routes (Story 2B-C.3). */
export function notificationAnalyticsRoutes() {
  return (
    <>
      <Route path="/notifications/analytics" element={<NotificationAnalyticsPageRoute />} />
    </>
  );
}
