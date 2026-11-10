/**
 * UnitsSection - lists and manages the units within a building.
 *
 * Story 3.2: managers can list, create, edit, and archive units from the web
 * app. Handles load/error/empty states and an archive confirmation. Data access
 * goes through the buildings feature hooks; {@link UnitForm} is presentational.
 */

import type { UnitSummary, UpdateUnitRequest } from '@ppt/api-client';
import type { TFunction } from 'i18next';
import { useState } from 'react';
import { Trans, useTranslation } from 'react-i18next';
import { useUnit, useUnitMutations, useUnits } from '../hooks';
import { UnitForm, type UnitFormValues } from './UnitForm';
import { UnitResidentsDialog } from './UnitResidentsDialog';

interface UnitsSectionProps {
  buildingId: string;
}

/** Localized mirror of the backend `Unit::floor_display` helper. */
function floorLabel(floor: number, t: TFunction): string {
  if (floor < 0) return t('buildings.unitsSection.floorBasement', { level: Math.abs(floor) });
  if (floor === 0) return t('buildings.unitsSection.floorGround');
  return String(floor);
}

function errorMessage(error: unknown, t: TFunction): string | null {
  if (!error) return null;
  return error instanceof Error ? error.message : t('buildings.errors.generic');
}

export function UnitsSection({ buildingId }: UnitsSectionProps) {
  const { t } = useTranslation();
  const [showArchived, setShowArchived] = useState(false);
  const [dialogMode, setDialogMode] = useState<'create' | 'edit' | null>(null);
  const [editingUnitId, setEditingUnitId] = useState<string | null>(null);
  const [unitToArchive, setUnitToArchive] = useState<UnitSummary | null>(null);
  const [residentsUnit, setResidentsUnit] = useState<UnitSummary | null>(null);

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
        <h2 className="text-lg font-semibold text-gray-900">{t('buildings.units')}</h2>
        <div className="flex items-center gap-4">
          <label className="flex items-center gap-2 text-sm text-gray-600 cursor-pointer">
            <input
              type="checkbox"
              checked={showArchived}
              onChange={(e) => setShowArchived(e.target.checked)}
              className="rounded border-gray-300"
            />
            {t('buildings.unitsSection.showArchived')}
          </label>
          <button
            type="button"
            onClick={() => {
              setDialogMode('create');
              setEditingUnitId(null);
            }}
            className="px-3 py-1.5 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700"
          >
            {t('buildings.unitForm.addTitle')}
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
          <h3 className="text-sm font-medium text-red-800">
            {t('buildings.unitsSection.loadFailed')}
          </h3>
          <p className="mt-1 text-sm text-red-700">
            {errorMessage(loadError, t) ?? t('buildings.unitsSection.loadFailedBody')}
          </p>
        </div>
      ) : units.length === 0 ? (
        <p className="text-gray-500 text-sm">{t('buildings.unitsSection.empty')}</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="min-w-full text-sm">
            <thead>
              <tr className="text-left text-gray-500 border-b">
                <th className="py-2 pr-4 font-medium">{t('buildings.unitsSection.unit')}</th>
                <th className="py-2 pr-4 font-medium">{t('buildings.unitForm.floor')}</th>
                <th className="py-2 pr-4 font-medium">{t('buildings.unitForm.type')}</th>
                <th className="py-2 pr-4 font-medium">{t('buildings.unitForm.occupancy')}</th>
                <th className="py-2 pr-4 font-medium">{t('buildings.unitsSection.status')}</th>
                <th className="py-2 pr-4 font-medium text-right">{t('common.actions')}</th>
              </tr>
            </thead>
            <tbody>
              {units.map((unit) => (
                <tr key={unit.id} className="border-b last:border-0">
                  <td className="py-2 pr-4 font-medium text-gray-900">{unit.designation}</td>
                  <td className="py-2 pr-4 text-gray-600">{floorLabel(unit.floor, t)}</td>
                  <td className="py-2 pr-4 text-gray-600">
                    {t(`buildings.unitType.${unit.unit_type}`, { defaultValue: unit.unit_type })}
                  </td>
                  <td className="py-2 pr-4 text-gray-600">
                    {t(`buildings.occupancy.${unit.occupancy_status}`, {
                      defaultValue: unit.occupancy_status,
                    })}
                  </td>
                  <td className="py-2 pr-4">
                    <span
                      className={`px-2 py-0.5 rounded-full text-xs font-medium ${
                        unit.status === 'archived'
                          ? 'bg-gray-100 text-gray-600'
                          : 'bg-green-100 text-green-800'
                      }`}
                    >
                      {unit.status === 'archived'
                        ? t('buildings.unitsSection.archived')
                        : t('buildings.unitsSection.active')}
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
                        {t('buildings.unitsSection.restore')}
                      </button>
                    ) : (
                      <>
                        <button
                          type="button"
                          onClick={() => setResidentsUnit(unit)}
                          className="mr-4 text-blue-600 hover:text-blue-800"
                        >
                          {t('buildings.residents.manage')}
                        </button>
                        <button
                          type="button"
                          onClick={() => {
                            setDialogMode('edit');
                            setEditingUnitId(unit.id);
                          }}
                          className="text-blue-600 hover:text-blue-800"
                        >
                          {t('common.edit')}
                        </button>
                        <button
                          type="button"
                          onClick={() => setUnitToArchive(unit)}
                          className="ml-4 text-red-600 hover:text-red-800"
                        >
                          {t('buildings.unitsSection.archive')}
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

      {residentsUnit && (
        <UnitResidentsDialog
          buildingId={buildingId}
          unitId={residentsUnit.id}
          unitLabel={residentsUnit.designation}
          onClose={() => setResidentsUnit(null)}
        />
      )}

      {/* Create dialog */}
      {dialogMode === 'create' && (
        <UnitForm
          key="create"
          isOpen
          mode="create"
          isSubmitting={createUnit.isPending}
          error={errorMessage(createUnit.error, t)}
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
            error={errorMessage(updateUnit.error, t)}
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
              <h3 className="text-lg font-semibold text-gray-900">
                {t('buildings.unitsSection.archiveTitle')}
              </h3>
              <p className="mt-2 text-sm text-gray-600">
                <Trans
                  i18nKey="buildings.unitsSection.archiveConfirm"
                  values={{ designation: unitToArchive.designation }}
                  components={{ bold: <span className="font-medium" /> }}
                />
              </p>
              {archiveUnit.error && (
                <p className="mt-3 text-sm text-red-600">{errorMessage(archiveUnit.error, t)}</p>
              )}
              <div className="mt-6 flex justify-end gap-3">
                <button
                  type="button"
                  onClick={() => setUnitToArchive(null)}
                  className="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
                >
                  {t('common.cancel')}
                </button>
                <button
                  type="button"
                  onClick={confirmArchive}
                  disabled={archiveUnit.isPending}
                  className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
                >
                  {archiveUnit.isPending
                    ? t('buildings.unitsSection.archiving')
                    : t('buildings.unitsSection.archive')}
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
