/**
 * SensorForm — shared create/edit form for IoT sensors (Epic 14 — FR71).
 *
 * Presentational only; validation is manual (useState). The route wrapper
 * converts SensorFormData to the @ppt/api-client request shape.
 */
import { type FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface SensorFormData {
  name: string;
  sensor_type: string;
  building_id: string;
  location: string;
  unit_of_measurement: string;
  connection_type: string;
  manufacturer: string;
  model: string;
  serial_number: string;
  data_interval_seconds: string; // string for input, convert to number|null on submit
}

interface SensorFormProps {
  initialData?: Partial<SensorFormData>;
  isLoading?: boolean;
  onSubmit: (data: SensorFormData) => void;
  onCancel: () => void;
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
] as const;

const CONNECTION_TYPES = ['mqtt', 'http', 'modbus', 'zigbee', 'zwave', 'other'] as const;

const EMPTY: SensorFormData = {
  name: '',
  sensor_type: '',
  building_id: '',
  location: '',
  unit_of_measurement: '',
  connection_type: '',
  manufacturer: '',
  model: '',
  serial_number: '',
  data_interval_seconds: '',
};

const fieldClass =
  'mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm';

export function SensorForm({ initialData, isLoading, onSubmit, onCancel }: SensorFormProps) {
  const { t } = useTranslation();
  const [formData, setFormData] = useState<SensorFormData>({ ...EMPTY, ...initialData });
  const [errors, setErrors] = useState<Partial<Record<keyof SensorFormData, string>>>({});

  const validate = (): boolean => {
    const next: Partial<Record<keyof SensorFormData, string>> = {};
    if (!formData.name.trim()) {
      next.name = t('iot.errors.nameRequired', { defaultValue: 'Name is required' });
    }
    if (!formData.sensor_type) {
      next.sensor_type = t('iot.errors.sensorTypeRequired', {
        defaultValue: 'Sensor type is required',
      });
    }
    if (!formData.building_id.trim()) {
      next.building_id = t('iot.errors.buildingIdRequired', {
        defaultValue: 'Building ID is required',
      });
    }
    setErrors(next);
    return Object.keys(next).length === 0;
  };

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (validate()) {
      onSubmit(formData);
    }
  };

  const set = (key: keyof SensorFormData, value: string) =>
    setFormData((prev: SensorFormData) => ({ ...prev, [key]: value }));

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Name */}
      <div>
        <label htmlFor="sf-name" className="block text-sm font-medium text-gray-700">
          {t('iot.form.name', { defaultValue: 'Name' })} *
        </label>
        <input
          type="text"
          id="sf-name"
          value={formData.name}
          onChange={(e) => set('name', e.target.value)}
          placeholder={t('iot.form.namePlaceholder', {
            defaultValue: 'e.g. Living room temperature',
          })}
          className={`${fieldClass} ${errors.name ? 'border-red-300' : 'border-gray-300'}`}
        />
        {errors.name && <p className="mt-1 text-sm text-red-600">{errors.name}</p>}
      </div>

      {/* Sensor Type */}
      <div>
        <label htmlFor="sf-sensor-type" className="block text-sm font-medium text-gray-700">
          {t('iot.form.sensorType', { defaultValue: 'Sensor Type' })} *
        </label>
        <select
          id="sf-sensor-type"
          value={formData.sensor_type}
          onChange={(e) => set('sensor_type', e.target.value)}
          className={`${fieldClass} ${errors.sensor_type ? 'border-red-300' : 'border-gray-300'}`}
        >
          <option value="">—</option>
          {SENSOR_TYPES.map((type) => (
            <option key={type} value={type}>
              {t(`iot.sensorType.${type}`, { defaultValue: type })}
            </option>
          ))}
        </select>
        {errors.sensor_type && <p className="mt-1 text-sm text-red-600">{errors.sensor_type}</p>}
      </div>

      {/* Building ID */}
      <div>
        <label htmlFor="sf-building-id" className="block text-sm font-medium text-gray-700">
          {t('iot.form.buildingId', { defaultValue: 'Building ID' })} *
        </label>
        <input
          type="text"
          id="sf-building-id"
          value={formData.building_id}
          onChange={(e) => set('building_id', e.target.value)}
          placeholder={t('iot.form.buildingIdPlaceholder', { defaultValue: 'Enter building ID' })}
          className={`${fieldClass} ${errors.building_id ? 'border-red-300' : 'border-gray-300'}`}
        />
        {errors.building_id && <p className="mt-1 text-sm text-red-600">{errors.building_id}</p>}
      </div>

      {/* Location */}
      <div>
        <label htmlFor="sf-location" className="block text-sm font-medium text-gray-700">
          {t('iot.form.location', { defaultValue: 'Location' })}
        </label>
        <input
          type="text"
          id="sf-location"
          value={formData.location}
          onChange={(e) => set('location', e.target.value)}
          placeholder={t('iot.form.locationPlaceholder', { defaultValue: 'e.g. Room 101' })}
          className={`${fieldClass} border-gray-300`}
        />
      </div>

      {/* Unit of Measurement + Connection Type */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label htmlFor="sf-unit" className="block text-sm font-medium text-gray-700">
            {t('iot.form.unitOfMeasurement', { defaultValue: 'Unit of Measurement' })}
          </label>
          <input
            type="text"
            id="sf-unit"
            value={formData.unit_of_measurement}
            onChange={(e) => set('unit_of_measurement', e.target.value)}
            placeholder={t('iot.form.unitPlaceholder', { defaultValue: 'e.g. °C, %, ppm' })}
            className={`${fieldClass} border-gray-300`}
          />
        </div>
        <div>
          <label htmlFor="sf-connection" className="block text-sm font-medium text-gray-700">
            {t('iot.form.connectionType', { defaultValue: 'Connection Type' })}
          </label>
          <select
            id="sf-connection"
            value={formData.connection_type}
            onChange={(e) => set('connection_type', e.target.value)}
            className={`${fieldClass} border-gray-300`}
          >
            <option value="">—</option>
            {CONNECTION_TYPES.map((ct) => (
              <option key={ct} value={ct}>
                {t(`iot.connectionType.${ct}`, { defaultValue: ct })}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Manufacturer + Model */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label htmlFor="sf-manufacturer" className="block text-sm font-medium text-gray-700">
            {t('iot.form.manufacturer', { defaultValue: 'Manufacturer' })}
          </label>
          <input
            type="text"
            id="sf-manufacturer"
            value={formData.manufacturer}
            onChange={(e) => set('manufacturer', e.target.value)}
            placeholder={t('iot.form.manufacturerPlaceholder', { defaultValue: 'e.g. Bosch' })}
            className={`${fieldClass} border-gray-300`}
          />
        </div>
        <div>
          <label htmlFor="sf-model" className="block text-sm font-medium text-gray-700">
            {t('iot.form.model', { defaultValue: 'Model' })}
          </label>
          <input
            type="text"
            id="sf-model"
            value={formData.model}
            onChange={(e) => set('model', e.target.value)}
            placeholder={t('iot.form.modelPlaceholder', { defaultValue: 'e.g. BME280' })}
            className={`${fieldClass} border-gray-300`}
          />
        </div>
      </div>

      {/* Serial Number + Data Interval */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label htmlFor="sf-serial" className="block text-sm font-medium text-gray-700">
            {t('iot.form.serialNumber', { defaultValue: 'Serial Number' })}
          </label>
          <input
            type="text"
            id="sf-serial"
            value={formData.serial_number}
            onChange={(e) => set('serial_number', e.target.value)}
            placeholder={t('iot.form.serialNumberPlaceholder', {
              defaultValue: 'e.g. SN-123456',
            })}
            className={`${fieldClass} border-gray-300`}
          />
        </div>
        <div>
          <label htmlFor="sf-interval" className="block text-sm font-medium text-gray-700">
            {t('iot.form.dataIntervalSeconds', { defaultValue: 'Data Interval (seconds)' })}
          </label>
          <input
            type="number"
            id="sf-interval"
            min="0"
            value={formData.data_interval_seconds}
            onChange={(e) => set('data_interval_seconds', e.target.value)}
            placeholder={t('iot.form.dataIntervalPlaceholder', { defaultValue: 'e.g. 60' })}
            className={`${fieldClass} border-gray-300`}
          />
        </div>
      </div>

      {/* Actions */}
      <div className="flex items-center justify-end gap-3 pt-4 border-t">
        <button
          type="button"
          onClick={onCancel}
          disabled={isLoading}
          className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
        >
          {t('common.cancel', { defaultValue: 'Cancel' })}
        </button>
        <button
          type="submit"
          disabled={isLoading}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50 flex items-center gap-2"
        >
          {isLoading && (
            <span className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
          )}
          {t('common.save', { defaultValue: 'Save' })}
        </button>
      </div>
    </form>
  );
}
