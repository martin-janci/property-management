/**
 * Unit tests for the live invoice totals helper (UC-ACC-05.2, .19, .20).
 *
 * Converted from the original `node:test` + `node:assert` form to vitest
 * (#1828) so accounting-web has a single test runner — the new `vitest` glob
 * picked this file up and `node:test` registrations are invisible to vitest's
 * collector. Behaviour and assertions are unchanged.
 */

import { describe, expect, it } from 'vitest';
import { computeDocumentTotals, computeLine, roundHalfUp } from './totals';

describe('invoice totals helper', () => {
  it('roundHalfUp rounds to 2 decimals half-up', () => {
    expect(roundHalfUp(1.005, 2)).toBe(1.01);
    expect(roundHalfUp(2.675, 2)).toBe(2.68);
    expect(roundHalfUp(10, 2)).toBe(10);
    expect(roundHalfUp(0.1 + 0.2, 2)).toBe(0.3);
  });

  it('computeLine: net, VAT and gross for a single line', () => {
    // 3 × 100.00 @ 20% → net 300.00, VAT 60.00, gross 360.00
    const lt = computeLine({ qty: '3', unit_price: '100', vat_rate: '20' });
    expect(lt.base).toBe(300);
    expect(lt.vat).toBe(60);
    expect(lt.gross).toBe(360);
  });

  it('computeLine: 4-dp precision on inputs retained before rounding', () => {
    // 1.5 × 33.3333 = 49.99995 → base rounds to 50.00; VAT @ 21% = 10.50
    const lt = computeLine({ qty: '1.5', unit_price: '33.3333', vat_rate: '21' });
    expect(lt.base).toBe(50);
    expect(lt.vat).toBe(10.5);
    expect(lt.gross).toBe(60.5);
  });

  it('computeDocumentTotals: groups VAT per rate (mixed rates)', () => {
    const totals = computeDocumentTotals([
      { qty: '1', unit_price: '100', vat_rate: '20' }, // base 100, vat 20
      { qty: '2', unit_price: '50', vat_rate: '20' }, //  base 100, vat 20
      { qty: '1', unit_price: '200', vat_rate: '10' }, // base 200, vat 20
      { qty: '1', unit_price: '300', vat_rate: '0' }, //  base 300, vat 0
    ]);

    // Three rate groups, ascending by rate
    expect(totals.groups.length).toBe(3);
    expect(totals.groups.map((g) => g.rate)).toEqual([0, 10, 20]);

    const g20 = totals.groups.find((g) => g.rate === 20);
    expect(g20).toBeTruthy();
    expect(g20?.base).toBe(200); // 100 + 100
    expect(g20?.vat).toBe(40);

    const g10 = totals.groups.find((g) => g.rate === 10);
    expect(g10?.base).toBe(200);
    expect(g10?.vat).toBe(20);

    const g0 = totals.groups.find((g) => g.rate === 0);
    expect(g0?.base).toBe(300);
    expect(g0?.vat).toBe(0);

    // Document totals
    expect(totals.base).toBe(700); // 200 + 200 + 300
    expect(totals.vat).toBe(60); //  40 + 20 + 0
    expect(totals.gross).toBe(760);
  });

  it('computeDocumentTotals: ignores blank/zero lines', () => {
    const totals = computeDocumentTotals([
      { qty: '0', unit_price: '0', vat_rate: '20' },
      { qty: '1', unit_price: '100', vat_rate: '20' },
    ]);
    expect(totals.groups.length).toBe(1);
    expect(totals.base).toBe(100);
    expect(totals.vat).toBe(20);
    expect(totals.gross).toBe(120);
  });

  it('computeDocumentTotals: empty document is all zero', () => {
    const totals = computeDocumentTotals([]);
    expect(totals.base).toBe(0);
    expect(totals.vat).toBe(0);
    expect(totals.gross).toBe(0);
    expect(totals.groups.length).toBe(0);
  });
});
