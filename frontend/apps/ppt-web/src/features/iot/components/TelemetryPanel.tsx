/**
 * Telemetry panel (Epic 14 — FR72). Recent readings for the selected sensor.
 */
import type { Sensor, SensorReading } from '@ppt/api-client';
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';

export interface TelemetryPanelProps {
  sensor: Sensor | null;
  readings: SensorReading[];
  isLoading: boolean;
}

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
}

export function TelemetryPanel({ sensor, readings, isLoading }: TelemetryPanelProps): JSX.Element {
  const { t } = useTranslation();

  if (!sensor) {
    return (
      <section className="rounded-lg border border-gray-200 bg-white p-8 text-center shadow-sm">
        <p className="text-sm text-gray-400">
          {t('iot.selectDevicePrompt', {
            defaultValue: 'Select a device to view its telemetry.',
          })}
        </p>
      </section>
    );
  }

  const latest = readings[0];

  return (
    <section className="rounded-lg border border-gray-200 bg-white shadow-sm">
      <header className="border-b border-gray-100 px-4 py-3">
        <h2 className="text-sm font-semibold text-gray-700">
          {t('iot.telemetry', { defaultValue: 'Telemetry' })}
          <span className="ml-2 text-xs font-normal text-gray-400">{sensor.name}</span>
        </h2>
      </header>

      {latest && (
        <div className="border-b border-gray-100 px-4 py-3">
          <span className="text-3xl font-semibold text-gray-900">{latest.value}</span>
          <span className="ml-1 text-sm text-gray-500">
            {latest.unit || sensor.unit_of_measurement || ''}
          </span>
          <div className="mt-1 text-xs text-gray-400">
            {t('iot.lastReading', { defaultValue: 'Last reading' })}:{' '}
            {formatTimestamp(latest.timestamp)}
          </div>
        </div>
      )}

      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <div className="h-6 w-6 animate-spin rounded-full border-b-2 border-blue-600" />
        </div>
      ) : readings.length === 0 ? (
        <p className="px-4 py-8 text-center text-sm text-gray-400">
          {t('iot.noReadings', { defaultValue: 'No readings in the selected window.' })}
        </p>
      ) : (
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-xs uppercase tracking-wide text-gray-400">
              <th className="px-4 py-2 font-medium">
                {t('iot.time', { defaultValue: 'Time' })}
              </th>
              <th className="px-4 py-2 text-right font-medium">
                {t('iot.value', { defaultValue: 'Value' })}
              </th>
              <th className="px-4 py-2 font-medium">
                {t('iot.quality', { defaultValue: 'Quality' })}
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-50">
            {readings.map((r) => (
              <tr key={r.id}>
                <td className="px-4 py-2 text-gray-500">{formatTimestamp(r.timestamp)}</td>
                <td className="px-4 py-2 text-right font-medium text-gray-900">
                  {r.value}
                  <span className="ml-1 text-xs text-gray-400">{r.unit}</span>
                </td>
                <td className="px-4 py-2 text-gray-400">{r.quality ?? '—'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
