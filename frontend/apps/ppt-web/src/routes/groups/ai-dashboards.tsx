/**
 * AI Dashboard route group — Sentiment (13.2) & Predictive Maintenance (13.3).
 *
 * Each route wrapper wires API hooks to the presentational page component,
 * following the same convention as routes/groups/iot.tsx.
 */
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Route } from 'react-router-dom';
import { AuthRequiredGate, ProtectedRoute, useToast } from '../../components';
import { useAuth } from '../../contexts';
import {
  useAcknowledgeMaintenancePrediction,
  useEquipmentList,
  useMaintenancePredictions,
} from '../../features/predictive-maintenance/hooks';
import {
  useAcknowledgeSentimentAlert,
  useSentimentAlerts,
  useSentimentDashboard,
  useSentimentTrends,
} from '../../features/sentiment/hooks';
import { PredictiveMaintenancePage, SentimentDashboardPage } from '../lazyRoutes';
import { MANAGER_ROLES } from '../shared';

// ── Sentiment ──────────────────────────────────────────────────────────────

function SentimentDashboardRoute() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const { showToast } = useToast();
  const {
    data: dashboard,
    isLoading: dashboardLoading,
    isError: dashboardError,
    refetch: refetchDashboard,
  } = useSentimentDashboard();
  const { data: trends, isLoading: trendsLoading } = useSentimentTrends({ limit: 60 });
  const { data: alerts, isLoading: alertsLoading } = useSentimentAlerts(false);
  const acknowledge = useAcknowledgeSentimentAlert();

  const pendingAcknowledgeId = acknowledge.isPending ? (acknowledge.variables ?? null) : null;

  const handleAcknowledge = async (alertId: string) => {
    try {
      await acknowledge.mutateAsync(alertId);
    } catch (error) {
      showToast({
        type: 'error',
        title: t('sentiment.acknowledgeFailed', { defaultValue: 'Failed to acknowledge alert' }),
        error: error,
      });
    }
  };

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  return (
    <SentimentDashboardPage
      dashboard={dashboard}
      dashboardLoading={dashboardLoading}
      trends={trends ?? []}
      trendsLoading={trendsLoading}
      alerts={alerts ?? []}
      alertsLoading={alertsLoading}
      isError={dashboardError}
      onRetry={() => refetchDashboard()}
      pendingAcknowledgeId={pendingAcknowledgeId}
      onAcknowledge={handleAcknowledge}
    />
  );
}

// ── Predictive Maintenance ─────────────────────────────────────────────────

function PredictiveMaintenanceRoute() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const { showToast } = useToast();
  const [selectedEquipmentId, setSelectedEquipmentId] = useState<string | null>(null);

  const { data: equipmentData, isLoading: equipmentLoading } = useEquipmentList();
  const { data: predictions, isLoading: predictionsLoading } = useMaintenancePredictions({
    equipmentId: selectedEquipmentId ?? undefined,
    limit: 50,
  });
  const acknowledge = useAcknowledgeMaintenancePrediction();

  const pendingAcknowledgeId = acknowledge.isPending ? (acknowledge.variables ?? null) : null;

  const handleAcknowledgePrediction = async (predictionId: string) => {
    try {
      await acknowledge.mutateAsync(predictionId);
    } catch (error) {
      showToast({
        type: 'error',
        title: t('predictive.acknowledgeFailed', {
          defaultValue: 'Failed to acknowledge prediction',
        }),
        error: error,
      });
    }
  };

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  return (
    <PredictiveMaintenancePage
      equipment={equipmentData?.equipment ?? []}
      equipmentLoading={equipmentLoading}
      selectedEquipmentId={selectedEquipmentId}
      onSelectEquipment={setSelectedEquipmentId}
      predictions={predictions?.predictions ?? []}
      predictionsLoading={predictionsLoading}
      pendingAcknowledgeId={pendingAcknowledgeId}
      onAcknowledgePrediction={handleAcknowledgePrediction}
    />
  );
}

// ── Route table ────────────────────────────────────────────────────────────

export function aiDashboardRoutes() {
  return (
    <>
      <Route
        path="/ai/sentiment"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <SentimentDashboardRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/ai/predictive-maintenance"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <PredictiveMaintenanceRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
