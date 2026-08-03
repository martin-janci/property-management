/**
 * UnitsSection - lists and manages the units within a building.
 *
 * Story 3.2: managers can list, create, edit, and archive units from the web
 * app. Handles load/error/empty states and an archive confirmation. Data access
 * goes through the buildings feature hooks; {@link UnitForm} is presentational.
 */

import type {
  UnitOccupancyStatus,
  UnitSummary,
  UnitType,
  UpdateUnitRequest,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useUnit, useUnitMutations, useUnits } from '../hooks';
import { OCCUPANCY_OPTIONS, UNIT_TYPE_OPTIONS, UnitForm, type UnitFormValues } from './UnitForm';

interface UnitsSectionProps {
  buildingId: string;
}

const UNIT_TYPE_LABELS: Record<UnitType, string> = Object.fromEntries(
  UNIT_TYPE_OPTIONS.map((o) => [o.value, o.label])
) as Record<UnitType, string>;

const OCCUPANCY_LABELS: Record<UnitOccupancyStatus, string> = Object.fromEntries(
  OCCUPANCY_OPTIONS.map((o) => [o.value, o.label])
) as Record<UnitOccupancyStatus, string>;

/** Mirror of the backend `Unit::floor_display` helper. */
function floorLabel(floor: number): string {
  if (floor < 0) return `B${Math.abs(floor)}`;
  if (floor === 0) return 'G';
  return String(floor);
}

function errorMessage(error: unknown): string | null {
  if (!error) return null;
  return error instanceof Error ? error.message : 'Something went wrong. Please try again.';
}

export function UnitsSection({ buildingId }: UnitsSectionProps) {
  const { t } = useTranslation();
  const [showArchived, setShowArchived] = useState(false);
  const [dialogMode, setDialogMode] = useState<'create' | 'edit' | null>(null);
  const [editingUnitId, setEditingUnitId] = useState<string | null>(null);
  const [unitToArchive, setUnitToArchive] = useState<UnitSummary | null>(null);

  const {
    data: unitsResponse,
    isLoading,
    error: loadError,
  } = useUnits(buildingId, { include_archived: showArchived });

  const { data: editingUnit, isLoading: editingLoading } = useUnit(
    buildingId,
    editingUnitId ?? undefined,
    dialogMode === 'edit'
  );

  const { useCreateUnit, useUpdateUnit, useArchiveUnit, useRestoreUnit } = useUnitMutations();
  const createUnit = useCreateUnit();
  const updateUnit = useUpdateUnit();
  const archiveUnit = useArchiveUnit();
  const restoreUnit = useRestoreUnit();

  const units = unitsResponse?.units ?? [];

  const closeDialog = () => {
    setDialogMode(null);
    setEditingUnitId(null);
    createUnit.reset();
    updateUnit.reset();
  };

  const handleCreate = (values: UnitFormValues) => {
    createUnit.mutate(
      {
        buildingId,
        data: {
          designation: values.designation,
          entrance: values.entrance,
          floor: values.floor,
          unit_type: values.unit_type,
          size_sqm: values.size_sqm,
          rooms: values.rooms,
          ownership_share: values.ownership_share,
          description: values.description,
        },
      },
      { onSuccess: closeDialog }
    );
  };

  const handleUpdate = (values: UnitFormValues) => {
    if (!editingUnitId) return;
    const data: UpdateUnitRequest = {
      designation: values.designation,
      entrance: values.entrance,
      floor: values.floor,
      unit_type: values.unit_type,
      size_sqm: values.size_sqm,
      rooms: values.rooms,
      ownership_share: values.ownership_share,
      occupancy_status: values.occupancy_status,
      description: values.description,
    };
    updateUnit.mutate({ buildingId, unitId: editingUnitId, data }, { onSuccess: closeDialog });
  };

  const confirmArchive = () => {
    if (!unitToArchive) return;
    archiveUnit.mutate(
      { buildingId, unitId: unitToArchive.id },
      { onSuccess: () => setUnitToArchive(null) }
    );
  };

  return (
    <div className="bg-white rounded-lg shadow-md p-6 mb-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-gray-900">Units</h2>
        <div className="flex items-center gap-4">
          <label className="flex items-center gap-2 text-sm text-gray-600 cursor-pointer">
            <input
              type="checkbox"
              checked={showArchived}
              onChange={(e) => setShowArchived(e.target.checked)}
              className="rounded border-gray-300"
            />
            Show archived
          </label>
          <button
            type="button"
            onClick={() => {
              setDialogMode('create');
              setEditingUnitId(null);
            }}
            className="px-3 py-1.5 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700"
          >
            Add unit
          </button>
        </div>
      </div>

      {/* Body */}
      {isLoading ? (
        <div className="flex justify-center py-6">
          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600" />
        </div>
      ) : loadError ? (
        <div className="bg-red-50 border border-red-200 rounded-md p-4">
          <h3 className="text-sm font-medium text-red-800">Could not load units</h3>
          <p className="mt-1 text-sm text-red-700">
            {errorMessage(loadError) ?? 'The units could not be loaded.'}
          </p>
        </div>
      ) : units.length === 0 ? (
        <p className="text-gray-500 text-sm">
          No units defined for this building yet. Use “Add unit” to create one.
        </p>
      ) : (
        <div className="overflow-x-auto">
          <table className="min-w-full text-sm">
            <thead>
              <tr className="text-left text-gray-500 border-b">
                <th className="py-2 pr-4 font-medium">Unit</th>
                <th className="py-2 pr-4 font-medium">Floor</th>
                <th className="py-2 pr-4 font-medium">Type</th>
                <th className="py-2 pr-4 font-medium">Occupancy</th>
                <th className="py-2 pr-4 font-medium">Status</th>
                <th className="py-2 pr-4 font-medium text-right">Actions</th>
              </tr>
            </thead>
            <tbody>
              {units.map((unit) => (
                <tr key={unit.id} className="border-b last:border-0">
                  <td className="py-2 pr-4 font-medium text-gray-900">{unit.designation}</td>
                  <td className="py-2 pr-4 text-gray-600">{floorLabel(unit.floor)}</td>
                  <td className="py-2 pr-4 text-gray-600">
                    {UNIT_TYPE_LABELS[unit.unit_type] ?? unit.unit_type}
                  </td>
                  <td className="py-2 pr-4 text-gray-600">
                    {OCCUPANCY_LABELS[unit.occupancy_status] ?? unit.occupancy_status}
                  </td>
                  <td className="py-2 pr-4">
                    <span
                      className={`px-2 py-0.5 rounded-full text-xs font-medium ${
                        unit.status === 'archived'
                          ? 'bg-gray-100 text-gray-600'
                          : 'bg-green-100 text-green-800'
                      }`}
                    >
                      {unit.status === 'archived' ? 'Archived' : 'Active'}
                    </span>
                  </td>
                  <td className="py-2 pr-4 text-right whitespace-nowrap">
                    {unit.status === 'archived' ? (
                      <button
                        type="button"
                        onClick={() => restoreUnit.mutate({ buildingId, unitId: unit.id })}
                        disabled={restoreUnit.isPending}
                        className="text-blue-600 hover:text-blue-800 disabled:opacity-50"
                      >
                        Restore
                      </button>
                    ) : (
                      <>
                        <button
                          type="button"
                          onClick={() => {
                            setDialogMode('edit');
                            setEditingUnitId(unit.id);
                          }}
                          className="text-blue-600 hover:text-blue-800"
                        >
                          Edit
                        </button>
                        <button
                          type="button"
                          onClick={() => setUnitToArchive(unit)}
                          className="ml-4 text-red-600 hover:text-red-800"
                        >
                          Archive
                        </button>
                      </>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Create dialog */}
      {dialogMode === 'create' && (
        <UnitForm
          key="create"
          isOpen
          mode="create"
          isSubmitting={createUnit.isPending}
          error={errorMessage(createUnit.error)}
          onSubmit={handleCreate}
          onClose={closeDialog}
        />
      )}

      {/* Edit dialog — mounted only once the full unit has loaded so its initial
          field values are populated. */}
      {dialogMode === 'edit' &&
        (editingLoading || !editingUnit ? (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-white" />
          </div>
        ) : (
          <UnitForm
            key={editingUnit.id}
            isOpen
            mode="edit"
            initial={{
              designation: editingUnit.designation,
              entrance: editingUnit.entrance ?? undefined,
              floor: editingUnit.floor,
              unit_type: editingUnit.unit_type,
              size_sqm: editingUnit.size_sqm ?? undefined,
              rooms: editingUnit.rooms ?? undefined,
              ownership_share: editingUnit.ownership_share,
              occupancy_status: editingUnit.occupancy_status,
              description: editingUnit.description ?? undefined,
            }}
            isSubmitting={updateUnit.isPending}
            error={errorMessage(updateUnit.error)}
            onSubmit={handleUpdate}
            onClose={closeDialog}
          />
        ))}

      {/* Archive confirmation */}
      {unitToArchive && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <button
            type="button"
            className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
            onClick={() => setUnitToArchive(null)}
            aria-label={t('aria.cancel')}
          />
          <div className="flex min-h-full items-center justify-center p-4">
            <div className="relative w-full max-w-md bg-white rounded-lg shadow-xl p-6">
              <h3 className="text-lg font-semibold text-gray-900">Archive unit</h3>
              <p className="mt-2 text-sm text-gray-600">
                Archive unit <span className="font-medium">{unitToArchive.designation}</span>? It
                will be hidden from the active list but can be restored later.
              </p>
              {archiveUnit.error && (
                <p className="mt-3 text-sm text-red-600">{errorMessage(archiveUnit.error)}</p>
              )}
              <div className="mt-6 flex justify-end gap-3">
                <button
                  type="button"
                  onClick={() => setUnitToArchive(null)}
                  className="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={confirmArchive}
                  disabled={archiveUnit.isPending}
                  className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
                >
                  {archiveUnit.isPending ? 'Archiving…' : 'Archive'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

UnitsSection.displayName = 'UnitsSection';
