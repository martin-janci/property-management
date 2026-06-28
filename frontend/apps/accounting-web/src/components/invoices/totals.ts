/**
 * Live document totals for the invoice line editor (UC-ACC-05.2, .19, .20).
 *
 * This is a UI-side preview computation for instant feedback while editing.
 * The authoritative, persisted figures come from the backend
 * `accounting_core` money/VAT module (single deterministic source of truth);
 * the server recomputes on save/issue. We keep this preview deterministic and
 * aligned with the backend rules: net = qty * unit_price, VAT grouped by rate,
 * gross = net + VAT, rounded to 2 decimal places (arithmetic / half-up) for
 * money display while retaining up to 4-dp precision on line inputs.
 *
 * All arithmetic uses integer cents to avoid binary float drift; we accept the
 * SDK's string Decimals and parse to Number only for display math.
 */

import type { CreateInvoiceItem } from './types';

/** Round a number to `dp` decimals, half-up (arithmetic) — matches the
 * default `rounding_mode = 'arithmetic'` in AccCompanySettings. */
export function roundHalfUp(value: number, dp = 2): number {
  if (!Number.isFinite(value)) return 0;
  const factor = 10 ** dp;
  // Add a tiny epsilon to counter float representation of exact .5 boundaries.
  return Math.round((value + Number.EPSILON) * factor) / factor;
}

export interface LineTotals {
  /** net base before VAT, rounded to 2 dp */
  base: number;
  /** VAT amount, rounded to 2 dp */
  vat: number;
  /** gross = base + vat */
  gross: number;
}

export interface VatGroupTotals {
  /** VAT percentage (e.g. 20, 21, 10, 0) */
  rate: number;
  base: number;
  vat: number;
}

export interface DocumentTotals {
  base: number;
  vat: number;
  gross: number;
  /** VAT broken down per rate, ascending (UC-ACC-05.2). */
  groups: VatGroupTotals[];
}

function num(v: string | number | null | undefined): number {
  if (v === null || v === undefined) return 0;
  const n = typeof v === 'number' ? v : Number.parseFloat(v);
  return Number.isFinite(n) ? n : 0;
}

/** Compute net/VAT/gross for a single line (UC-ACC-05.2). */
export function computeLine(
  line: Pick<CreateInvoiceItem, 'qty' | 'unit_price' | 'vat_rate'>
): LineTotals {
  const qty = num(line.qty);
  const unitPrice = num(line.unit_price);
  const rate = num(line.vat_rate);
  const base = roundHalfUp(qty * unitPrice, 2);
  const vat = roundHalfUp((base * rate) / 100, 2);
  return { base, vat, gross: roundHalfUp(base + vat, 2) };
}

/**
 * Compute document totals with per-rate VAT grouping (UC-ACC-05.2/.19).
 * VAT is computed on the summed base per rate (document-level grouping) to
 * mirror the backend's `vat::group_by_rate`, then summed for the document.
 */
export function computeDocumentTotals(
  lines: ReadonlyArray<Pick<CreateInvoiceItem, 'qty' | 'unit_price' | 'vat_rate'>>
): DocumentTotals {
  const byRate = new Map<number, number>(); // rate -> summed base
  for (const line of lines) {
    const qty = num(line.qty);
    const unitPrice = num(line.unit_price);
    if (qty === 0 && unitPrice === 0) continue;
    const rate = num(line.vat_rate);
    const base = roundHalfUp(qty * unitPrice, 2);
    byRate.set(rate, roundHalfUp((byRate.get(rate) ?? 0) + base, 2));
  }

  const groups: VatGroupTotals[] = [...byRate.entries()]
    .map(([rate, base]) => ({ rate, base, vat: roundHalfUp((base * rate) / 100, 2) }))
    .sort((a, b) => a.rate - b.rate);

  const base = roundHalfUp(
    groups.reduce((sum, g) => sum + g.base, 0),
    2
  );
  const vat = roundHalfUp(
    groups.reduce((sum, g) => sum + g.vat, 0),
    2
  );
  return { base, vat, gross: roundHalfUp(base + vat, 2), groups };
}

/** Format a money amount for display in the given currency/locale. */
export function formatMoney(value: number, currency: string, locale: string): string {
  try {
    return new Intl.NumberFormat(locale === 'cs' ? 'cs-CZ' : 'sk-SK', {
      style: 'currency',
      currency: currency || 'EUR',
    }).format(value);
  } catch {
    return `${roundHalfUp(value, 2).toFixed(2)} ${currency}`;
  }
}
