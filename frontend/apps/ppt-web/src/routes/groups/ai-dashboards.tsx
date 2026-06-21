/**
 * AI Dashboard route group — Sentiment (13.2) & Predictive Maintenance (13.3).
 *
 * Each route wrapper wires API hooks to the presentational page component,
 * following the same convention as routes/groups/iot.tsx.
 */
import { useState } from 'react';
import { Route } from 'react-router-dom';
import {
  useAcknowledgeSentimentAlert,
  useSentimentAlerts,
  useSentimentDashboard,
  useSentimentTrends,
} from '../../features/sentiment/hooks';
import {
  useAcknowledgeMaintenancePrediction,
  useEquipmentList,
  useMaintenancePredictions,
} from '../../features/predictive-maintenance/hooks';
import { PredictiveMaintenancePage, SentimentDashboardPage } from '../lazyRoutes';

// ── Sentiment ──────────────────────────────────────────────────────────────

function SentimentDashboardRoute() {
  const { data: dashboard, isLoading: dashboardLoading } = useSentimentDashboard();
  const { data: trends, isLoading: trendsLoading } = useSentimentTrends({ limit: 60 });
  const { data: alerts, isLoading: alertsLoading } = useSentimentAlerts(false);
  const acknowledge = useAcknowledgeSentimentAlert();

  const pendingAcknowledgeId = acknowledge.isPending ? (acknowledge.variables ?? null) : null;

  return (
    <SentimentDashboardPage
      dashboard={dashboard}
      dashboardLoading={dashboardLoading}
      trends={trends ?? []}
      trendsLoading={trendsLoading}
      alerts={alerts ?? []}
      alertsLoading={alertsLoading}
      pendingAcknowledgeId={pendingAcknowledgeId}
      onAcknowledge={(id) => acknowledge.mutate(id)}
    />
  );
}

// ── Predictive Maintenance ─────────────────────────────────────────────────

function PredictiveMaintenanceRoute() {
  const [selectedEquipmentId, setSelectedEquipmentId] = useState<string | null>(null);

  const { data: equipmentData, isLoading: equipmentLoading } = useEquipmentList();
  const { data: predictions, isLoading: predictionsLoading } = useMaintenancePredictions({
    equipmentId: selectedEquipmentId ?? undefined,
    limit: 50,
  });
  const acknowledge = useAcknowledgeMaintenancePrediction();

  const pendingAcknowledgeId = acknowledge.isPending ? (acknowledge.variables ?? null) : null;

  return (
    <PredictiveMaintenancePage
      equipment={equipmentData?.equipment ?? []}
      equipmentLoading={equipmentLoading}
      selectedEquipmentId={selectedEquipmentId}
      onSelectEquipment={setSelectedEquipmentId}
      predictions={predictions?.predictions ?? []}
      predictionsLoading={predictionsLoading}
      pendingAcknowledgeId={pendingAcknowledgeId}
      onAcknowledgePrediction={(id) => acknowledge.mutate(id)}
    />
  );
}

// ── Route table ────────────────────────────────────────────────────────────

export function aiDashboardRoutes() {
  return (
    <>
      <Route path="/ai/sentiment" element={<SentimentDashboardRoute />} />
      <Route path="/ai/predictive-maintenance" element={<PredictiveMaintenanceRoute />} />
    </>
  );
}
