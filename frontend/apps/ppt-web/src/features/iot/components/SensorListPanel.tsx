/**
 * Device list panel (Epic 14 — FR71). Selectable list of sensors.
 */
import type { Sensor } from '@ppt/api-client';
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import { SensorStatusBadge } from './SensorStatusBadge';

export interface SensorListPanelProps {
  sensors: Sensor[];
  isLoading: boolean;
  selectedSensorId: string | null;
  onSelectSensor: (id: string) => void;
}

export function SensorListPanel({
  sensors,
  isLoading,
  selectedSensorId,
  onSelectSensor,
}: SensorListPanelProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <section className="rounded-lg border border-gray-200 bg-white shadow-sm">
      <header className="border-b border-gray-100 px-4 py-3">
        <h2 className="text-sm font-semibold text-gray-700">
          {t('iot.devices', { defaultValue: 'Devices' })}
          <span className="ml-2 text-xs font-normal text-gray-400">{sensors.length}</span>
        </h2>
      </header>

      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <div className="h-6 w-6 animate-spin rounded-full border-b-2 border-blue-600" />
        </div>
      ) : sensors.length === 0 ? (
        <p className="px-4 py-8 text-center text-sm text-gray-400">
          {t('iot.noDevices', { defaultValue: 'No devices registered yet.' })}
        </p>
      ) : (
        <ul className="divide-y divide-gray-100">
          {sensors.map((sensor) => {
            const selected = sensor.id === selectedSensorId;
            return (
              <li key={sensor.id}>
                <button
                  type="button"
                  onClick={() => onSelectSensor(sensor.id)}
                  className={`flex w-full items-center justify-between gap-3 px-4 py-3 text-left hover:bg-gray-50 ${
                    selected ? 'bg-blue-50' : ''
                  }`}
                >
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium text-gray-900">
                      {sensor.name}
                    </span>
                    <span className="block truncate text-xs text-gray-400">
                      {sensor.sensor_type}
                      {sensor.location ? ` · ${sensor.location}` : ''}
                    </span>
                  </span>
                  <SensorStatusBadge value={sensor.status} />
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
