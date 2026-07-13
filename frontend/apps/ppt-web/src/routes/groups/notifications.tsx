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
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Route } from 'react-router-dom';
import { useToast } from '../../components';
import { useAuth } from '../../contexts';
import type { NotificationAnalyticsFilters } from '../../features/notification-analytics';
import { useNotificationAnalytics } from '../../features/notification-analytics';
import type { TriggerChannel } from '../../features/notification-triggers';
import {
  useNotificationTriggers,
  useResetNotificationTriggers,
  useUpdateNotificationTrigger,
} from '../../features/notification-triggers';
import { NotificationAnalyticsPage, NotificationTriggersPage } from '../lazyRoutes';
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

/**
 * Notification trigger management route (Story 84.4 — Notification Trigger System,
 * PM gap 84-4).
 *
 * Owns the granular-events query + per-trigger channel mutations and the reset
 * action, bridging them to the presentational `NotificationTriggersPage`. Any
 * authenticated user manages their own triggers; a 401/403 surfaces the same
 * forbidden notice. The endpoint is not in the generated client, so the hooks
 * call the REST path directly.
 */
function NotificationTriggersPageRoute() {
  const { t } = useTranslation();
  const { showToast } = useToast();

  const { data, isLoading, error, refetch } = useNotificationTriggers();
  const updateTrigger = useUpdateNotificationTrigger();
  const resetTriggers = useResetNotificationTriggers();

  const status = (error as (Error & { status?: number }) | null)?.status;
  const isForbidden = status === 401 || status === 403;

  const onToggle = useCallback(
    (eventType: string, channel: TriggerChannel, enabled: boolean) => {
      const patch =
        channel === 'push'
          ? { pushEnabled: enabled }
          : channel === 'email'
            ? { emailEnabled: enabled }
            : { inAppEnabled: enabled };
      updateTrigger.mutate(
        { eventType, patch },
        {
          onError: (err) => {
            showToast({
              type: 'error',
              title: t('notificationTriggers.saveFailed', {
                defaultValue: 'Could not update trigger',
              }),
              message: err instanceof Error ? err.message : String(err),
            });
          },
        }
      );
    },
    [updateTrigger, showToast, t]
  );

  const onReset = useCallback(() => {
    resetTriggers.mutate(undefined, {
      onSuccess: () => {
        showToast({
          type: 'success',
          title: t('notificationTriggers.resetDone', {
            defaultValue: 'Triggers reset to defaults',
          }),
        });
      },
      onError: (err) => {
        showToast({
          type: 'error',
          title: t('notificationTriggers.resetFailed', {
            defaultValue: 'Could not reset triggers',
          }),
          message: err instanceof Error ? err.message : String(err),
        });
      },
    });
  }, [resetTriggers, showToast, t]);

  const savingEventType = updateTrigger.isPending
    ? (updateTrigger.variables?.eventType ?? null)
    : null;

  return (
    <NotificationTriggersPage
      data={data}
      isLoading={isLoading}
      isError={!!error && !isForbidden}
      isForbidden={isForbidden}
      onRetry={() => {
        void refetch();
      }}
      savingEventType={savingEventType}
      onToggle={onToggle}
      onReset={onReset}
      isResetting={resetTriggers.isPending}
    />
  );
}

/** Notification analytics + trigger-management routes (Stories 2B-C.3, 84.4). */
export function notificationAnalyticsRoutes() {
  return (
    <>
      <Route path="/notifications/analytics" element={<NotificationAnalyticsPageRoute />} />
      <Route path="/notifications/triggers" element={<NotificationTriggersPageRoute />} />
    </>
  );
}
