/**
 * IotSensorRegisterPage — create-sensor form wrapper (Epic 14 — FR71).
 * Presentational; route wrapper wires the useCreateSensor mutation.
 */
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import { SensorForm, type SensorFormData } from '../components';

export interface IotSensorRegisterPageProps {
  isLoading?: boolean;
  onSubmit: (data: SensorFormData) => void;
  onCancel: () => void;
}

export function IotSensorRegisterPage({
  isLoading,
  onSubmit,
  onCancel,
}: IotSensorRegisterPageProps): JSX.Element {
  const { t } = useTranslation();

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
            {t('iot.registerSensor', { defaultValue: 'Register Sensor' })}
          </h1>
        </div>
        <div className="p-6">
          <SensorForm isLoading={isLoading} onSubmit={onSubmit} onCancel={onCancel} />
        </div>
      </div>
    </div>
  );
}
