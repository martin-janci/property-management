/**
 * Resident "My Unit" page (Epic 3, Story 3.6).
 *
 * Read-only, resident-scoped view of the unit(s) the signed-in user is a
 * resident of. Resolves associations from `GET /api/v1/users/me/units`, lets
 * the resident switch between multiple units, and shows their association
 * status alongside non-PII unit/building detail.
 *
 * Privacy: the API returns only the caller's own association and unit/building
 * detail — never other residents' or owners' identities. That filter is
 * enforced server-side; this page simply renders what it is given.
 *
 * @module features/my-unit/pages/MyUnitPage
 */

import { useMyUnits } from '@ppt/api-client';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

export function MyUnitPage() {
  const { t } = useTranslation();
  const { data: units, isLoading, isError, error } = useMyUnits();
  const [selectedId, setSelectedId] = useState<string | null>(null);

  // Default the switcher to the first unit once data arrives (or whenever the
  // current selection is no longer present in the returned set).
  useEffect(() => {
    if (!units || units.length === 0) {
      return;
    }
    const stillPresent = units.some((u) => u.unit.id === selectedId);
    if (!stillPresent) {
      setSelectedId(units[0].unit.id);
    }
  }, [units, selectedId]);

  if (isLoading) {
    return (
      <div className="my-unit-page">
        <p>{t('buildings.myUnit.loading')}</p>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="my-unit-page">
        <p role="alert">
          {t('buildings.myUnit.loadError', {
            message: error instanceof Error ? error.message : t('buildings.myUnit.unknownError'),
          })}
        </p>
      </div>
    );
  }

  if (!units || units.length === 0) {
    return (
      <div className="my-unit-page">
        <h1>{t('buildings.myUnit.title')}</h1>
        <p>{t('buildings.myUnit.emptyNotRegistered')}</p>
        <p>{t('buildings.myUnit.emptyContactManager')}</p>
      </div>
    );
  }

  const selected = units.find((u) => u.unit.id === selectedId) ?? units[0];
  const { association, unit, building } = selected;

  return (
    <div className="my-unit-page">
      <header>
        <h1>{t('buildings.myUnit.title')}</h1>
        <p>{t('buildings.myUnit.subtitle')}</p>
      </header>

      {units.length > 1 && (
        <div className="my-unit-page__switcher">
          <label htmlFor="my-unit-switcher">{t('buildings.myUnit.chooseUnit')}</label>
          <select
            id="my-unit-switcher"
            value={selected.unit.id}
            onChange={(e) => setSelectedId(e.target.value)}
          >
            {units.map((u) => (
              <option key={u.unit.id} value={u.unit.id}>
                {u.building.street} — {u.unit.designation}
              </option>
            ))}
          </select>
        </div>
      )}

      <section className="my-unit-page__unit" aria-label={t('aria.unitDetails')}>
        <h2>
          {t('buildings.myUnit.unitHeading', { designation: unit.designation })}
          {!unit.entrance
            ? null
            : ` · ${t('buildings.myUnit.entrance', { entrance: unit.entrance })}`}
        </h2>
        <dl>
          <dt>{t('buildings.unitForm.type')}</dt>
          <dd>{t(`buildings.unitType.${unit.unitType}`, { defaultValue: unit.unitType })}</dd>
          <dt>{t('buildings.unitForm.floor')}</dt>
          <dd>{unit.floorDisplay}</dd>
          {unit.rooms !== null && (
            <>
              <dt>{t('buildings.unitForm.rooms')}</dt>
              <dd>{unit.rooms}</dd>
            </>
          )}
          {unit.sizeSqm !== null && (
            <>
              <dt>{t('buildings.myUnit.size')}</dt>
              <dd>{unit.sizeSqm} m²</dd>
            </>
          )}
          <dt>{t('buildings.myUnit.ownershipShare')}</dt>
          <dd>{unit.ownershipShare}</dd>
          <dt>{t('buildings.unitForm.occupancy')}</dt>
          <dd>
            {t(`buildings.occupancy.${unit.occupancyStatus}`, {
              defaultValue: unit.occupancyStatus,
            })}
          </dd>
          {unit.description && (
            <>
              <dt>{t('buildings.unitForm.description')}</dt>
              <dd>{unit.description}</dd>
            </>
          )}
        </dl>
      </section>

      <section className="my-unit-page__building" aria-label={t('aria.building')}>
        <h2>{t('buildings.myUnit.building')}</h2>
        <address>
          {building.name && (
            <>
              {building.name}
              <br />
            </>
          )}
          {building.street}
          <br />
          {building.postalCode} {building.city}
        </address>
      </section>

      <section className="my-unit-page__association" aria-label={t('aria.yourResidency')}>
        <h2>{t('buildings.myUnit.residency')}</h2>
        <dl>
          <dt>{t('buildings.myUnit.role')}</dt>
          <dd>
            {t(`buildings.myUnit.residentType.${association.residentType}`, {
              defaultValue: association.residentTypeDisplay,
            })}
          </dd>
          <dt>{t('buildings.myUnit.primaryContact')}</dt>
          <dd>{association.isPrimary ? t('common.yes') : t('common.no')}</dd>
          <dt>{t('buildings.unitsSection.status')}</dt>
          <dd>
            {association.isActive ? t('buildings.myUnit.active') : t('buildings.myUnit.ended')}
          </dd>
          <dt>{t('buildings.myUnit.since')}</dt>
          <dd>{association.startDate}</dd>
          {association.endDate && (
            <>
              <dt>{t('buildings.myUnit.until')}</dt>
              <dd>{association.endDate}</dd>
            </>
          )}
        </dl>
      </section>

      <p className="my-unit-page__privacy-note">{t('buildings.myUnit.privacyNote')}</p>
    </div>
  );
}
