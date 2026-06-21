/**
 * Predictive Maintenance Dashboard (Epic 13, Story 13.3)
 *
 * Presentational component — the route wrapper in routes/groups/ai-dashboards.tsx
 * wires API hooks and passes data + handlers in.
 */
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import { EquipmentList, PredictionsList } from '../components';
import type { Equipment, MaintenancePrediction } from '../types';

export interface PredictiveMaintenancePageProps {
  equipment: Equipment[];
  equipmentLoading: boolean;
  selectedEquipmentId: string | null;
  onSelectEquipment: (id: string) => void;
  predictions: MaintenancePrediction[];
  predictionsLoading: boolean;
  pendingAcknowledgeId: string | null;
  onAcknowledgePrediction: (id: string) => void;
}

export function PredictiveMaintenancePage({
  equipment,
  equipmentLoading,
  selectedEquipmentId,
  onSelectEquipment,
  predictions,
  predictionsLoading,
  pendingAcknowledgeId,
  onAcknowledgePrediction,
}: PredictiveMaintenancePageProps): JSX.Element {
  const { t } = useTranslation();

  const needsMaintenance = equipment.filter((e) => e.status === 'needs_maintenance').length;
  const highRisk = predictions.filter((p) => p.riskScore >= 0.75 && !p.acknowledged).length;

  return (
    <div className="mx-auto max-w-7xl px-4 py-6">
      <header className="mb-6">
        <h1 className="text-2xl font-semibold text-gray-900">
          {t('predictive.pageTitle', { defaultValue: 'Predictive Maintenance' })}
        </h1>
        <p className="mt-1 text-sm text-gray-500">
          {t('predictive.pageSubtitle', {
            defaultValue: 'AI-powered equipment health monitoring and failure predictions.',
          })}
        </p>
      </header>

      {/* KPI row */}
      <div className="mb-6 grid grid-cols-2 gap-4 md:grid-cols-4">
        <div className="rounded-lg border bg-white p-4 shadow-sm">
          <p className="text-sm font-medium text-gray-500">
            {t('predictive.totalEquipment', { defaultValue: 'Total equipment' })}
          </p>
          <p className="mt-1 text-2xl font-semibold text-gray-900">
            {equipmentLoading ? '—' : equipment.length}
          </p>
        </div>
        <div className="rounded-lg border bg-white p-4 shadow-sm">
          <p className="text-sm font-medium text-gray-500">
            {t('predictive.needsMaintenance', { defaultValue: 'Needs maintenance' })}
          </p>
          <p
            className={`mt-1 text-2xl font-semibold ${needsMaintenance > 0 ? 'text-yellow-600' : 'text-gray-900'}`}
          >
            {equipmentLoading ? '—' : needsMaintenance}
          </p>
        </div>
        <div className="rounded-lg border bg-white p-4 shadow-sm">
          <p className="text-sm font-medium text-gray-500">
            {t('predictive.highRisk', { defaultValue: 'High-risk open' })}
          </p>
          <p
            className={`mt-1 text-2xl font-semibold ${highRisk > 0 ? 'text-red-600' : 'text-gray-900'}`}
          >
            {predictionsLoading ? '—' : highRisk}
          </p>
        </div>
        <div className="rounded-lg border bg-white p-4 shadow-sm">
          <p className="text-sm font-medium text-gray-500">
            {t('predictive.totalPredictions', { defaultValue: 'Total predictions' })}
          </p>
          <p className="mt-1 text-2xl font-semibold text-gray-900">
            {predictionsLoading ? '—' : predictions.length}
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-3">
        <div className="lg:col-span-1">
          <EquipmentList
            equipment={equipment}
            isLoading={equipmentLoading}
            selectedId={selectedEquipmentId}
            onSelect={onSelectEquipment}
          />
        </div>
        <div className="lg:col-span-2">
          <PredictionsList
            predictions={predictions}
            isLoading={predictionsLoading}
            pendingAcknowledgeId={pendingAcknowledgeId}
            onAcknowledge={onAcknowledgePrediction}
          />
        </div>
      </div>
    </div>
  );
}
