/**
 * UnitsSection - manage the units of a building from the web app (Story 3.2).
 *
 * Lists units with loading / error / empty states and lets a manager create,
 * edit, archive and restore units. Archiving is guarded by a confirmation
 * dialog. Mounted inside {@link BuildingDetailPage}.
 */

import type { UnitSummary, UnitType } from '@ppt/api-client';
import { useState } from 'react';
import { ConfirmationDialog } from '../../../components';
import { useUnit, useUnitMutations, useUnits } from '../hooks';
import { UnitForm } from './UnitForm';

interface UnitsSectionProps {
  buildingId: string;
}

const UNIT_TYPE_LABELS: Record<UnitType, string> = {
  apartment: 'Apartment',
  commercial: 'Commercial',
  parking: 'Parking',
  storage: 'Storage',
  other: 'Other',
};

function unitTypeLabel(type: UnitType): string {
  return UNIT_TYPE_LABELS[type] ?? type;
}

/**
 * Loads a full unit by id, then renders the edit form. Kept as its own
 * component so the `useUnit` query hook is always called unconditionally.
 */
function EditUnitLoader({
  buildingId,
  unitId,
  onClose,
}: {
  buildingId: string;
  unitId: string;
  onClose: () => void;
}) {
  const { data: unit, isLoading, error } = useUnit(buildingId, unitId);

  if (isLoading) {
    return (
      <div
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
        role="dialog"
        aria-modal="true"
        aria-label="Loading unit"
      >
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-white" />
      </div>
    );
  }

  if (error || !unit) {
    return (
      <ConfirmationDialog
        isOpen
        title="Could not load unit"
        message={error instanceof Error ? error.message : 'The unit could not be loaded.'}
        confirmLabel="Close"
        cancelLabel="Dismiss"
        onConfirm={onClose}
        onCancel={onClose}
      />
    );
  }

  return <UnitForm buildingId={buildingId} unit={unit} onClose={onClose} />;
}

export function UnitsSection({ buildingId }: UnitsSectionProps) {
  const [includeArchived, setIncludeArchived] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [editUnitId, setEditUnitId] = useState<string | null>(null);
  const [archiveTarget, setArchiveTarget] = useState<UnitSummary | null>(null);

  const { data, isLoading, error, refetch } = useUnits(buildingId, { includeArchived });

  const { useArchive, useRestore } = useUnitMutations();
  const archiveMutation = useArchive();
  const restoreMutation = useRestore();

  const units = data?.units ?? [];

  function confirmArchive() {
    if (!archiveTarget) return;
    archiveMutation.mutate(
      { buildingId, unitId: archiveTarget.id },
      { onSettled: () => setArchiveTarget(null) }
    );
  }

  function handleRestore(unitId: string) {
    restoreMutation.mutate({ buildingId, unitId });
  }

  return (
    <div className="bg-white rounded-lg shadow-md p-6 mb-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-gray-900">Units</h2>
        <div className="flex items-center gap-4">
          <label className="flex items-center gap-2 text-sm text-gray-600">
            <input
              type="checkbox"
              checked={includeArchived}
              onChange={(e) => setIncludeArchived(e.target.checked)}
              className="rounded border-gray-300"
            />
            Show archived
          </label>
          <button
            type="button"
            onClick={() => setShowCreate(true)}
            className="px-3 py-1.5 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
          >
            Add unit
          </button>
        </div>
      </div>

      {isLoading ? (
        <div className="flex justify-center py-4">
          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600" />
        </div>
      ) : error ? (
        <div className="bg-red-50 border border-red-200 rounded-md p-4" role="alert">
          <p className="text-sm text-red-700">
            {error instanceof Error ? error.message : 'Failed to load units.'}
          </p>
          <button
            type="button"
            onClick={() => refetch()}
            className="mt-2 text-sm text-blue-600 hover:text-blue-800"
          >
            Try again
          </button>
        </div>
      ) : units.length > 0 ? (
        <div className="space-y-2">
          {units.map((unit) => {
            const isArchived = unit.status === 'archived';
            return (
              <div
                key={unit.id}
                className="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
              >
                <div>
                  <span className="font-medium text-gray-900">{unit.designation}</span>
                  <span className="text-gray-500 text-sm ml-2">
                    {unitTypeLabel(unit.unitType)} · Floor {unit.floor} · {unit.occupancyStatus}
                  </span>
                  {isArchived && (
                    <span className="ml-2 px-2 py-0.5 rounded-full text-xs bg-gray-200 text-gray-600">
                      Archived
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-3">
                  {isArchived ? (
                    <button
                      type="button"
                      onClick={() => handleRestore(unit.id)}
                      disabled={restoreMutation.isPending}
                      className="text-sm text-blue-600 hover:text-blue-800 disabled:opacity-50"
                    >
                      Restore
                    </button>
                  ) : (
                    <>
                      <button
                        type="button"
                        onClick={() => setEditUnitId(unit.id)}
                        className="text-sm text-blue-600 hover:text-blue-800"
                      >
                        Edit
                      </button>
                      <button
                        type="button"
                        onClick={() => setArchiveTarget(unit)}
                        className="text-sm text-red-600 hover:text-red-800"
                      >
                        Archive
                      </button>
                    </>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <p className="text-gray-500 text-sm">No units defined for this building yet.</p>
      )}

      {showCreate && <UnitForm buildingId={buildingId} onClose={() => setShowCreate(false)} />}

      {editUnitId && (
        <EditUnitLoader
          buildingId={buildingId}
          unitId={editUnitId}
          onClose={() => setEditUnitId(null)}
        />
      )}

      <ConfirmationDialog
        isOpen={!!archiveTarget}
        title="Archive unit"
        message={
          archiveTarget
            ? `Archive unit "${archiveTarget.designation}"? It will be hidden from the active list but can be restored later.`
            : ''
        }
        confirmLabel="Archive"
        cancelLabel="Cancel"
        variant="danger"
        isLoading={archiveMutation.isPending}
        onConfirm={confirmArchive}
        onCancel={() => setArchiveTarget(null)}
      />
    </div>
  );
}

UnitsSection.displayName = 'UnitsSection';
