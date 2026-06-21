/**
 * Sensor list / management page (Epic 14 — FR71).
 *
 * Standalone device registry: a sortable table of all sensors with register,
 * edit and decommission actions. Complements the read-only dashboard list
 * (`SensorListPanel`) with full CRUD entry points.
 *
 * Presentational only — the route wrapper in `routes/groups/iot.tsx` wires the
 * `@ppt/api-client` hooks and passes data + handlers in.
 */
import type { Sensor } from '@ppt/api-client';
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import { SensorStatusBadge } from '../components';

export interface SensorListPageProps {
  sensors: Sensor[];
  isLoading: boolean;
  deletingSensorId: string | null;
  onNavigateToDashboard: () => void;
  onNavigateToRegister: () => void;
  onNavigateToEdit: (id: string) => void;
  onDelete: (sensor: Sensor) => void;
}

function formatTimestamp(value: string | null): string {
  if (!value) {
    return '—';
  }
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? '—' : date.toLocaleString();
}

export function SensorListPage({
  sensors,
  isLoading,
  deletingSensorId,
  onNavigateToDashboard,
  onNavigateToRegister,
  onNavigateToEdit,
  onDelete,
}: SensorListPageProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <div className="mx-auto max-w-7xl px-4 py-6">
      <header className="mb-6 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold text-gray-900">
            {t('iot.sensorsTitle', { defaultValue: 'Sensors & Devices' })}
          </h1>
          <p className="mt-1 text-sm text-gray-500">
            {t('iot.sensorsSubtitle', {
              defaultValue: 'Register and manage the IoT devices across your buildings.',
            })}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={onNavigateToDashboard}
            className="btn-outline-token rounded-lg px-3 py-1.5 text-sm font-medium"
          >
            {t('iot.openDashboard', { defaultValue: 'Dashboard' })}
          </button>
          <button
            type="button"
            onClick={onNavigateToRegister}
            className="btn-primary-token rounded-lg px-3 py-1.5 text-sm font-medium"
          >
            {t('iot.registerSensor', { defaultValue: 'Register sensor' })}
          </button>
        </div>
      </header>

      <section className="overflow-hidden rounded-lg border border-gray-200 bg-white shadow-sm">
        {isLoading ? (
          <div className="flex items-center justify-center py-16">
            <div className="h-6 w-6 animate-spin rounded-full border-b-2 border-blue-600" />
          </div>
        ) : sensors.length === 0 ? (
          <div className="px-4 py-16 text-center">
            <p className="text-sm text-gray-400">
              {t('iot.noDevices', { defaultValue: 'No devices registered yet.' })}
            </p>
            <button
              type="button"
              onClick={onNavigateToRegister}
              className="btn-primary-token mt-4 rounded-lg px-3 py-1.5 text-sm font-medium"
            >
              {t('iot.registerSensor', { defaultValue: 'Register sensor' })}
            </button>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="min-w-full divide-y divide-gray-100 text-sm">
              <thead className="bg-gray-50 text-left text-xs font-semibold uppercase tracking-wide text-gray-500">
                <tr>
                  <th className="px-4 py-3">{t('iot.colName', { defaultValue: 'Name' })}</th>
                  <th className="px-4 py-3">{t('iot.colType', { defaultValue: 'Type' })}</th>
                  <th className="px-4 py-3">
                    {t('iot.colLocation', { defaultValue: 'Location' })}
                  </th>
                  <th className="px-4 py-3">{t('iot.colStatus', { defaultValue: 'Status' })}</th>
                  <th className="px-4 py-3">
                    {t('iot.colLastReading', { defaultValue: 'Last reading' })}
                  </th>
                  <th className="px-4 py-3 text-right">
                    {t('common.actions', { defaultValue: 'Actions' })}
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {sensors.map((sensor) => {
                  const deleting = sensor.id === deletingSensorId;
                  return (
                    <tr key={sensor.id} className="hover:bg-gray-50">
                      <td className="px-4 py-3 font-medium text-gray-900">{sensor.name}</td>
                      <td className="px-4 py-3 text-gray-600">{sensor.sensor_type}</td>
                      <td className="px-4 py-3 text-gray-600">
                        {sensor.location || sensor.location_description || '—'}
                      </td>
                      <td className="px-4 py-3">
                        <SensorStatusBadge value={sensor.status} />
                      </td>
                      <td className="px-4 py-3 text-gray-500">
                        {formatTimestamp(sensor.last_reading_at)}
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex items-center justify-end gap-2">
                          <button
                            type="button"
                            onClick={() => onNavigateToEdit(sensor.id)}
                            className="rounded px-2 py-1 text-xs font-medium text-blue-600 hover:bg-blue-50"
                          >
                            {t('common.edit', { defaultValue: 'Edit' })}
                          </button>
                          <button
                            type="button"
                            onClick={() => onDelete(sensor)}
                            disabled={deleting}
                            className="rounded px-2 py-1 text-xs font-medium text-red-600 hover:bg-red-50 disabled:opacity-50"
                          >
                            {deleting
                              ? t('common.deleting', { defaultValue: 'Deleting…' })
                              : t('common.delete', { defaultValue: 'Delete' })}
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}
