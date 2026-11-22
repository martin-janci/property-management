/**
 * Regression tests for the application-list URL helpers in `leases.tsx`.
 *
 * `leases.tsx` is a high-churn route-group file (>1900 lines, touched by
 * several PRs/week) and owns the URL<->filter helpers for every list route.
 * The lease-list helpers (`readLeaseListParams` / `writeLeaseListParams` /
 * `toLeaseListQuery`) are already pinned in `leases.detail.route.test.tsx`,
 * but the application-list pair (`readApplicationListParams` /
 * `writeApplicationListParams`) had no direct unit coverage — only indirect
 * exercise through `ApplicationsPageRoute`. These tests pin their current
 * behaviour so future edits to the file can't silently change how
 * `?status=&unit=&page=` round-trips.
 *
 * Pinned behaviour:
 *  - `status` is validated against the supported UI statuses; unknown values
 *    are dropped (not forwarded to the backend).
 *  - `unit` round-trips as `unitId`; an empty string collapses to `undefined`.
 *  - `page` is a positive integer defaulting to 1; non-positive / non-integer
 *    values fall back to 1 and page 1 is omitted from the serialized params.
 */

import { describe, expect, it } from 'vitest';
import type { ApplicationListParams } from '../../features/leases/pages/ApplicationsPage';
import { readApplicationListParams, writeApplicationListParams } from './leases';

describe('readApplicationListParams', () => {
  it('reads a supported status, unit and page', () => {
    const { filters, page } = readApplicationListParams(
      new URLSearchParams('status=screening&unit=unit-7&page=3')
    );
    expect(filters.status).toBe('screening');
    expect(filters.unitId).toBe('unit-7');
    expect(page).toBe(3);
  });

  it('drops an unsupported status value', () => {
    const { filters } = readApplicationListParams(new URLSearchParams('status=not_a_status'));
    expect(filters.status).toBeUndefined();
  });

  it('collapses an empty unit param to undefined', () => {
    const { filters } = readApplicationListParams(new URLSearchParams('unit='));
    expect(filters.unitId).toBeUndefined();
  });

  it('defaults page to 1 when absent, non-positive or non-integer', () => {
    expect(readApplicationListParams(new URLSearchParams('')).page).toBe(1);
    expect(readApplicationListParams(new URLSearchParams('page=0')).page).toBe(1);
    expect(readApplicationListParams(new URLSearchParams('page=-2')).page).toBe(1);
    expect(readApplicationListParams(new URLSearchParams('page=2.5')).page).toBe(1);
  });
});

describe('writeApplicationListParams', () => {
  const base: ApplicationListParams = { page: 1, pageSize: 12 };

  it('drops empty status / unit and page 1', () => {
    expect(writeApplicationListParams(base).toString()).toBe('');
  });

  it('serializes status, unit and a page beyond the first', () => {
    const out = writeApplicationListParams({
      ...base,
      status: 'under_review',
      unitId: 'unit-7',
      page: 4,
    });
    expect(out.get('status')).toBe('under_review');
    expect(out.get('unit')).toBe('unit-7');
    expect(out.get('page')).toBe('4');
  });

  it('omits page when on the first page', () => {
    const out = writeApplicationListParams({ ...base, status: 'approved', page: 1 });
    expect(out.has('page')).toBe(false);
    expect(out.get('status')).toBe('approved');
  });
});

describe('application list URL helpers round-trip', () => {
  it('read(write(x)) preserves the supported filters', () => {
    const written = writeApplicationListParams({
      status: 'rejected',
      unitId: 'unit-9',
      page: 2,
      pageSize: 12,
    });
    const { filters, page } = readApplicationListParams(written);
    expect(filters.status).toBe('rejected');
    expect(filters.unitId).toBe('unit-9');
    expect(page).toBe(2);
  });
});
