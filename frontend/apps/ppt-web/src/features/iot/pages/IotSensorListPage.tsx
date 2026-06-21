/**
 * IotSensorListPage — device registry list with filter bar and CRUD actions.
 * Epic 14 — FR71. Presentational only; route wrapper wires hooks.
 */
import type { Sensor } from '@ppt/api-client';
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import { SensorStatusBadge } from '../components';

export interface IotSensorListPageProps {
  sensors: Sensor[];
  isLoading: boolean;
  total: number;
  page: number;
  pageSize: number;
  filterType: string;
  filterStatus: string;
  filterSearch: string;
  onPageChange: (page: number) => void;
  onFilterTypeChange: (type: string) => void;
  onFilterStatusChange: (status: string) => void;
  onFilterSearchChange: (search: string) => void;
  onRegister: () => void;
  onEdit: (id: string) => void;
}

const SENSOR_TYPES = [
  'temperature',
  'humidity',
  'motion',
  'co2',
  'pressure',
  'energy',
  'water',
  'smoke',
  'door_window',
  'other',
];

const SENSOR_STATUSES = ['active', 'offline', 'error', 'maintenance'];

export function IotSensorListPage({
  sensors,
  isLoading,
  total,
  page,
  pageSize,
  filterType,
  filterStatus,
  filterSearch,
  onPageChange,
  onFilterTypeChange,
  onFilterStatusChange,
  onFilterSearchChange,
  onRegister,
  onEdit,
}: IotSensorListPageProps): JSX.Element {
  const { t } = useTranslation();
  const totalPages = Math.ceil(total / pageSize) || 1;

  return (
    <div className="mx-auto max-w-7xl px-4 py-6">
      <header className="mb-6 flex flex-wrap items-start justify-between gap-3">
        <h1 className="text-2xl font-semibold text-gray-900">
          {t('iot.title', { defaultValue: 'IoT Sensors' })}
        </h1>
        <button
          type="button"
          onClick={onRegister}
          className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700"
        >
          {t('iot.registerSensor', { defaultValue: 'Register Sensor' })}
        </button>
      </header>

      {/* Filter bar */}
      <div className="mb-4 flex flex-wrap gap-3">
        <input
          type="text"
          value={filterSearch}
          onChange={(e) => onFilterSearchChange(e.target.value)}
          placeholder={t('iot.filter.searchPlaceholder', { defaultValue: 'Search sensors...' })}
          className="rounded-md border border-gray-300 px-3 py-2 text-sm shadow-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
        />
        <select
          value={filterType}
          onChange={(e) => onFilterTypeChange(e.target.value)}
          className="rounded-md border border-gray-300 px-3 py-2 text-sm shadow-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
        >
          <option value="">{t('iot.filter.allTypes', { defaultValue: 'All Types' })}</option>
          {SENSOR_TYPES.map((type) => (
            <option key={type} value={type}>
              {t(`iot.sensorType.${type}`, { defaultValue: type })}
            </option>
          ))}
        </select>
        <select
          value={filterStatus}
          onChange={(e) => onFilterStatusChange(e.target.value)}
          className="rounded-md border border-gray-300 px-3 py-2 text-sm shadow-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
        >
          <option value="">{t('iot.filter.allStatuses', { defaultValue: 'All Statuses' })}</option>
          {SENSOR_STATUSES.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </div>

      <section className="overflow-hidden rounded-lg border border-gray-200 bg-white shadow-sm">
        {isLoading ? (
          <div className="flex items-center justify-center py-16">
            <div className="h-6 w-6 animate-spin rounded-full border-b-2 border-blue-600" />
          </div>
        ) : sensors.length === 0 ? (
          <div className="px-4 py-16 text-center">
            <p className="text-sm text-gray-400">
              {t('iot.noSensors', { defaultValue: 'No sensors found.' })}
            </p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="min-w-full divide-y divide-gray-100 text-sm">
              <thead className="bg-gray-50 text-left text-xs font-semibold uppercase tracking-wide text-gray-500">
                <tr>
                  <th className="px-4 py-3">{t('iot.columns.name', { defaultValue: 'Name' })}</th>
                  <th className="px-4 py-3">{t('iot.columns.type', { defaultValue: 'Type' })}</th>
                  <th className="px-4 py-3">
                    {t('iot.columns.status', { defaultValue: 'Status' })}
                  </th>
                  <th className="px-4 py-3">
                    {t('iot.columns.location', { defaultValue: 'Location' })}
                  </th>
                  <th className="px-4 py-3">
                    {t('iot.columns.building', { defaultValue: 'Building' })}
                  </th>
                  <th className="px-4 py-3 text-right">
                    {t('iot.columns.actions', { defaultValue: 'Actions' })}
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {sensors.map((sensor) => (
                  <tr key={sensor.id} className="hover:bg-gray-50">
                    <td className="px-4 py-3 font-medium text-gray-900">{sensor.name}</td>
                    <td className="px-4 py-3 text-gray-600">{sensor.sensor_type}</td>
                    <td className="px-4 py-3">
                      <SensorStatusBadge value={sensor.status} />
                    </td>
                    <td className="px-4 py-3 text-gray-600">{sensor.location ?? '—'}</td>
                    <td className="px-4 py-3 text-gray-600">{sensor.building_id}</td>
                    <td className="px-4 py-3 text-right">
                      <button
                        type="button"
                        onClick={() => onEdit(sensor.id)}
                        className="rounded px-2 py-1 text-xs font-medium text-blue-600 hover:bg-blue-50"
                      >
                        {t('iot.actions.edit', { defaultValue: 'Edit' })}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="mt-4 flex items-center justify-end gap-2">
          <button
            type="button"
            onClick={() => onPageChange(page - 1)}
            disabled={page <= 1}
            className="rounded px-3 py-1 text-sm text-gray-700 border border-gray-300 disabled:opacity-40 hover:bg-gray-50"
          >
            &lt;
          </button>
          <span className="text-sm text-gray-600">
            {page} / {totalPages}
          </span>
          <button
            type="button"
            onClick={() => onPageChange(page + 1)}
            disabled={page >= totalPages}
            className="rounded px-3 py-1 text-sm text-gray-700 border border-gray-300 disabled:opacity-40 hover:bg-gray-50"
          >
            &gt;
          </button>
        </div>
      )}
    </div>
  );
}
