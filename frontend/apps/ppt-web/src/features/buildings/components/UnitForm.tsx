/**
 * UnitForm - create / edit a unit within a building (Story 3.2).
 *
 * Rendered as a modal dialog. When `unit` is provided the form is in edit
 * mode (PUT), otherwise create mode (POST). Handles client-side validation
 * (required designation, numeric ranges) and surfaces server errors.
 */

import type {
  CreateUnitRequest,
  Unit,
  UnitOccupancyStatus,
  UnitType,
  UpdateUnitRequest,
} from '@ppt/api-client';
import { type FormEvent, useState } from 'react';
import { useUnitMutations } from '../hooks';

interface UnitFormProps {
  buildingId: string;
  /** When set, the form edits this unit; otherwise it creates a new one. */
  unit?: Unit;
  onClose: () => void;
  onSuccess?: () => void;
}

const UNIT_TYPE_OPTIONS: { value: UnitType; label: string }[] = [
  { value: 'apartment', label: 'Apartment' },
  { value: 'commercial', label: 'Commercial' },
  { value: 'parking', label: 'Parking' },
  { value: 'storage', label: 'Storage' },
  { value: 'other', label: 'Other' },
];

const OCCUPANCY_OPTIONS: { value: UnitOccupancyStatus; label: string }[] = [
  { value: 'vacant', label: 'Vacant' },
  { value: 'occupied', label: 'Occupied' },
  { value: 'reserved', label: 'Reserved' },
];

/** Parse an optional numeric input; empty string => undefined. */
function parseOptionalNumber(value: string): number | undefined {
  if (value.trim() === '') return undefined;
  const n = Number(value);
  return Number.isNaN(n) ? undefined : n;
}

export function UnitForm({ buildingId, unit, onClose, onSuccess }: UnitFormProps) {
  const isEdit = !!unit;
  const { useCreate, useUpdate } = useUnitMutations();
  const createMutation = useCreate();
  const updateMutation = useUpdate();

  const [designation, setDesignation] = useState(unit?.designation ?? '');
  const [entrance, setEntrance] = useState(unit?.entrance ?? '');
  const [floor, setFloor] = useState(String(unit?.floor ?? 0));
  const [unitType, setUnitType] = useState<UnitType>(unit?.unitType ?? 'apartment');
  const [sizeSqm, setSizeSqm] = useState(unit?.sizeSqm != null ? String(unit.sizeSqm) : '');
  const [rooms, setRooms] = useState(unit?.rooms != null ? String(unit.rooms) : '');
  const [ownershipShare, setOwnershipShare] = useState(
    unit?.ownershipShare != null ? String(unit.ownershipShare) : '100'
  );
  const [occupancyStatus, setOccupancyStatus] = useState<UnitOccupancyStatus>(
    unit?.occupancyStatus ?? 'vacant'
  );
  const [description, setDescription] = useState(unit?.description ?? '');
  const [notes, setNotes] = useState(unit?.notes ?? '');
  const [validationError, setValidationError] = useState<string | null>(null);

  const isSubmitting = createMutation.isPending || updateMutation.isPending;
  const serverError = createMutation.error ?? updateMutation.error;

  function validate(): string | null {
    if (designation.trim() === '') {
      return 'Designation is required.';
    }
    const share = parseOptionalNumber(ownershipShare);
    if (share !== undefined && (share < 0 || share > 100)) {
      return 'Ownership share must be between 0 and 100.';
    }
    const size = parseOptionalNumber(sizeSqm);
    if (size !== undefined && size < 0) {
      return 'Size must not be negative.';
    }
    const roomCount = parseOptionalNumber(rooms);
    if (roomCount !== undefined && roomCount < 0) {
      return 'Rooms must not be negative.';
    }
    return null;
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const error = validate();
    if (error) {
      setValidationError(error);
      return;
    }
    setValidationError(null);

    const floorValue = parseOptionalNumber(floor) ?? 0;

    try {
      if (isEdit && unit) {
        const data: UpdateUnitRequest = {
          designation: designation.trim(),
          entrance: entrance.trim() || undefined,
          floor: floorValue,
          unitType,
          sizeSqm: parseOptionalNumber(sizeSqm),
          rooms: parseOptionalNumber(rooms),
          ownershipShare: parseOptionalNumber(ownershipShare),
          occupancyStatus,
          description: description.trim() || undefined,
          notes: notes.trim() || undefined,
        };
        await updateMutation.mutateAsync({ buildingId, unitId: unit.id, data });
      } else {
        const data: CreateUnitRequest = {
          designation: designation.trim(),
          entrance: entrance.trim() || undefined,
          floor: floorValue,
          unitType,
          sizeSqm: parseOptionalNumber(sizeSqm),
          rooms: parseOptionalNumber(rooms),
          ownershipShare: parseOptionalNumber(ownershipShare),
          description: description.trim() || undefined,
        };
        await createMutation.mutateAsync({ buildingId, data });
      }
      onSuccess?.();
      onClose();
    } catch {
      // Error is surfaced via serverError below; keep the dialog open.
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      role="dialog"
      aria-modal="true"
      aria-label={isEdit ? 'Edit unit' : 'Add unit'}
    >
      <div className="bg-white rounded-lg shadow-xl w-full max-w-lg max-h-[90vh] overflow-y-auto">
        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          <h2 className="text-lg font-semibold text-gray-900">
            {isEdit ? 'Edit unit' : 'Add unit'}
          </h2>

          {(validationError || serverError) && (
            <div className="bg-red-50 border border-red-200 rounded-md p-3" role="alert">
              <p className="text-sm text-red-700">
                {validationError ??
                  (serverError instanceof Error ? serverError.message : 'Failed to save unit.')}
              </p>
            </div>
          )}

          <div>
            <label htmlFor="unit-designation" className="block text-sm font-medium text-gray-700">
              Designation <span className="text-red-600">*</span>
            </label>
            <input
              id="unit-designation"
              type="text"
              value={designation}
              onChange={(e) => setDesignation(e.target.value)}
              placeholder="e.g. 3B, 101"
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label htmlFor="unit-entrance" className="block text-sm font-medium text-gray-700">
                Entrance
              </label>
              <input
                id="unit-entrance"
                type="text"
                value={entrance}
                onChange={(e) => setEntrance(e.target.value)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
              />
            </div>
            <div>
              <label htmlFor="unit-floor" className="block text-sm font-medium text-gray-700">
                Floor
              </label>
              <input
                id="unit-floor"
                type="number"
                value={floor}
                onChange={(e) => setFloor(e.target.value)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label htmlFor="unit-type" className="block text-sm font-medium text-gray-700">
                Type
              </label>
              <select
                id="unit-type"
                value={unitType}
                onChange={(e) => setUnitType(e.target.value as UnitType)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
              >
                {UNIT_TYPE_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
            </div>
            {isEdit && (
              <div>
                <label htmlFor="unit-occupancy" className="block text-sm font-medium text-gray-700">
                  Occupancy
                </label>
                <select
                  id="unit-occupancy"
                  value={occupancyStatus}
                  onChange={(e) => setOccupancyStatus(e.target.value as UnitOccupancyStatus)}
                  className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
                >
                  {OCCUPANCY_OPTIONS.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </select>
              </div>
            )}
          </div>

          <div className="grid grid-cols-3 gap-4">
            <div>
              <label htmlFor="unit-size" className="block text-sm font-medium text-gray-700">
                Size (m2)
              </label>
              <input
                id="unit-size"
                type="number"
                min="0"
                step="0.01"
                value={sizeSqm}
                onChange={(e) => setSizeSqm(e.target.value)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
              />
            </div>
            <div>
              <label htmlFor="unit-rooms" className="block text-sm font-medium text-gray-700">
                Rooms
              </label>
              <input
                id="unit-rooms"
                type="number"
                min="0"
                value={rooms}
                onChange={(e) => setRooms(e.target.value)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
              />
            </div>
            <div>
              <label htmlFor="unit-share" className="block text-sm font-medium text-gray-700">
                Share (%)
              </label>
              <input
                id="unit-share"
                type="number"
                min="0"
                max="100"
                step="0.01"
                value={ownershipShare}
                onChange={(e) => setOwnershipShare(e.target.value)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
              />
            </div>
          </div>

          <div>
            <label htmlFor="unit-description" className="block text-sm font-medium text-gray-700">
              Description
            </label>
            <textarea
              id="unit-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={2}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
            />
          </div>

          {isEdit && (
            <div>
              <label htmlFor="unit-notes" className="block text-sm font-medium text-gray-700">
                Notes
              </label>
              <textarea
                id="unit-notes"
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                rows={2}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
              />
            </div>
          )}

          <div className="flex justify-end gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
            >
              {isSubmitting ? 'Saving…' : isEdit ? 'Save changes' : 'Add unit'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

UnitForm.displayName = 'UnitForm';
