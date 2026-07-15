/**
 * Built-in report types (issue #2324).
 *
 * There is deliberately no report-definitions table (issue #2198): `report_id`
 * is an opaque UUID and the reports this system generates — fault statistics,
 * voting participation, occupancy, consumption — are computed on demand from
 * live data, not stored as rows. The old "New Schedule" report selector was fed
 * by `useReports` → `GET /api/v1/reports/definitions`, a route that does not
 * exist, so the dropdown was always empty and a schedule could never be created.
 *
 * This fixed list replaces that dead endpoint. Each entry maps a built-in report
 * to a stable, well-known (non-nil) UUID that the backend stores verbatim as the
 * schedule's `report_id`. The scheduler treats `report_id` as opaque, so these
 * IDs need only be stable and non-nil — they are NOT foreign keys.
 */

export interface BuiltinReport {
  /** Stable, well-known, non-nil UUID persisted as the schedule's `report_id`. */
  id: string;
  /** i18n-independent display name shown in the report selector. */
  name: string;
  /** The backend report generator this schedule fires. */
  type: 'faults' | 'voting' | 'occupancy' | 'consumption';
}

export const BUILTIN_REPORTS: readonly BuiltinReport[] = [
  { id: 'a0000000-0000-4000-8000-000000000001', name: 'Fault statistics', type: 'faults' },
  { id: 'a0000000-0000-4000-8000-000000000002', name: 'Voting participation', type: 'voting' },
  { id: 'a0000000-0000-4000-8000-000000000003', name: 'Occupancy', type: 'occupancy' },
  { id: 'a0000000-0000-4000-8000-000000000004', name: 'Consumption', type: 'consumption' },
] as const;
