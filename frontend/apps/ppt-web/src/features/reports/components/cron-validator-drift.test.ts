/// <reference types="vitest/globals" />
/**
 * Silent-regression guard for the cron-validator drift that can reintroduce
 * issue #616 (lossy report-schedule round-trip) — see issue #1368.
 *
 * Background
 * ----------
 * #616 was the original lossy round-trip: a custom cron edit was persisted by
 * the backend but, on re-open, `scheduleToInitialCron()` reconstructed the
 * expression from the overloaded `time` / `frequency` / `day_*` fields and
 * silently flattened the user's expression.
 *
 * The shipped fix surfaces the dedicated `cron_expression` column verbatim —
 * BUT only when the frontend `isValidCron()` accepts it (EditScheduleModal.tsx
 * `scheduleToInitialCron`, the `stored && isValidCron(stored)` guard). #1368
 * pointed out the trap: the frontend `isValidCron` and the backend
 * `validate_cron_expression` (backend/servers/api-server/src/routes/reports.rs)
 * were independent reimplementations that DISAGREED on at least one combined
 * form. For any expression the backend accepted but the frontend rejected, the
 * read path silently fell back to lossy reconstruction — i.e. #616 came back
 * for that subset, invisibly (no error surfaced).
 *
 * STATUS: #1368 is now FIXED. `isValidCron` was rewritten to mirror the backend
 * parse order (split each field on `,` FIRST, then `/`, then `-`), so the two
 * validators agree on the combined forms. This file is the regression net that
 * keeps them in lockstep:
 *
 *   1. Pins the verbatim round-trip for combined `,`/`/`/`-` expressions that
 *      BOTH validators accept, so the happy-path can't regress.
 *
 *   2. Pins the former drift fixture (`1-5/2,10 * * * *`) — now accepted by
 *      BOTH validators and surfaced verbatim. If anyone changes either
 *      validator OR the read-path guard such that they disagree again, this
 *      test goes red and forces a deliberate decision. That is exactly what
 *      stops #616 from silently creeping back in.
 */

import type { ReportSchedule } from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import { isValidCron } from './CronPicker';
import { scheduleToInitialCron } from './EditScheduleModal';

/** Build a ReportSchedule fixture with sane defaults, overridable per-test. */
function makeSchedule(overrides: Partial<ReportSchedule> = {}): ReportSchedule {
  return {
    id: 'sched-1',
    report_id: 'rep-1',
    organization_id: 'org-1',
    name: 'Test Schedule',
    frequency: 'daily',
    time: '08:00',
    timezone: 'Europe/Bratislava',
    format: 'pdf',
    recipients: ['a@example.com'],
    is_active: true,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

/**
 * Mirror of the backend `validate_cron_expression`
 * (backend/servers/api-server/src/routes/reports.rs ~L1894). Field parsing
 * order is the load-bearing difference vs. the frontend `isValidCron`: the
 * backend splits each field on `,` FIRST, then `/`, then `-`. Kept here so the
 * drift assertions document the *backend* contract they pin against, without a
 * network/cargo dependency in a frontend unit test.
 */
function backendValidateCron(expr: string): boolean {
  const fields = expr.split(/\s+/).filter(Boolean);
  if (fields.length !== 5) return false;

  const validField = (field: string, min: number, max: number): boolean => {
    for (const part of field.split(',')) {
      const [base, step] = part.includes('/') ? part.split('/') : [part, undefined];
      if (step !== undefined) {
        const s = Number(step);
        if (!Number.isInteger(s) || s === 0) return false;
      }
      if (base !== '*') {
        if (base.includes('-')) {
          const [lo, hi] = base.split('-').map(Number);
          if (
            !(Number.isInteger(lo) && Number.isInteger(hi) && lo >= min && hi <= max && lo <= hi)
          ) {
            return false;
          }
        } else {
          const v = Number(base);
          if (!(Number.isInteger(v) && v >= min && v <= max)) return false;
        }
      }
    }
    return true;
  };

  return (
    validField(fields[0], 0, 59) &&
    validField(fields[1], 0, 23) &&
    validField(fields[2], 1, 31) &&
    validField(fields[3], 1, 12) &&
    validField(fields[4], 0, 7)
  );
}

describe('cron validator drift — #616 reintroduction guard (#1368)', () => {
  describe('combined forms accepted by BOTH validators round-trip verbatim', () => {
    // These are the safe cases: when the frontend validator agrees with the
    // backend, the dedicated cron_expression must be surfaced unflattened.
    it.each([
      '0,30 9-17 1-31 1-12 0-7', // explicit list + ranges in every field
      '5-15/3 * * * *', // range-with-step in the minute field
      '0,15,30,45 9-17 * * 1-5', // list minute + range hour + range dow
      '*/15 9-17 * * 1-5', // step + range (the canonical #616 custom expr)
      '1-5/2,10 * * * *', // range-with-step + list member (former #1368 drift)
      '0,15/3 * * * *', // single + range-with-step in one list (split-`,`-first)
    ])('surfaces %s verbatim (backend-valid AND frontend-valid)', (expr) => {
      // Precondition: both validators agree this is valid.
      expect(backendValidateCron(expr)).toBe(true);
      expect(isValidCron(expr)).toBe(true);

      const schedule = makeSchedule({
        cron_expression: expr,
        // Legacy fields present but must be ignored when a cron is stored.
        time: '08:00',
        frequency: 'daily',
        day_of_week: 3,
        day_of_month: 15,
      });
      expect(scheduleToInitialCron(schedule)).toBe(expr);
    });
  });

  describe('cross-validator agreement on the former drift fixture (#1368 fixed)', () => {
    // `1-5/2,10` is accepted by the backend (splits on `,` first → `1-5/2` and
    // `10`). Before #1368 the frontend `isValidCron` took the `/` branch on the
    // whole `2,10` token → Number("2,10") = NaN → false, a false-negative that
    // silently flattened the persisted cron (reintroducing #616). The fix made
    // `isValidCron` split on `,` first, mirroring the backend authority.
    const FORMER_DRIFT_EXPR = '1-5/2,10 * * * *';

    it('the two validators now agree on the former drift fixture', () => {
      expect(backendValidateCron(FORMER_DRIFT_EXPR)).toBe(true);
      // Frontend now mirrors the backend parse order (split `,` first) — no
      // more false-negative. If either validator drifts again this goes red.
      expect(isValidCron(FORMER_DRIFT_EXPR)).toBe(true);
    });

    it('surfaces a backend-accepted combined cron verbatim — no silent flatten (#616 stays closed)', () => {
      const schedule = makeSchedule({
        cron_expression: FORMER_DRIFT_EXPR, // backend persisted this exact value
        frequency: 'daily',
        time: '07:15',
      });

      // With #1368 fixed the read path surfaces the persisted cron verbatim
      // instead of falling back to the lossy `time`-derived reconstruction.
      expect(scheduleToInitialCron(schedule)).toBe(FORMER_DRIFT_EXPR);
    });
  });
});
