/**
 * Unit tests for the live invoice totals helper (UC-ACC-05.2, .19, .20).
 *
 * Runnable with the Node built-in test runner (no extra dependency):
 *   node --test --import tsx src/components/invoices/totals.test.ts
 * (or via whatever runner Phase 2 wires up — the assertions are runner-agnostic
 * `node:assert` calls). Uses `node:test` + `node:assert`, both covered by the
 * existing `@types/node` devDependency, so this file typechecks under
 * `tsc --noEmit` without adding a test framework.
 */

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { computeDocumentTotals, computeLine, roundHalfUp } from './totals';

test('roundHalfUp rounds to 2 decimals half-up', () => {
  assert.equal(roundHalfUp(1.005, 2), 1.01);
  assert.equal(roundHalfUp(2.675, 2), 2.68);
  assert.equal(roundHalfUp(10, 2), 10);
  assert.equal(roundHalfUp(0.1 + 0.2, 2), 0.3);
});

test('computeLine: net, VAT and gross for a single line', () => {
  // 3 × 100.00 @ 20% → net 300.00, VAT 60.00, gross 360.00
  const lt = computeLine({ qty: '3', unit_price: '100', vat_rate: '20' });
  assert.equal(lt.base, 300);
  assert.equal(lt.vat, 60);
  assert.equal(lt.gross, 360);
});

test('computeLine: 4-dp precision on inputs retained before rounding', () => {
  // 1.5 × 33.3333 = 49.99995 → base rounds to 50.00; VAT @ 21% = 10.50
  const lt = computeLine({ qty: '1.5', unit_price: '33.3333', vat_rate: '21' });
  assert.equal(lt.base, 50);
  assert.equal(lt.vat, 10.5);
  assert.equal(lt.gross, 60.5);
});

test('computeDocumentTotals: groups VAT per rate (mixed rates)', () => {
  const totals = computeDocumentTotals([
    { qty: '1', unit_price: '100', vat_rate: '20' }, // base 100, vat 20
    { qty: '2', unit_price: '50', vat_rate: '20' }, //  base 100, vat 20
    { qty: '1', unit_price: '200', vat_rate: '10' }, // base 200, vat 20
    { qty: '1', unit_price: '300', vat_rate: '0' }, //  base 300, vat 0
  ]);

  // Three rate groups, ascending by rate
  assert.equal(totals.groups.length, 3);
  assert.deepEqual(
    totals.groups.map((g) => g.rate),
    [0, 10, 20]
  );

  const g20 = totals.groups.find((g) => g.rate === 20);
  assert.ok(g20);
  assert.equal(g20?.base, 200); // 100 + 100
  assert.equal(g20?.vat, 40);

  const g10 = totals.groups.find((g) => g.rate === 10);
  assert.equal(g10?.base, 200);
  assert.equal(g10?.vat, 20);

  const g0 = totals.groups.find((g) => g.rate === 0);
  assert.equal(g0?.base, 300);
  assert.equal(g0?.vat, 0);

  // Document totals
  assert.equal(totals.base, 700); // 200 + 200 + 300
  assert.equal(totals.vat, 60); //  40 + 20 + 0
  assert.equal(totals.gross, 760);
});

test('computeDocumentTotals: ignores blank/zero lines', () => {
  const totals = computeDocumentTotals([
    { qty: '0', unit_price: '0', vat_rate: '20' },
    { qty: '1', unit_price: '100', vat_rate: '20' },
  ]);
  assert.equal(totals.groups.length, 1);
  assert.equal(totals.base, 100);
  assert.equal(totals.vat, 20);
  assert.equal(totals.gross, 120);
});

test('computeDocumentTotals: empty document is all zero', () => {
  const totals = computeDocumentTotals([]);
  assert.equal(totals.base, 0);
  assert.equal(totals.vat, 0);
  assert.equal(totals.gross, 0);
  assert.equal(totals.groups.length, 0);
});
