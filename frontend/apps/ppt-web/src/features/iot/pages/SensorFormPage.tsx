/**
 * Sensor register / edit form (Epic 14 — FR71).
 *
 * One presentational form drives both the register (`mode="create"`) and edit
 * (`mode="edit"`) flows. The route wrapper in `routes/groups/iot.tsx` maps the
 * emitted {@link SensorFormValues} onto the `@ppt/api-client` create/update
 * request shapes.
 *
 * In edit mode the immutable identity fields (building, type, serial number)
 * are rendered read-only because the api-server `UpdateSensor` shape does not
 * accept them; `status` becomes editable instead.
 */
import type { JSX } from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface SensorFormBuildingOption {
  id: string;
  name: string;
}

export interface SensorFormValues {
  buildingId: string;
  name: string;
  sensorType: string;
  location: string;
  locationDescription: string;
  connectionType: string;
  unitOfMeasurement: string;
  dataIntervalSeconds: string;
  manufacturer: string;
  model: string;
  firmwareVersion: string;
  serialNumber: string;
  status: string;
}

export interface SensorFormPageProps {
  mode: 'create' | 'edit';
  buildings: SensorFormBuildingOption[];
  buildingsLoading?: boolean;
  initialValues?: Partial<SensorFormValues>;
  isSaving: boolean;
  isLoading?: boolean;
  onSubmit: (values: SensorFormValues) => void;
  onCancel: () => void;
}

const EMPTY_VALUES: SensorFormValues = {
  buildingId: '',
  name: '',
  sensorType: '',
  location: '',
  locationDescription: '',
  connectionType: '',
  unitOfMeasurement: '',
  dataIntervalSeconds: '',
  manufacturer: '',
  model: '',
  firmwareVersion: '',
  serialNumber: '',
  status: 'active',
};

/** Common sensor types — offered as suggestions, free text still allowed. */
const SENSOR_TYPE_OPTIONS = [
  'temperature',
  'humidity',
  'co2',
  'water_leak',
  'electricity',
  'gas',
  'motion',
  'door',
  'smoke',
  'pressure',
];

const STATUS_OPTIONS = ['active', 'offline', 'error', 'maintenance'];

const fieldClass =
  'mt-1 block w-full rounded-lg border border-gray-300 px-3 py-2 text-sm shadow-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 disabled:bg-gray-100 disabled:text-gray-500';
const labelClass = 'block text-sm font-medium text-gray-700';

export function SensorFormPage({
  mode,
  buildings,
  buildingsLoading = false,
  initialValues,
  isSaving,
  isLoading = false,
  onSubmit,
  onCancel,
}: SensorFormPageProps): JSX.Element {
  const { t } = useTranslation();
  const [values, setValues] = useState<SensorFormValues>({
    ...EMPTY_VALUES,
    ...initialValues,
  });

  const isEdit = mode === 'edit';

  const setField = (key: keyof SensorFormValues, value: string) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  };

  const canSubmit =
    values.name.trim().length > 0 &&
    values.sensorType.trim().length > 0 &&
    (isEdit || values.buildingId.trim().length > 0);

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    if (!canSubmit || isSaving) {
      return;
    }
    onSubmit(values);
  };

  if (isLoading) {
    return (
      <div className="mx-auto max-w-3xl px-4 py-6">
        <div className="flex items-center justify-center py-16">
          <div className="h-6 w-6 animate-spin rounded-full border-b-2 border-blue-600" />
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-3xl px-4 py-6">
      <header className="mb-6">
        <h1 className="text-2xl font-semibold text-gray-900">
          {isEdit
            ? t('iot.editSensorTitle', { defaultValue: 'Edit sensor' })
            : t('iot.registerSensorTitle', { defaultValue: 'Register sensor' })}
        </h1>
        <p className="mt-1 text-sm text-gray-500">
          {isEdit
            ? t('iot.editSensorSubtitle', { defaultValue: 'Update device configuration.' })
            : t('iot.registerSensorSubtitle', {
                defaultValue: 'Add a new IoT device to a building.',
              })}
        </p>
      </header>

      <form
        onSubmit={handleSubmit}
        className="space-y-6 rounded-lg border border-gray-200 bg-white p-6 shadow-sm"
      >
        {/* Identity */}
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label htmlFor="sensor-building" className={labelClass}>
              {t('iot.fieldBuilding', { defaultValue: 'Building' })}
              {!isEdit && <span className="text-red-500"> *</span>}
            </label>
            <select
              id="sensor-building"
              className={fieldClass}
              value={values.buildingId}
              disabled={isEdit || buildingsLoading}
              onChange={(e) => setField('buildingId', e.target.value)}
              required={!isEdit}
            >
              <option value="">
                {buildingsLoading
                  ? t('common.loading', { defaultValue: 'Loading…' })
                  : t('iot.selectBuilding', { defaultValue: 'Select a building' })}
              </option>
              {buildings.map((b) => (
                <option key={b.id} value={b.id}>
                  {b.name}
                </option>
              ))}
            </select>
            {isEdit && (
              <p className="mt-1 text-xs text-gray-400">
                {t('iot.buildingImmutable', {
                  defaultValue: 'Building cannot be changed after registration.',
                })}
              </p>
            )}
          </div>

          <div>
            <label htmlFor="sensor-name" className={labelClass}>
              {t('iot.fieldName', { defaultValue: 'Name' })}
              <span className="text-red-500"> *</span>
            </label>
            <input
              id="sensor-name"
              type="text"
              className={fieldClass}
              value={values.name}
              onChange={(e) => setField('name', e.target.value)}
              required
            />
          </div>

          <div>
            <label htmlFor="sensor-type" className={labelClass}>
              {t('iot.fieldType', { defaultValue: 'Sensor type' })}
              <span className="text-red-500"> *</span>
            </label>
            <input
              id="sensor-type"
              type="text"
              list="sensor-type-options"
              className={fieldClass}
              value={values.sensorType}
              disabled={isEdit}
              onChange={(e) => setField('sensorType', e.target.value)}
              required
            />
            <datalist id="sensor-type-options">
              {SENSOR_TYPE_OPTIONS.map((opt) => (
                <option key={opt} value={opt} />
              ))}
            </datalist>
          </div>

          {isEdit && (
            <div>
              <label htmlFor="sensor-status" className={labelClass}>
                {t('iot.fieldStatus', { defaultValue: 'Status' })}
              </label>
              <select
                id="sensor-status"
                className={fieldClass}
                value={values.status}
                onChange={(e) => setField('status', e.target.value)}
              >
                {STATUS_OPTIONS.map((opt) => (
                  <option key={opt} value={opt}>
                    {opt}
                  </option>
                ))}
              </select>
            </div>
          )}
        </div>

        {/* Placement */}
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label htmlFor="sensor-location" className={labelClass}>
              {t('iot.fieldLocation', { defaultValue: 'Location' })}
            </label>
            <input
              id="sensor-location"
              type="text"
              className={fieldClass}
              value={values.location}
              onChange={(e) => setField('location', e.target.value)}
            />
          </div>
          <div>
            <label htmlFor="sensor-location-desc" className={labelClass}>
              {t('iot.fieldLocationDescription', { defaultValue: 'Location description' })}
            </label>
            <input
              id="sensor-location-desc"
              type="text"
              className={fieldClass}
              value={values.locationDescription}
              onChange={(e) => setField('locationDescription', e.target.value)}
            />
          </div>
        </div>

        {/* Connectivity */}
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
          <div>
            <label htmlFor="sensor-connection" className={labelClass}>
              {t('iot.fieldConnectionType', { defaultValue: 'Connection type' })}
            </label>
            <input
              id="sensor-connection"
              type="text"
              className={fieldClass}
              placeholder="mqtt, http, modbus…"
              value={values.connectionType}
              onChange={(e) => setField('connectionType', e.target.value)}
            />
          </div>
          <div>
            <label htmlFor="sensor-unit" className={labelClass}>
              {t('iot.fieldUnit', { defaultValue: 'Unit of measurement' })}
            </label>
            <input
              id="sensor-unit"
              type="text"
              className={fieldClass}
              placeholder="°C, %, ppm…"
              value={values.unitOfMeasurement}
              onChange={(e) => setField('unitOfMeasurement', e.target.value)}
            />
          </div>
          <div>
            <label htmlFor="sensor-interval" className={labelClass}>
              {t('iot.fieldInterval', { defaultValue: 'Reporting interval (s)' })}
            </label>
            <input
              id="sensor-interval"
              type="number"
              min="0"
              className={fieldClass}
              value={values.dataIntervalSeconds}
              onChange={(e) => setField('dataIntervalSeconds', e.target.value)}
            />
          </div>
        </div>

        {/* Hardware */}
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label htmlFor="sensor-manufacturer" className={labelClass}>
              {t('iot.fieldManufacturer', { defaultValue: 'Manufacturer' })}
            </label>
            <input
              id="sensor-manufacturer"
              type="text"
              className={fieldClass}
              value={values.manufacturer}
              onChange={(e) => setField('manufacturer', e.target.value)}
            />
          </div>
          <div>
            <label htmlFor="sensor-model" className={labelClass}>
              {t('iot.fieldModel', { defaultValue: 'Model' })}
            </label>
            <input
              id="sensor-model"
              type="text"
              className={fieldClass}
              value={values.model}
              onChange={(e) => setField('model', e.target.value)}
            />
          </div>
          <div>
            <label htmlFor="sensor-firmware" className={labelClass}>
              {t('iot.fieldFirmware', { defaultValue: 'Firmware version' })}
            </label>
            <input
              id="sensor-firmware"
              type="text"
              className={fieldClass}
              value={values.firmwareVersion}
              onChange={(e) => setField('firmwareVersion', e.target.value)}
            />
          </div>
          <div>
            <label htmlFor="sensor-serial" className={labelClass}>
              {t('iot.fieldSerial', { defaultValue: 'Serial number' })}
            </label>
            <input
              id="sensor-serial"
              type="text"
              className={fieldClass}
              value={values.serialNumber}
              disabled={isEdit}
              onChange={(e) => setField('serialNumber', e.target.value)}
            />
          </div>
        </div>

        <div className="flex items-center justify-end gap-3 border-t border-gray-100 pt-4">
          <button
            type="button"
            onClick={onCancel}
            className="btn-outline-token rounded-lg px-4 py-2 text-sm font-medium"
          >
            {t('common.cancel', { defaultValue: 'Cancel' })}
          </button>
          <button
            type="submit"
            disabled={!canSubmit || isSaving}
            className="btn-primary-token rounded-lg px-4 py-2 text-sm font-medium disabled:opacity-50"
          >
            {isSaving
              ? t('common.saving', { defaultValue: 'Saving…' })
              : isEdit
                ? t('common.saveChanges', { defaultValue: 'Save changes' })
                : t('iot.registerSensor', { defaultValue: 'Register sensor' })}
          </button>
        </div>
      </form>
    </div>
  );
}
