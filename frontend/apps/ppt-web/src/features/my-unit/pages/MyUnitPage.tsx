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
        <p>Loading your unit…</p>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="my-unit-page">
        <p role="alert">
          Could not load your unit: {error instanceof Error ? error.message : 'Unknown error'}
        </p>
      </div>
    );
  }

  if (!units || units.length === 0) {
    return (
      <div className="my-unit-page">
        <h1>My Unit</h1>
        <p>You are not currently registered as a resident of any unit.</p>
        <p>If you believe this is a mistake, please contact your building manager.</p>
      </div>
    );
  }

  const selected = units.find((u) => u.unit.id === selectedId) ?? units[0];
  const { association, unit, building } = selected;

  return (
    <div className="my-unit-page">
      <header>
        <h1>My Unit</h1>
        <p>Your unit details and residency status.</p>
      </header>

      {units.length > 1 && (
        <div className="my-unit-page__switcher">
          <label htmlFor="my-unit-switcher">Choose a unit</label>
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
          Unit {unit.designation}
          {!unit.entrance ? null : ` · Entrance ${unit.entrance}`}
        </h2>
        <dl>
          <dt>Type</dt>
          <dd>{unit.unitType}</dd>
          <dt>Floor</dt>
          <dd>{unit.floorDisplay}</dd>
          {unit.rooms !== null && (
            <>
              <dt>Rooms</dt>
              <dd>{unit.rooms}</dd>
            </>
          )}
          {unit.sizeSqm !== null && (
            <>
              <dt>Size</dt>
              <dd>{unit.sizeSqm} m²</dd>
            </>
          )}
          <dt>Ownership share</dt>
          <dd>{unit.ownershipShare}</dd>
          <dt>Occupancy</dt>
          <dd>{unit.occupancyStatus}</dd>
          {unit.description && (
            <>
              <dt>Description</dt>
              <dd>{unit.description}</dd>
            </>
          )}
        </dl>
      </section>

      <section className="my-unit-page__building" aria-label={t('aria.building')}>
        <h2>Building</h2>
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
        <h2>Your residency</h2>
        <dl>
          <dt>Role</dt>
          <dd>{association.residentTypeDisplay}</dd>
          <dt>Primary contact</dt>
          <dd>{association.isPrimary ? 'Yes' : 'No'}</dd>
          <dt>Status</dt>
          <dd>{association.isActive ? 'Active' : 'Ended'}</dd>
          <dt>Since</dt>
          <dd>{association.startDate}</dd>
          {association.endDate && (
            <>
              <dt>Until</dt>
              <dd>{association.endDate}</dd>
            </>
          )}
        </dl>
      </section>

      <p className="my-unit-page__privacy-note">
        For your privacy, other residents and owners of this unit are not shown here.
      </p>
    </div>
  );
}
