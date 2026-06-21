/**
 * IotSensorEditPage — edit-sensor form wrapper (Epic 14 — FR71).
 * Presentational; route wrapper wires the useUpdateSensor mutation.
 */
import type { Sensor } from '@ppt/api-client';
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import { SensorForm, type SensorFormData } from '../components';

export interface IotSensorEditPageProps {
  sensor?: Sensor;
  isLoading?: boolean;
  onSubmit: (data: SensorFormData) => void;
  onCancel: () => void;
}

export function IotSensorEditPage({
  sensor,
  isLoading,
  onSubmit,
  onCancel,
}: IotSensorEditPageProps): JSX.Element {
  const { t } = useTranslation();

  const initialData: Partial<SensorFormData> | undefined = sensor
    ? {
        name: sensor.name,
        sensor_type: sensor.sensor_type,
        building_id: sensor.building_id,
        location: sensor.location ?? '',
        unit_of_measurement: sensor.unit_of_measurement ?? '',
        connection_type: sensor.connection_type ?? '',
        manufacturer: sensor.manufacturer ?? '',
        model: sensor.model ?? '',
        serial_number: sensor.serial_number ?? '',
        data_interval_seconds: sensor.data_interval_seconds != null
          ? String(sensor.data_interval_seconds)
          : '',
      }
    : undefined;

  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      <button
        type="button"
        onClick={onCancel}
        className="mb-4 text-blue-600 hover:text-blue-800 flex items-center gap-1"
      >
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        {t('iot.backToSensors', { defaultValue: 'Back to Sensors' })}
      </button>

      <div className="bg-white rounded-lg shadow">
        <div className="p-6 border-b">
          <h1 className="text-2xl font-bold text-gray-900">
            {t('iot.editSensor', { defaultValue: 'Edit Sensor' })}
          </h1>
        </div>
        <div className="p-6">
          {!sensor && isLoading ? (
            <div className="flex items-center justify-center py-16">
              <div className="h-6 w-6 animate-spin rounded-full border-b-2 border-blue-600" />
            </div>
          ) : (
            <SensorForm
              initialData={initialData}
              isLoading={isLoading}
              onSubmit={onSubmit}
              onCancel={onCancel}
            />
          )}
        </div>
      </div>
    </div>
  );
}
