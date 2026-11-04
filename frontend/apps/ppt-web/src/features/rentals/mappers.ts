/**
 * Pure helpers for the rentals calendar and daňový výkaz (guest report) pages:
 * local-date arithmetic, API → UI mappers and the building/unit picker model.
 *
 * Dates on the rentals API are plain `YYYY-MM-DD` (`NaiveDate`). They are parsed
 * as *local* dates here — `new Date('2026-11-01')` would be UTC midnight and
 * land on the previous day west of Greenwich.
 */

import type {
  RentalBuildingListItem,
  RentalBuildingUnitItem,
  RentalCalendarEvent,
  RentalReportPreview,
  RentalReportSummary,
} from '@ppt/api-client';
import type {
  BookingSource,
  BookingStatus,
  CalendarBlockReason,
  CalendarEvent,
  GuestReportPeriodType,
  GuestReportPreviewData,
  GuestReportSummaryItem,
  RentalBuildingOption,
  RentalUnitOption,
} from './types';

// ---------------------------------------------------------------------------
// Local dates
// ---------------------------------------------------------------------------

/** `Date` → `YYYY-MM-DD` using the local calendar day. */
export function toIsoDate(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

/** `YYYY-MM-DD` (or an ISO timestamp) → local midnight of that calendar day. */
export function parseIsoDate(value: string): Date {
  const [y, m, d] = value.slice(0, 10).split('-').map(Number);
  return new Date(y, (m || 1) - 1, d || 1);
}

/** Shift an ISO date by whole days. */
export function addDaysIso(value: string, days: number): string {
  const date = parseIsoDate(value);
  date.setDate(date.getDate() + days);
  return toIsoDate(date);
}

/**
 * Days shown in a month grid that starts on Monday: leading days of the
 * previous month, the month itself, and trailing days to complete the week.
 */
export function getMonthGridDays(month: Date): Date[] {
  const year = month.getFullYear();
  const m = month.getMonth();
  const first = new Date(year, m, 1);
  const lead = (first.getDay() + 6) % 7; // Monday = 0
  const daysInMonth = new Date(year, m + 1, 0).getDate();
  const total = Math.ceil((lead + daysInMonth) / 7) * 7;
  return Array.from({ length: total }, (_, i) => new Date(year, m, 1 - lead + i));
}

/**
 * Half-open date range `[start, end)` covering the whole visible month grid,
 * as sent to `GET /rentals/calendar/{unit_id}`.
 */
export function calendarFetchRange(month: Date): { startDate: string; endDate: string } {
  const days = getMonthGridDays(month);
  const last = days[days.length - 1];
  return {
    startDate: toIsoDate(days[0]),
    endDate: toIsoDate(new Date(last.getFullYear(), last.getMonth(), last.getDate() + 1)),
  };
}

/**
 * Whether a calendar event occupies `day`. Events are half-open — a booking's
 * `endDate` is its check-out day, which is free for the next guest. A
 * zero-length event still shows on its single day.
 */
export function eventCoversDay(event: Pick<CalendarEvent, 'startDate' | 'endDate'>, day: Date) {
  const iso = toIsoDate(day);
  const start = event.startDate.slice(0, 10);
  const end = event.endDate.slice(0, 10);
  if (end <= start) return iso === start;
  return iso >= start && iso < end;
}

// ---------------------------------------------------------------------------
// Calendar events
// ---------------------------------------------------------------------------

const KNOWN_SOURCES: readonly BookingSource[] = ['airbnb', 'booking', 'direct'];
const KNOWN_STATUSES: readonly BookingStatus[] = [
  'pending',
  'confirmed',
  'checked_in',
  'checked_out',
  'cancelled',
  'no_show',
];

/**
 * The server sends only an English title for manual blocks ("Maintenance",
 * "Owner Use", "Blocked"); recover the reason so the UI can translate it.
 */
export function blockReasonFromTitle(title: string): CalendarBlockReason {
  const t = title.trim().toLowerCase();
  if (t === 'maintenance') return 'maintenance';
  if (t === 'owner use') return 'owner_use';
  return 'blocked';
}

/** `GET /rentals/calendar/{unit_id}` row → UI calendar event. */
export function mapCalendarEventToUi(ev: RentalCalendarEvent): CalendarEvent {
  const base = {
    id: ev.id,
    unitId: ev.unit_id,
    title: ev.title,
    startDate: ev.start_date,
    endDate: ev.end_date,
    color: ev.color,
  };
  if (ev.event_type === 'booking') {
    const platform = (ev.platform ?? '').toLowerCase();
    const status = ev.booking_status ?? undefined;
    return {
      ...base,
      type: 'booking',
      guestName: ev.title,
      source: platform
        ? KNOWN_SOURCES.includes(platform as BookingSource)
          ? (platform as BookingSource)
          : 'other'
        : undefined,
      status: KNOWN_STATUSES.includes(status as BookingStatus)
        ? (status as BookingStatus)
        : undefined,
    };
  }
  const blockReason = blockReasonFromTitle(ev.title);
  return {
    ...base,
    type: blockReason === 'maintenance' ? 'maintenance' : 'block',
    blockReason,
  };
}

// ---------------------------------------------------------------------------
// Building / unit pickers
// ---------------------------------------------------------------------------

/** Building display name; falls back to the address when the name is empty. */
export function buildingDisplayName(b: RentalBuildingListItem): string {
  const name = b.name?.trim();
  if (name) return name;
  return [b.street, b.city].filter(Boolean).join(', ');
}

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

/**
 * Build the picker model from `GET /buildings` + `GET /buildings/{id}/units`.
 * Buildings and units are sorted naturally (so "2" comes before "10").
 */
export function buildPropertyOptions(
  buildings: RentalBuildingListItem[],
  unitsByBuilding: Record<string, RentalBuildingUnitItem[]>
): { buildings: RentalBuildingOption[]; units: RentalUnitOption[] } {
  const buildingOptions = buildings
    .map((b) => ({ id: b.id, name: buildingDisplayName(b) }))
    .sort((a, b) => collator.compare(a.name, b.name));
  const units: RentalUnitOption[] = [];
  for (const building of buildingOptions) {
    const rows = [...(unitsByBuilding[building.id] ?? [])].sort((a, b) =>
      collator.compare(a.designation, b.designation)
    );
    for (const unit of rows) {
      units.push({
        id: unit.id,
        name: unit.designation,
        buildingId: building.id,
        buildingName: building.name,
      });
    }
  }
  return { buildings: buildingOptions, units };
}

// ---------------------------------------------------------------------------
// Daňový výkaz (authority guest report)
// ---------------------------------------------------------------------------

/**
 * Period bounds for a report. `index` is the month (1–12) for monthly, the
 * quarter (1–4) for quarterly, and ignored for annual. `periodEnd` is the last
 * day of the period (inclusive), matching how stored reports display it.
 */
export function computeReportPeriod(
  type: GuestReportPeriodType,
  year: number,
  index: number
): { periodStart: string; periodEnd: string } {
  let startMonth = 0;
  let months = 12;
  if (type === 'monthly') {
    startMonth = Math.min(Math.max(index, 1), 12) - 1;
    months = 1;
  } else if (type === 'quarterly') {
    startMonth = (Math.min(Math.max(index, 1), 4) - 1) * 3;
    months = 3;
  }
  return {
    periodStart: toIsoDate(new Date(year, startMonth, 1)),
    periodEnd: toIsoDate(new Date(year, startMonth + months, 0)),
  };
}

/** `ReportPreview` → UI; nationalities sorted by count, then name. */
export function mapReportPreviewToUi(p: RentalReportPreview): GuestReportPreviewData {
  return {
    buildingId: p.building_id,
    buildingName: p.building_name,
    periodStart: p.period_start,
    periodEnd: p.period_end,
    totalGuests: p.total_guests,
    bookingsCount: p.bookings_count,
    byNationality: [...p.by_nationality]
      .sort((a, b) => b.count - a.count || a.nationality.localeCompare(b.nationality))
      .map((row) => ({
        nationality: row.nationality,
        countryName: row.country_name,
        count: row.count,
        percentage: row.percentage,
      })),
  };
}

/** `ReportSummary` → UI. */
export function mapReportSummaryToUi(r: RentalReportSummary): GuestReportSummaryItem {
  return {
    id: r.id,
    buildingId: r.building_id,
    buildingName: r.building_name,
    periodType: r.report_type,
    periodStart: r.period_start,
    periodEnd: r.period_end,
    authorityCode: r.authority_code,
    authorityName: r.authority_name,
    totalGuests: r.total_guests,
    status: r.status,
    fileUrl: r.report_file_url ?? undefined,
    submittedAt: r.submitted_at ?? undefined,
  };
}

/**
 * Localised country name for an ISO code via `Intl.DisplayNames`; falls back
 * to the server-provided (English) name, and returns null for `UNK`.
 */
export function localizedCountryName(
  code: string,
  fallback: string,
  language: string
): string | null {
  if (!code || code.toUpperCase() === 'UNK') return null;
  try {
    const names = new Intl.DisplayNames([language], { type: 'region' });
    const name = names.of(code.toUpperCase());
    if (name && name.toUpperCase() !== code.toUpperCase()) return name;
  } catch {
    // Unsupported code or runtime without DisplayNames.
  }
  return fallback || code;
}
