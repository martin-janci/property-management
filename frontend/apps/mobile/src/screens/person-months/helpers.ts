/**
 * Pure domain helpers for PersonMonthsScreen (UC-10).
 *
 * Extracted from the screen so the parsing / validation / date logic can be
 * unit-tested and reasoned about without rendering the component. Everything
 * here is side-effect free; the only dependency is the i18n date formatter.
 */

import { formatDate } from '../../i18n/format';

/** Largest person-count a resident may report for a single month. */
export const MAX_REPORT_COUNT = 50;

/** How many months back (including the current one) the server accepts reports for. */
export const REPORT_WINDOW_MONTHS = 24;

/** One `PersonMonthResponse` row from the api-server. */
export interface PersonMonthRow {
  id: string;
  unit_id: string;
  year: number;
  month: number;
  count: number;
  source: string;
  notes: string | null;
}

/** One resident submission (`PersonMonthSubmission`). */
export interface SubmissionRow {
  id: string;
  year: number;
  month: number;
  count: number;
  status: 'pending' | 'approved' | 'rejected';
  submitted_at: string;
  decision_note: string | null;
}

/** Validate the raw response and sort newest month first. */
export function parsePersonMonths(data: unknown): PersonMonthRow[] {
  if (!Array.isArray(data)) return [];
  return data
    .filter(
      (r): r is PersonMonthRow =>
        !!r &&
        typeof r === 'object' &&
        typeof (r as PersonMonthRow).id === 'string' &&
        typeof (r as PersonMonthRow).month === 'number' &&
        typeof (r as PersonMonthRow).year === 'number' &&
        typeof (r as PersonMonthRow).count === 'number'
    )
    .sort((a, b) => b.year - a.year || b.month - a.month);
}

/** Recorded months keyed by month number (newest-first order is irrelevant for a Map). */
export function recordedByMonth(data: unknown): Map<number, PersonMonthRow> {
  return new Map(parsePersonMonths(data).map((r) => [r.month, r]));
}

/** Latest submission per month (input is newest-first from the server). */
export function latestSubmissionByMonth(data: unknown): Map<number, SubmissionRow> {
  const out = new Map<number, SubmissionRow>();
  if (!Array.isArray(data)) return out;
  for (const r of data as SubmissionRow[]) {
    if (!r || typeof r.month !== 'number' || typeof r.status !== 'string') continue;
    const prev = out.get(r.month);
    if (!prev || r.submitted_at > prev.submitted_at) out.set(r.month, r);
  }
  return out;
}

/** Months to show for `year`: up to the current month, newest first. */
export function visibleMonths(year: number, today = new Date()): number[] {
  const thisYear = today.getFullYear();
  if (year > thisYear) return [];
  const last = year === thisYear ? today.getMonth() + 1 : 12;
  return Array.from({ length: last }, (_, i) => last - i);
}

/** The server accepts reports for the current month and the 24 before it. */
export function canReport(year: number, month: number, today = new Date()): boolean {
  const period = year * 12 + month;
  const now = today.getFullYear() * 12 + today.getMonth() + 1;
  return period <= now && period >= now - REPORT_WINDOW_MONTHS;
}

/**
 * Parse a report draft into a valid count, or `null` when it is not a whole
 * number within the accepted range. Mirrors the server's validation so the
 * screen can reject bad input before issuing the mutation.
 */
export function parseReportCount(raw: string): number | null {
  if (raw.trim() === '') return null;
  const count = Number(raw);
  if (!Number.isInteger(count) || count < 0 || count > MAX_REPORT_COUNT) return null;
  return count;
}

/** Localised "Month Year" label (e.g. "March 2026"). */
export function formatMonth(year: number, month: number): string {
  return formatDate(new Date(year, month - 1, 1), { month: 'long', year: 'numeric' });
}
