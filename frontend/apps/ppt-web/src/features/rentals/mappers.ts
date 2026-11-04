/**
 * Pure helpers for the rentals calendar and daňový výkaz (guest report) pages:
 * local-date arithmetic, API → UI mappers and the building/unit picker model.
 *
 * Dates on the rentals API are plain `YYYY-MM-DD` (`NaiveDate`). They are parsed
 * as *local* dates here — `new Date('2026-11-01')` would be UTC midnight and
 * land on the previous day west of Greenwich.
 */

import type {
  RentalBookingRecord,
  RentalBuildingListItem,
  RentalBuildingUnitItem,
  RentalCalendarEvent,
  RentalConnectionDetail,
  RentalCreateBookingBody,
  RentalReportPreview,
  RentalReportSummary,
  RentalUpdateBookingBody,
  RentalUpdateConnectionBody,
} from '@ppt/api-client';
import type {
  BookingFormValues,
  BookingSource,
  BookingStatus,
  CalendarBlockReason,
  CalendarEvent,
  ConnectionSettings,
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

// ---------------------------------------------------------------------------
// Platform connection settings
// ---------------------------------------------------------------------------

/** Sync intervals offered in the connection settings dialog (minutes). */
export const SYNC_INTERVAL_OPTIONS = [15, 30, 60, 180, 360, 720, 1440] as const;

/** `GET /rentals/connections/{id}` → the settings form model. */
export function mapConnectionDetailToSettings(d: RentalConnectionDetail): ConnectionSettings {
  return {
    externalPropertyId: d.external_property_id ?? '',
    externalListingUrl: d.external_listing_url ?? '',
    syncCalendar: d.sync_calendar,
    syncIntervalMinutes: d.sync_interval_minutes,
    blockOtherPlatforms: d.block_other_platforms,
  };
}

/**
 * Settings form → `PUT /rentals/connections/{id}` body. Text fields are
 * trimmed; an empty field that was empty before is omitted (the server keeps
 * the stored value for omitted fields, so there is nothing to send).
 */
export function mapConnectionSettingsToApi(
  next: ConnectionSettings,
  previous?: ConnectionSettings
): RentalUpdateConnectionBody {
  const body: RentalUpdateConnectionBody = {
    sync_calendar: next.syncCalendar,
    sync_interval_minutes: next.syncIntervalMinutes,
    block_other_platforms: next.blockOtherPlatforms,
  };
  const propertyId = next.externalPropertyId.trim();
  if (propertyId || previous?.externalPropertyId) body.external_property_id = propertyId;
  const listingUrl = next.externalListingUrl.trim();
  if (listingUrl || previous?.externalListingUrl) body.external_listing_url = listingUrl;
  return body;
}

/** True for an empty value or an absolute http(s) URL. */
export function isValidListingUrl(value: string): boolean {
  const v = value.trim();
  if (!v) return true;
  try {
    const url = new URL(v);
    return url.protocol === 'http:' || url.protocol === 'https:';
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Booking create / edit form
// ---------------------------------------------------------------------------

/** Empty create-booking form (direct booking, one guest, EUR). */
export function emptyBookingForm(unitId = ''): BookingFormValues {
  return {
    unitId,
    source: 'direct',
    guestName: '',
    guestEmail: '',
    guestPhone: '',
    guestCount: 1,
    checkIn: '',
    checkOut: '',
    totalPrice: '',
    currency: 'EUR',
    guestNotes: '',
    internalNotes: '',
  };
}

/** A full booking record → the edit form. */
export function bookingRecordToForm(rec: RentalBookingRecord): BookingFormValues {
  const platform = rec.platform;
  return {
    unitId: rec.unit_id,
    source:
      platform === 'airbnb' || platform === 'booking' || platform === 'direct' ? platform : 'other',
    guestName: rec.guest_name,
    guestEmail: rec.guest_email ?? '',
    guestPhone: rec.guest_phone ?? '',
    guestCount: rec.guest_count,
    checkIn: rec.check_in.slice(0, 10),
    checkOut: rec.check_out.slice(0, 10),
    totalPrice: rec.total_amount ?? '',
    currency: rec.currency ?? 'EUR',
    guestNotes: rec.guest_notes ?? '',
    internalNotes: rec.internal_notes ?? '',
  };
}

/** Field-level problems of a booking form; empty when it can be submitted. */
export type BookingFormError =
  | 'unit'
  | 'guestName'
  | 'dates'
  | 'dateOrder'
  | 'guestCount'
  | 'price';

export function validateBookingForm(v: BookingFormValues): BookingFormError[] {
  const errors: BookingFormError[] = [];
  if (!v.unitId) errors.push('unit');
  if (!v.guestName.trim()) errors.push('guestName');
  if (!v.checkIn || !v.checkOut) errors.push('dates');
  else if (v.checkOut <= v.checkIn) errors.push('dateOrder');
  if (!Number.isInteger(v.guestCount) || v.guestCount < 1) errors.push('guestCount');
  const price = v.totalPrice.trim().replace(',', '.');
  if (price && !(Number.isFinite(Number(price)) && Number(price) >= 0)) errors.push('price');
  return errors;
}

function optionalText(value: string): string | undefined {
  const v = value.trim();
  return v ? v : undefined;
}

function normalizedAmount(value: string): string | undefined {
  const v = value.trim().replace(',', '.');
  return v ? v : undefined;
}

/** Booking form → `POST /rentals/bookings` body. */
export function bookingFormToCreateBody(v: BookingFormValues): RentalCreateBookingBody {
  return {
    unit_id: v.unitId,
    // The server enum has no 'other'; a manual entry is a direct booking.
    platform: v.source === 'other' ? 'direct' : v.source,
    guest_name: v.guestName.trim(),
    guest_email: optionalText(v.guestEmail),
    guest_phone: optionalText(v.guestPhone),
    guest_count: v.guestCount,
    check_in: v.checkIn,
    check_out: v.checkOut,
    total_amount: normalizedAmount(v.totalPrice),
    currency: normalizedAmount(v.totalPrice) ? optionalText(v.currency) : undefined,
    guest_notes: optionalText(v.guestNotes),
    internal_notes: optionalText(v.internalNotes),
  };
}

/**
 * Edit form → `PUT /rentals/bookings/{id}` body. The server keeps the stored
 * value for omitted fields, so cleared optional text is sent as `""` only when
 * it had a value before.
 */
export function bookingFormToUpdateBody(
  v: BookingFormValues,
  previous: BookingFormValues
): RentalUpdateBookingBody {
  const text = (next: string, prev: string) => {
    const t = next.trim();
    return t || prev ? t : undefined;
  };
  return {
    guest_name: v.guestName.trim(),
    guest_email: text(v.guestEmail, previous.guestEmail),
    guest_phone: text(v.guestPhone, previous.guestPhone),
    guest_count: v.guestCount,
    check_in: v.checkIn,
    check_out: v.checkOut,
    total_amount: normalizedAmount(v.totalPrice),
    currency: normalizedAmount(v.totalPrice) ? optionalText(v.currency) : undefined,
    guest_notes: text(v.guestNotes, previous.guestNotes),
    internal_notes: text(v.internalNotes, previous.internalNotes),
  };
}
