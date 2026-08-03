/**
 * UnitForm - dialog for creating or editing a unit in a building.
 *
 * Story 3.2: Unit management UI. Presentational only — it owns local field
 * state and validation and forwards a {@link UnitFormValues} payload to
 * `onSubmit`; data fetching/mutations live in {@link UnitsSection}.
 */

import type { UnitOccupancyStatus, UnitType } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

/** Values emitted by the form on submit (decimals are strings, matching the API). */
export interface UnitFormValues {
  designation: string;
  entrance?: string;
  floor: number;
  unit_type: UnitType;
  size_sqm?: string;
  rooms?: number;
  ownership_share: string;
  occupancy_status: UnitOccupancyStatus;
  description?: string;
}

interface UnitFormProps {
  isOpen: boolean;
  mode: 'create' | 'edit';
  /** Pre-filled values when editing. */
  initial?: Partial<UnitFormValues>;
  isSubmitting?: boolean;
  /** Server-side error from the mutation, shown in the form. */
  error?: string | null;
  onSubmit: (values: UnitFormValues) => void;
  onClose: () => void;
}

export const UNIT_TYPE_OPTIONS: { value: UnitType; label: string }[] = [
  { value: 'apartment', label: 'Apartment' },
  { value: 'commercial', label: 'Commercial' },
  { value: 'parking', label: 'Parking' },
  { value: 'storage', label: 'Storage' },
  { value: 'other', label: 'Other' },
];

export const OCCUPANCY_OPTIONS: { value: UnitOccupancyStatus; label: string }[] = [
  { value: 'unknown', label: 'Unknown' },
  { value: 'owner_occupied', label: 'Owner occupied' },
  { value: 'rented', label: 'Rented' },
  { value: 'vacant', label: 'Vacant' },
];

const inputClass =
  'w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500';

export function UnitForm({
  isOpen,
  mode,
  initial,
  isSubmitting,
  error,
  onSubmit,
  onClose,
}: UnitFormProps) {
  const { t } = useTranslation();
  const [designation, setDesignation] = useState(initial?.designation ?? '');
  const [entrance, setEntrance] = useState(initial?.entrance ?? '');
  const [floor, setFloor] = useState(String(initial?.floor ?? 0));
  const [unitType, setUnitType] = useState<UnitType>(initial?.unit_type ?? 'apartment');
  const [sizeSqm, setSizeSqm] = useState(initial?.size_sqm ?? '');
  const [rooms, setRooms] = useState(initial?.rooms != null ? String(initial.rooms) : '');
  const [ownershipShare, setOwnershipShare] = useState(initial?.ownership_share ?? '100');
  const [occupancyStatus, setOccupancyStatus] = useState<UnitOccupancyStatus>(
    initial?.occupancy_status ?? 'unknown'
  );
  const [description, setDescription] = useState(initial?.description ?? '');

  if (!isOpen) return null;

  const trimmedDesignation = designation.trim();
  const floorNum = Number(floor);
  const ownershipNum = Number(ownershipShare);
  const sizeNum = sizeSqm.trim() === '' ? null : Number(sizeSqm);
  const roomsNum = rooms.trim() === '' ? null : Number(rooms);

  const errors: Record<string, string> = {};
  if (!trimmedDesignation) errors.designation = 'Designation is required.';
  else if (trimmedDesignation.length > 50)
    errors.designation = 'Designation must be 50 characters or fewer.';
  if (floor.trim() === '' || !Number.isInteger(floorNum))
    errors.floor = 'Floor must be a whole number.';
  if (ownershipShare.trim() === '' || Number.isNaN(ownershipNum))
    errors.ownership_share = 'Ownership share is required.';
  else if (ownershipNum < 0 || ownershipNum > 100)
    errors.ownership_share = 'Ownership share must be between 0 and 100.';
  if (sizeNum !== null && (Number.isNaN(sizeNum) || sizeNum < 0))
    errors.size_sqm = 'Size must be a non-negative number.';
  if (roomsNum !== null && (!Number.isInteger(roomsNum) || roomsNum < 0))
    errors.rooms = 'Rooms must be a non-negative whole number.';

  const isValid = Object.keys(errors).length === 0;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!isValid) return;
    onSubmit({
      designation: trimmedDesignation,
      entrance: entrance.trim() || undefined,
      floor: floorNum,
      unit_type: unitType,
      size_sqm: sizeNum !== null ? sizeSqm.trim() : undefined,
      rooms: roomsNum !== null ? roomsNum : undefined,
      ownership_share: ownershipShare.trim(),
      occupancy_status: occupancyStatus,
      description: description.trim() || undefined,
    });
  };

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <button
        type="button"
        className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
        onClick={onClose}
        onKeyDown={(e) => e.key === 'Escape' && onClose()}
        aria-label={t('aria.closeDialog')}
      />
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="relative w-full max-w-lg bg-white rounded-lg shadow-xl">
          <form
            onSubmit={handleSubmit}
            aria-label={mode === 'create' ? 'Create unit' : 'Edit unit'}
          >
            {/* Header */}
            <div className="flex items-center justify-between p-6 border-b">
              <h2 className="text-xl font-semibold text-gray-900">
                {mode === 'create' ? 'Add unit' : 'Edit unit'}
              </h2>
              <button type="button" onClick={onClose} className="text-gray-400 hover:text-gray-600">
                X
              </button>
            </div>

            {/* Content */}
            <div className="p-6 space-y-4 max-h-[60vh] overflow-y-auto">
              {error && (
                <div className="bg-red-50 border border-red-200 rounded-md p-3 text-sm text-red-700">
                  {error}
                </div>
              )}

              <div>
                <label
                  htmlFor="unit-designation"
                  className="block text-sm font-medium text-gray-700 mb-1"
                >
                  Designation *
                </label>
                <input
                  id="unit-designation"
                  type="text"
                  value={designation}
                  onChange={(e) => setDesignation(e.target.value)}
                  placeholder='e.g. "3B" or "101"'
                  className={inputClass}
                  maxLength={50}
                />
                {errors.designation && (
                  <p className="mt-1 text-sm text-red-600">{errors.designation}</p>
                )}
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label
                    htmlFor="unit-floor"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Floor
                  </label>
                  <input
                    id="unit-floor"
                    type="number"
                    value={floor}
                    onChange={(e) => setFloor(e.target.value)}
                    className={inputClass}
                  />
                  {errors.floor && <p className="mt-1 text-sm text-red-600">{errors.floor}</p>}
                </div>
                <div>
                  <label
                    htmlFor="unit-entrance"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Entrance
                  </label>
                  <input
                    id="unit-entrance"
                    type="text"
                    value={entrance}
                    onChange={(e) => setEntrance(e.target.value)}
                    placeholder="Optional"
                    className={inputClass}
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label
                    htmlFor="unit-type"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Type
                  </label>
                  <select
                    id="unit-type"
                    value={unitType}
                    onChange={(e) => setUnitType(e.target.value as UnitType)}
                    className={inputClass}
                  >
                    {UNIT_TYPE_OPTIONS.map((o) => (
                      <option key={o.value} value={o.value}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label
                    htmlFor="unit-occupancy"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Occupancy
                  </label>
                  <select
                    id="unit-occupancy"
                    value={occupancyStatus}
                    onChange={(e) => setOccupancyStatus(e.target.value as UnitOccupancyStatus)}
                    className={inputClass}
                  >
                    {OCCUPANCY_OPTIONS.map((o) => (
                      <option key={o.value} value={o.value}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-3 gap-4">
                <div>
                  <label
                    htmlFor="unit-size"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Size (m²)
                  </label>
                  <input
                    id="unit-size"
                    type="number"
                    step="0.01"
                    min="0"
                    value={sizeSqm}
                    onChange={(e) => setSizeSqm(e.target.value)}
                    placeholder="Optional"
                    className={inputClass}
                  />
                  {errors.size_sqm && (
                    <p className="mt-1 text-sm text-red-600">{errors.size_sqm}</p>
                  )}
                </div>
                <div>
                  <label
                    htmlFor="unit-rooms"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Rooms
                  </label>
                  <input
                    id="unit-rooms"
                    type="number"
                    min="0"
                    value={rooms}
                    onChange={(e) => setRooms(e.target.value)}
                    placeholder="Optional"
                    className={inputClass}
                  />
                  {errors.rooms && <p className="mt-1 text-sm text-red-600">{errors.rooms}</p>}
                </div>
                <div>
                  <label
                    htmlFor="unit-ownership"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Share (%) *
                  </label>
                  <input
                    id="unit-ownership"
                    type="number"
                    step="0.01"
                    min="0"
                    max="100"
                    value={ownershipShare}
                    onChange={(e) => setOwnershipShare(e.target.value)}
                    className={inputClass}
                  />
                  {errors.ownership_share && (
                    <p className="mt-1 text-sm text-red-600">{errors.ownership_share}</p>
                  )}
                </div>
              </div>

              <div>
                <label
                  htmlFor="unit-description"
                  className="block text-sm font-medium text-gray-700 mb-1"
                >
                  Description
                </label>
                <textarea
                  id="unit-description"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="Optional"
                  rows={2}
                  className={inputClass}
                />
              </div>
            </div>

            {/* Footer */}
            <div className="flex justify-end gap-3 p-6 border-t">
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={!isValid || isSubmitting}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
              >
                {isSubmitting ? 'Saving…' : mode === 'create' ? 'Add unit' : 'Save changes'}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}

UnitForm.displayName = 'UnitForm';
