/**
 * BuildingFormPage - create a building or edit an existing one.
 *
 * Create → `POST /api/v1/buildings` (organization from the active tenant).
 * Edit   → `PUT /api/v1/buildings/{id}`.
 * The api-client maps the camelCase form onto the server's flat snake_case
 * body (`toApiCreateBuilding` / `toApiUpdateBuilding`).
 */

import type { Building } from '@ppt/api-client';
import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useBuilding, useBuildingMutations } from '../hooks';

interface BuildingFormPageProps {
  /** Omit to create a new building. */
  buildingId?: string;
  onSaved: (id: string) => void;
  onCancel: () => void;
}

export interface BuildingFormValues {
  name: string;
  street: string;
  city: string;
  postalCode: string;
  country: string;
  yearBuilt: string;
  floorCount: string;
  totalEntrances: string;
  description: string;
}

const inputClass =
  'w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500';

export function valuesFromBuilding(b?: Building): BuildingFormValues {
  return {
    name: b?.name ?? '',
    street: b?.address.street ?? '',
    city: b?.address.city ?? '',
    postalCode: b?.address.postalCode ?? '',
    country: b?.address.country || 'SK',
    yearBuilt: b?.yearBuilt != null ? String(b.yearBuilt) : '',
    floorCount: b?.floorCount ? String(b.floorCount) : '1',
    totalEntrances: b?.totalEntrances ? String(b.totalEntrances) : '1',
    description: b?.description ?? '',
  };
}

/** Returns a map of field → i18n error key (relative to `buildings.form.errors`). */
export function validateBuilding(v: BuildingFormValues): Record<string, string> {
  const errors: Record<string, string> = {};
  if (!v.street.trim()) errors.street = 'streetRequired';
  if (!v.city.trim()) errors.city = 'cityRequired';
  if (!v.postalCode.trim()) errors.postalCode = 'postalCodeRequired';
  const floors = Number(v.floorCount);
  if (!Number.isInteger(floors) || floors < 1) errors.floorCount = 'floorsInvalid';
  const entrances = Number(v.totalEntrances);
  if (!Number.isInteger(entrances) || entrances < 1) errors.totalEntrances = 'entrancesInvalid';
  if (v.yearBuilt.trim()) {
    const year = Number(v.yearBuilt);
    if (!Number.isInteger(year) || year < 1000 || year > new Date().getFullYear() + 5)
      errors.yearBuilt = 'yearInvalid';
  }
  return errors;
}

export function BuildingFormPage({ buildingId, onSaved, onCancel }: BuildingFormPageProps) {
  const isEdit = !!buildingId;
  const { data: building, isLoading } = useBuilding(buildingId ?? '', isEdit);

  if (isEdit && isLoading) {
    return <div className="max-w-2xl mx-auto px-4 py-8 animate-pulse h-64 bg-gray-100 rounded" />;
  }
  // Key on the loaded building so the form re-initialises once data arrives.
  return (
    <BuildingForm
      key={building?.id ?? 'new'}
      buildingId={buildingId}
      initial={valuesFromBuilding(isEdit ? building : undefined)}
      onSaved={onSaved}
      onCancel={onCancel}
    />
  );
}

function BuildingForm({
  buildingId,
  initial,
  onSaved,
  onCancel,
}: {
  buildingId?: string;
  initial: BuildingFormValues;
  onSaved: (id: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation();
  const [values, setValues] = useState<BuildingFormValues>(initial);
  const [touched, setTouched] = useState(false);
  const { useCreate, useUpdate } = useBuildingMutations();
  const create = useCreate();
  const update = useUpdate();
  const saving = create.isPending || update.isPending;
  const saveError = create.error ?? update.error;

  const errors = validateBuilding(values);
  const set =
    (field: keyof BuildingFormValues) =>
    (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) =>
      setValues((prev) => ({ ...prev, [field]: e.target.value }));

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setTouched(true);
    if (Object.keys(errors).length > 0) return;
    const payload = {
      name: values.name.trim() || undefined,
      address: {
        street: values.street.trim(),
        city: values.city.trim(),
        postalCode: values.postalCode.trim(),
        country: values.country.trim().toUpperCase() || 'SK',
      },
      yearBuilt: values.yearBuilt.trim() ? Number(values.yearBuilt) : undefined,
      floorCount: Number(values.floorCount),
      totalEntrances: Number(values.totalEntrances),
      description: values.description.trim() || undefined,
    };
    if (buildingId) {
      update.mutate({ id: buildingId, data: payload }, { onSuccess: () => onSaved(buildingId) });
    } else {
      create.mutate(
        { ...payload, name: payload.name ?? payload.address.street },
        { onSuccess: (b) => onSaved(b.id) }
      );
    }
  };

  const field = (
    name: keyof BuildingFormValues,
    labelKey: string,
    opts: { type?: string; required?: boolean; autoComplete?: string } = {}
  ) => {
    const err = touched ? errors[name] : undefined;
    const id = `building-${name}`;
    return (
      <div>
        <label htmlFor={id} className="block text-sm font-medium text-gray-700 mb-1">
          {t(labelKey)}
          {opts.required && <span aria-hidden="true"> *</span>}
        </label>
        <input
          id={id}
          type={opts.type ?? 'text'}
          className={inputClass}
          value={values[name]}
          onChange={set(name)}
          autoComplete={opts.autoComplete}
          aria-invalid={!!err}
          aria-describedby={err ? `${id}-error` : undefined}
        />
        {err && (
          <p id={`${id}-error`} className="mt-1 text-sm text-red-600">
            {t(`buildings.form.errors.${err}`)}
          </p>
        )}
      </div>
    );
  };

  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      <h1 className="text-2xl font-bold text-gray-900 mb-6">
        {buildingId ? t('buildings.form.editTitle') : t('buildings.form.createTitle')}
      </h1>
      <form onSubmit={handleSubmit} noValidate className="bg-white rounded-lg shadow p-6 space-y-4">
        {saveError && (
          <div
            role="alert"
            className="rounded-md bg-red-50 border border-red-200 p-3 text-sm text-red-700"
          >
            {t('buildings.form.saveFailed')}
          </div>
        )}
        {field('name', 'buildings.form.name')}
        {field('street', 'buildings.form.street', {
          required: true,
          autoComplete: 'street-address',
        })}
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          {field('postalCode', 'buildings.form.postalCode', {
            required: true,
            autoComplete: 'postal-code',
          })}
          {field('city', 'buildings.form.city', { required: true, autoComplete: 'address-level2' })}
          {field('country', 'buildings.form.country', { autoComplete: 'country' })}
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          {field('floorCount', 'buildings.form.floorCount', { type: 'number', required: true })}
          {field('totalEntrances', 'buildings.form.totalEntrances', {
            type: 'number',
            required: true,
          })}
          {field('yearBuilt', 'buildings.form.yearBuilt', { type: 'number' })}
        </div>
        <div>
          <label
            htmlFor="building-description"
            className="block text-sm font-medium text-gray-700 mb-1"
          >
            {t('buildings.form.description')}
          </label>
          <textarea
            id="building-description"
            rows={3}
            className={inputClass}
            value={values.description}
            onChange={set('description')}
          />
        </div>
        <div className="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onClick={onCancel}
            className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
          >
            {t('common.cancel')}
          </button>
          <button
            type="submit"
            disabled={saving}
            className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
          >
            {saving ? t('buildings.form.saving') : t('common.save')}
          </button>
        </div>
      </form>
    </div>
  );
}

BuildingFormPage.displayName = 'BuildingFormPage';
