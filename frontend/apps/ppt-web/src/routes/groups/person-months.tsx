/**
 * Person-months route group (Epic 3, Story 3.5 — person-month tracking).
 *
 * Mounts the already-built `features/person-months/` pages into ppt-web, wired
 * to the `@ppt/api-client` person-months module (BIT-190). This group owns the
 * API↔UI mappers (backend wire shapes are snake_case) and the thin route-wrapper
 * components.
 *
 * Person-months hang off a building (and, for the unit views, a unit), so every
 * route is nested under `/buildings/:buildingId`. The building- and unit-level
 * endpoints only return units that already have an entry, so the building-level
 * wrappers also list the building's units (`/api/v1/units?buildingId=`) and
 * left-join the counts onto them — units with no data show a count of 0.
 */
import type {
  PersonMonth as ApiPersonMonth,
  BulkPersonMonthUpsertResult,
  PersonMonthWithUnit,
  YearlyPersonMonthSummary,
} from '@ppt/api-client';
import {
  calculateFromResidents,
  personMonthKeys,
  useBuilding,
  useBuildingPersonMonths,
  useBulkUpsertPersonMonths,
  useDeletePersonMonth,
  useUnitPersonMonths,
  useUnitYearlySummary,
  useUpsertPersonMonth,
} from '@ppt/api-client';
import { useQuery } from '@tanstack/react-query';
import { lazy, useState } from 'react';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { ProtectedRoute, useToast } from '../../components';
import type { PersonMonth as UiPersonMonth } from '../../features/person-months/components/PersonMonthCard';
import type { YearlySummary as UiYearlySummary } from '../../features/person-months/components/YearlySummaryChart';
import type {
  BuildingPersonMonthSummary,
  UnitPersonMonthSummary,
} from '../../features/person-months/pages/PersonMonthsPage';
import type {
  BulkEntryFormData,
  BulkEntrySummary,
  BulkEntryUnit,
} from '../../features/person-months/types';
import { getApiClient } from '../../lib/api';

// Person-months feature pages (Epic 3) — lazy-loaded for code splitting.
const PersonMonthsPage = lazy(() =>
  import('../../features/person-months').then((m) => ({ default: m.PersonMonthsPage }))
);
const BulkEntryPage = lazy(() =>
  import('../../features/person-months').then((m) => ({ default: m.BulkEntryPage }))
);
const UnitPersonMonthsPage = lazy(() =>
  import('../../features/person-months').then((m) => ({ default: m.UnitPersonMonthsPage }))
);
const EditPersonMonthPage = lazy(() =>
  import('../../features/person-months').then((m) => ({ default: m.EditPersonMonthPage }))
);

const currentYear = new Date().getFullYear();
const currentMonth = new Date().getMonth() + 1;

// ============================================================================
// Building units (no person-months endpoint lists all units, so fetch them)
// ============================================================================

interface BuildingUnit {
  id: string;
  designation: string;
}

/**
 * Fetch every unit for a building, mapped to `{ id, designation }`.
 *
 * Routes through `getApiClient()` (path relative to the `/api/v1` baseURL) so
 * the shared axios interceptors apply — most importantly Bearer-token
 * injection. A raw `fetch()` here bypassed the JWT interceptor and 401'd for
 * real signed-in users, silently leaving the units dropdown empty.
 */
export function useBuildingUnits(buildingId: string | undefined) {
  const { data, isLoading } = useQuery({
    queryKey: ['person-months', 'building-units', buildingId ?? ''],
    enabled: !!buildingId,
    queryFn: async (): Promise<BuildingUnit[]> => {
      const res = await getApiClient().get<{
        data: Array<{ id: string; unitNumber: string }>;
      }>('/units', { params: { buildingId, limit: 500 } });
      return (res.data.data ?? []).map((u) => ({ id: u.id, designation: u.unitNumber }));
    },
  });
  return { units: data ?? [], isLoading };
}

// ============================================================================
// API → UI mappers
// ============================================================================

/** Map a wire person-month entry → the UI `PersonMonth` card shape. */
function mapApiPersonMonthToUi(pm: ApiPersonMonth, unitDesignation: string): UiPersonMonth {
  return {
    id: pm.id,
    unitId: pm.unit_id,
    unitDesignation,
    year: pm.year,
    month: pm.month,
    personCount: pm.count,
    updatedAt: pm.updated_at,
  };
}

/** Map a wire yearly summary → the UI `YearlySummary` chart shape. */
function mapYearlySummaryToUi(summary: YearlyPersonMonthSummary): UiYearlySummary {
  const monthsWithData = summary.months.filter((m) => m.count > 0);
  return {
    year: summary.year,
    totalPersonMonths: summary.total,
    averagePersons: monthsWithData.length > 0 ? summary.total / monthsWithData.length : 0,
    monthlyData: summary.months.map((m) => ({ month: m.month, personCount: m.count })),
  };
}

/** Build the building-level summary the `PersonMonthsPage` expects. */
function buildBuildingSummary(
  buildingId: string,
  buildingName: string,
  year: number,
  month: number,
  units: BuildingUnit[],
  entries: PersonMonthWithUnit[]
): BuildingPersonMonthSummary {
  const countByUnit = new Map(entries.map((e) => [e.unit_id, e.count]));
  const unitSummaries: UnitPersonMonthSummary[] = units.map((u) => ({
    unitId: u.id,
    unitDesignation: u.designation,
    personCount: countByUnit.get(u.id) ?? 0,
  }));
  const totalPersons = unitSummaries.reduce((sum, u) => sum + u.personCount, 0);
  return { buildingId, buildingName, year, month, totalPersons, units: unitSummaries };
}

// ============================================================================
// Route wrappers
// ============================================================================

/** Building-level person-months summary (`/buildings/:buildingId/person-months`). */
function PersonMonthsPageRoute() {
  const navigate = useNavigate();
  const { buildingId = '' } = useParams<{ buildingId: string }>();
  const [year, setYear] = useState(currentYear);
  const [month, setMonth] = useState(currentMonth);

  const buildingQuery = useBuilding(buildingId);
  const { units, isLoading: unitsLoading } = useBuildingUnits(buildingId);
  const entriesQuery = useBuildingPersonMonths(buildingId, { year, month });

  const buildingName = buildingQuery.data?.name ?? '';
  const summary = buildBuildingSummary(
    buildingId,
    buildingName,
    year,
    month,
    units,
    entriesQuery.data ?? []
  );

  return (
    <PersonMonthsPage
      buildingName={buildingName}
      summary={summary}
      isLoading={unitsLoading || entriesQuery.isLoading}
      onYearChange={setYear}
      onMonthChange={setMonth}
      onNavigateToUnit={(unitId) =>
        navigate(`/buildings/${buildingId}/units/${unitId}/person-months`)
      }
      onNavigateToEdit={(unitId, y, m) =>
        navigate(`/buildings/${buildingId}/units/${unitId}/person-months/edit/${y}/${m}`)
      }
      onNavigateToBulkEntry={() => navigate(`/buildings/${buildingId}/person-months/bulk`)}
      onBack={() => navigate(`/buildings/${buildingId}`)}
    />
  );
}

/** Bulk entry for every unit (`/buildings/:buildingId/person-months/bulk`). */
function BulkEntryPageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { buildingId = '' } = useParams<{ buildingId: string }>();

  const buildingQuery = useBuilding(buildingId);
  const { units } = useBuildingUnits(buildingId);
  const entriesQuery = useBuildingPersonMonths(buildingId, {
    year: currentYear,
    month: currentMonth,
  });
  const bulkUpsert = useBulkUpsertPersonMonths(buildingId);
  const [results, setResults] = useState<BulkEntrySummary | undefined>();

  const countByUnit = new Map((entriesQuery.data ?? []).map((e) => [e.unit_id, e.count]));
  const bulkUnits: BulkEntryUnit[] = units.map((u) => ({
    unitId: u.id,
    unitDesignation: u.designation,
    personCount: countByUnit.get(u.id) ?? 0,
    currentPersonCount: countByUnit.get(u.id),
  }));

  const mapResult = (
    res: BulkPersonMonthUpsertResult,
    submitted: BulkEntryFormData
  ): BulkEntrySummary => {
    const designationByUnit = new Map(units.map((u) => [u.id, u.designation]));
    return {
      total: submitted.entries.length,
      successful: res.successful,
      failed: res.failed,
      results: res.results.map((r) => ({
        unitId: r.unit_id,
        unitDesignation: designationByUnit.get(r.unit_id) ?? r.unit_id,
        success: r.success,
        error: r.error ?? undefined,
      })),
    };
  };

  return (
    <BulkEntryPage
      buildingName={buildingQuery.data?.name ?? ''}
      units={bulkUnits}
      initialYear={currentYear}
      initialMonth={currentMonth}
      isSubmitting={bulkUpsert.isPending}
      submitResults={results}
      onDismissResults={() => setResults(undefined)}
      onBack={() => navigate(`/buildings/${buildingId}/person-months`)}
      onSubmit={async (data: BulkEntryFormData) => {
        try {
          const res = await bulkUpsert.mutateAsync({
            year: data.year,
            month: data.month,
            entries: data.entries.map((e) => ({ unit_id: e.unitId, count: e.personCount })),
          });
          setResults(mapResult(res, data));
          showToast({
            type: res.failed > 0 ? 'warning' : 'success',
            title: 'Saved',
            message: `${res.successful} unit(s) updated`,
          });
        } catch (err) {
          showToast({
            type: 'error',
            title: 'Bulk entry failed',
            message: err instanceof Error ? err.message : 'Could not save person-months',
          });
        }
      }}
    />
  );
}

/** Unit history + yearly summary (`/buildings/:buildingId/units/:unitId/person-months`). */
function UnitPersonMonthsPageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { buildingId = '', unitId = '' } = useParams<{ buildingId: string; unitId: string }>();
  const [year, setYear] = useState(currentYear);

  const buildingQuery = useBuilding(buildingId);
  const { units } = useBuildingUnits(buildingId);
  const entriesQuery = useUnitPersonMonths(buildingId, unitId, { year });
  const summaryQuery = useUnitYearlySummary(buildingId, unitId, year);
  const deleteEntry = useDeletePersonMonth(buildingId, unitId);

  const unitDesignation = units.find((u) => u.id === unitId)?.designation ?? '';
  const personMonths = (entriesQuery.data ?? []).map((pm) =>
    mapApiPersonMonthToUi(pm, unitDesignation)
  );
  const yearlySummary: UiYearlySummary = summaryQuery.data
    ? mapYearlySummaryToUi(summaryQuery.data)
    : { year, totalPersonMonths: 0, averagePersons: 0, monthlyData: [] };

  return (
    <UnitPersonMonthsPage
      buildingName={buildingQuery.data?.name ?? ''}
      unitDesignation={unitDesignation}
      personMonths={personMonths}
      yearlySummary={yearlySummary}
      isLoading={entriesQuery.isLoading || summaryQuery.isLoading}
      isDeleting={deleteEntry.isPending}
      selectedYear={year}
      onYearChange={setYear}
      onNavigateToEdit={(y, m) =>
        navigate(`/buildings/${buildingId}/units/${unitId}/person-months/edit/${y}/${m}`)
      }
      onDelete={async (pm) => {
        try {
          await deleteEntry.mutateAsync(pm.id);
          showToast({ type: 'success', title: 'Deleted', message: 'Entry removed' });
        } catch (err) {
          showToast({
            type: 'error',
            title: 'Delete failed',
            message: err instanceof Error ? err.message : 'Could not delete entry',
          });
        }
      }}
      onBack={() => navigate(`/buildings/${buildingId}/person-months`)}
    />
  );
}

/**
 * Add / edit a single entry
 * (`/buildings/:buildingId/units/:unitId/person-months/edit/:year/:month`).
 *
 * When no entry exists for the period yet, the count is pre-populated from the
 * unit's resident history via the `calculate` endpoint (Story 3.5 auto-suggest).
 */
function EditPersonMonthPageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const {
    buildingId = '',
    unitId = '',
    year: yearParam,
    month: monthParam,
  } = useParams<{ buildingId: string; unitId: string; year: string; month: string }>();
  const year = Number.parseInt(yearParam ?? '', 10) || currentYear;
  const month = Number.parseInt(monthParam ?? '', 10) || currentMonth;

  const buildingQuery = useBuilding(buildingId);
  const { units } = useBuildingUnits(buildingId);
  const existingQuery = useUnitPersonMonths(buildingId, unitId, { year, month });
  const upsert = useUpsertPersonMonth(buildingId, unitId);

  const existing = existingQuery.data?.[0];
  // Auto-suggest a count from residents only when there is no entry yet.
  const suggestionQuery = useQuery({
    queryKey: [...personMonthKeys.unit(buildingId, unitId), 'calculate', year, month],
    enabled: !!buildingId && !!unitId && existingQuery.isSuccess && !existing,
    queryFn: () => calculateFromResidents(buildingId, unitId, { year, month }),
  });

  const unitDesignation = units.find((u) => u.id === unitId)?.designation ?? '';
  const isLoading =
    existingQuery.isLoading || (existingQuery.isSuccess && !existing && suggestionQuery.isLoading);

  const personCount = existing?.count ?? suggestionQuery.data;
  const backToUnit = () => navigate(`/buildings/${buildingId}/units/${unitId}/person-months`);

  if (isLoading) {
    return (
      <div className="mx-auto max-w-md py-16 text-center">
        <span className="animate-spin inline-block rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <EditPersonMonthPage
      buildingName={buildingQuery.data?.name ?? ''}
      unitDesignation={unitDesignation}
      initialData={{ year, month, personCount }}
      isSubmitting={upsert.isPending}
      onCancel={backToUnit}
      onSubmit={async (data) => {
        try {
          await upsert.mutateAsync({
            year: data.year,
            month: data.month,
            count: data.personCount,
            source: 'manual',
          });
          showToast({ type: 'success', title: 'Saved', message: 'Person-month saved' });
          backToUnit();
        } catch (err) {
          showToast({
            type: 'error',
            title: 'Save failed',
            message: err instanceof Error ? err.message : 'Could not save person-month',
          });
        }
      }}
    />
  );
}

/** Person-months routes (Epic 3, Story 3.5). */
export function personMonthRoutes() {
  return (
    <>
      <Route
        path="/buildings/:buildingId/person-months"
        element={
          <ProtectedRoute>
            <PersonMonthsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/buildings/:buildingId/person-months/bulk"
        element={
          <ProtectedRoute>
            <BulkEntryPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/buildings/:buildingId/units/:unitId/person-months"
        element={
          <ProtectedRoute>
            <UnitPersonMonthsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/buildings/:buildingId/units/:unitId/person-months/edit/:year/:month"
        element={
          <ProtectedRoute>
            <EditPersonMonthPageRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
