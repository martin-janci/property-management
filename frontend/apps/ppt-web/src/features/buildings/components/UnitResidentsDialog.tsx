/**
 * UnitResidentsDialog — owners / tenants assigned to one unit.
 *
 * Lists active residents (`GET …/units/{u}/residents`), adds an organization
 * member as owner / tenant / family member / subtenant, and ends a residency
 * (keeps history) via `POST …/residents/{id}/end`. A person must be a member
 * of the organization first (Members page) — the picker lists members only.
 */

import {
  ApiError,
  RESIDENT_TYPES,
  type ResidentType,
  useAddUnitResident,
  useEndUnitResidency,
  useOrgMembers,
  useUnitResidents,
} from '@ppt/api-client';
import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { useAuth } from '../../../contexts';

interface UnitResidentsDialogProps {
  buildingId: string;
  unitId: string;
  unitLabel: string;
  onClose: () => void;
}

const inputClass =
  'w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500';

/** Today as a local YYYY-MM-DD string. */
export function todayIso(now = new Date()): string {
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, '0');
  const d = String(now.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

export function UnitResidentsDialog({
  buildingId,
  unitId,
  unitLabel,
  onClose,
}: UnitResidentsDialogProps) {
  const { t } = useTranslation();
  const { user } = useAuth();
  const { data: residents = [], isLoading, error } = useUnitResidents(buildingId, unitId);
  const { data: membersData } = useOrgMembers(user?.organizationId);
  const addResident = useAddUnitResident(buildingId, unitId);
  const endResidency = useEndUnitResidency(buildingId, unitId);

  const [userId, setUserId] = useState('');
  const [residentType, setResidentType] = useState<ResidentType>('owner');
  const [isPrimary, setIsPrimary] = useState(false);
  const [formError, setFormError] = useState<string>();

  const active = residents.filter((r) => r.is_active);
  const activeIds = new Set(active.map((r) => r.user_id));
  const candidates = (membersData?.members ?? []).filter((m) => !activeIds.has(m.user_id));

  const handleAdd = (e: React.FormEvent) => {
    e.preventDefault();
    if (!userId) return;
    setFormError(undefined);
    addResident.mutate(
      { user_id: userId, resident_type: residentType, is_primary: isPrimary },
      {
        onSuccess: () => {
          setUserId('');
          setIsPrimary(false);
        },
        onError: (err) =>
          setFormError(
            err instanceof ApiError && err.status === 409
              ? t('buildings.residents.errors.duplicate')
              : t('buildings.residents.errors.addFailed')
          ),
      }
    );
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
        <div
          className="relative w-full max-w-2xl bg-white rounded-lg shadow-xl"
          role="dialog"
          aria-modal="true"
          aria-labelledby="unit-residents-title"
        >
          <div className="flex items-center justify-between p-6 border-b">
            <h2 id="unit-residents-title" className="text-xl font-semibold text-gray-900">
              {t('buildings.residents.title', { unit: unitLabel })}
            </h2>
            <button
              type="button"
              onClick={onClose}
              className="text-gray-400 hover:text-gray-600"
              aria-label={t('aria.closeDialog')}
            >
              ×
            </button>
          </div>

          <div className="p-6 space-y-6">
            {isLoading ? (
              <p className="text-gray-500" role="status">
                {t('common.loading')}
              </p>
            ) : error ? (
              <p className="text-red-700" role="alert">
                {t('buildings.residents.errors.loadFailed')}
              </p>
            ) : active.length === 0 ? (
              <p className="text-gray-500">{t('buildings.residents.empty')}</p>
            ) : (
              <ul className="divide-y divide-gray-100">
                {active.map((r) => (
                  <li key={r.id} className="py-2 flex items-center justify-between gap-4">
                    <div>
                      <p className="text-sm font-medium text-gray-900">
                        {r.user_name}
                        {r.is_primary && (
                          <span className="ml-2 text-xs text-blue-700">
                            {t('buildings.residents.primary')}
                          </span>
                        )}
                      </p>
                      <p className="text-xs text-gray-500">
                        {r.user_email} ·{' '}
                        {t(`buildings.residents.type.${r.resident_type}`, {
                          defaultValue: r.resident_type,
                        })}
                      </p>
                    </div>
                    <button
                      type="button"
                      disabled={endResidency.isPending}
                      onClick={() => endResidency.mutate({ residentId: r.id, endDate: todayIso() })}
                      className="text-sm text-red-600 hover:text-red-800 disabled:opacity-50"
                    >
                      {t('buildings.residents.end')}
                    </button>
                  </li>
                ))}
              </ul>
            )}

            <form onSubmit={handleAdd} className="border-t pt-4 space-y-3">
              <h3 className="text-sm font-semibold text-gray-900">
                {t('buildings.residents.addTitle')}
              </h3>
              {formError && (
                <p role="alert" className="text-sm text-red-700">
                  {formError}
                </p>
              )}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div>
                  <label htmlFor="resident-user" className="block text-sm text-gray-700 mb-1">
                    {t('buildings.residents.person')}
                  </label>
                  <select
                    id="resident-user"
                    className={inputClass}
                    value={userId}
                    onChange={(e) => setUserId(e.target.value)}
                  >
                    <option value="">{t('buildings.residents.choosePerson')}</option>
                    {candidates.map((m) => (
                      <option key={m.user_id} value={m.user_id}>
                        {m.name} ({m.email})
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label htmlFor="resident-type" className="block text-sm text-gray-700 mb-1">
                    {t('buildings.residents.typeLabel')}
                  </label>
                  <select
                    id="resident-type"
                    className={inputClass}
                    value={residentType}
                    onChange={(e) => setResidentType(e.target.value as ResidentType)}
                  >
                    {RESIDENT_TYPES.map((rt) => (
                      <option key={rt} value={rt}>
                        {t(`buildings.residents.type.${rt}`)}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
              <label className="flex items-center gap-2 text-sm text-gray-700">
                <input
                  type="checkbox"
                  checked={isPrimary}
                  onChange={(e) => setIsPrimary(e.target.checked)}
                  className="rounded border-gray-300"
                />
                {t('buildings.residents.primaryLabel')}
              </label>
              <div className="flex items-center justify-between">
                <Link to="/settings/members" className="text-sm text-blue-600 hover:text-blue-800">
                  {t('buildings.residents.notListed')}
                </Link>
                <button
                  type="submit"
                  disabled={!userId || addResident.isPending}
                  className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
                >
                  {addResident.isPending
                    ? t('buildings.residents.adding')
                    : t('buildings.residents.add')}
                </button>
              </div>
            </form>
          </div>
        </div>
      </div>
    </div>
  );
}
