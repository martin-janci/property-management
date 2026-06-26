//! VAT computation & grouping by rate (UC-ACC-05.2 / EPIC-ACC-10). OWNED BY: BE-Invoicing.
//!
//! Pure functions: no I/O. Two responsibilities:
//!   1. `group_by_rate` — bucket lines by VAT rate and sum the (rounded) base +
//!      VAT per bucket. Drives the "VAT breakdown by rate" totals panel and the
//!      VAT-records feed (EPIC-ACC-10). Document-level VAT mode.
//!   2. `reverse_groups` — produce the signed (negated) groups for a credit
//!      note so the correction offsets the corrected invoice exactly
//!      (UC-ACC-05.7, VAT reversal).
//!
//! The grouping rounds per-line first (via `money::compute_line`) and sums the
//! rounded values — the same discipline `money::compute_document_total` uses —
//! so the VAT breakdown reconciles to the document total to the cent.

use crate::errors::Result;
use crate::money::compute_line;
use crate::types::{LineInput, RoundingMode, VatGroup};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// 100, used to turn a percentage rate into a fraction.
fn hundred() -> Decimal {
    Decimal::from(100)
}

/// Compute the VAT amount for a single net base at the given rate, rounded per
/// `mode`. Exposed for callers that already hold a rounded base (e.g. document
/// vs line VAT-mode comparisons).
pub fn vat_for_base(base: Decimal, vat_rate: Decimal, mode: RoundingMode) -> Decimal {
    crate::rounding::round_money(base * (vat_rate / hundred()), mode)
}

/// Group lines by VAT rate and return one [`VatGroup`] per distinct rate, each
/// with the summed base and summed VAT (the "VAT breakdown by rate" panel and
/// the VAT-records feed). Rates are returned in ASCENDING order.
///
/// Per-line values are rounded first (`money::compute_line`) then summed into
/// the bucket, so `Σ group.base == document.base` and `Σ group.vat ==
/// document.vat` exactly (UC-ACC-05.2).
pub fn group_by_rate(lines: &[LineInput], mode: RoundingMode) -> Result<Vec<VatGroup>> {
    // BTreeMap keyed by the Decimal rate keeps the output deterministically
    // sorted ascending by rate without a separate sort pass.
    let mut buckets: BTreeMap<Decimal, (Decimal, Decimal)> = BTreeMap::new();

    for line in lines {
        let m = compute_line(line, mode)?;
        let entry = buckets
            .entry(line.vat_rate)
            .or_insert((Decimal::ZERO, Decimal::ZERO));
        entry.0 += m.base;
        entry.1 += m.vat;
    }

    Ok(buckets
        .into_iter()
        .map(|(rate, (base, vat))| VatGroup { rate, base, vat })
        .collect())
}

/// Reverse the VAT on a credit note: returns the signed (negated) VAT groups
/// that offset the corrected invoice's groups (UC-ACC-05.7). Each group's base
/// and VAT are negated; the rate is preserved so the correction lands in the
/// same VAT-records bucket as the original (EPIC-ACC-10).
pub fn reverse_groups(groups: &[VatGroup]) -> Vec<VatGroup> {
    groups
        .iter()
        .map(|g| VatGroup {
            rate: g.rate,
            base: -g.base,
            vat: -g.vat,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn line(qty: &str, price: &str, vat: &str, disc: &str) -> LineInput {
        LineInput {
            qty: d(qty),
            unit_price: d(price),
            vat_rate: d(vat),
            discount_pct: d(disc),
        }
    }

    #[test]
    fn vat_for_base_rounds() {
        assert_eq!(
            vat_for_base(d("100.00"), d("20"), RoundingMode::Arithmetic),
            d("20.00")
        );
        assert_eq!(
            vat_for_base(d("9.99"), d("20"), RoundingMode::Arithmetic),
            d("2.00")
        );
    }

    #[test]
    fn groups_single_rate() {
        let lines = vec![line("2", "100", "20", "0"), line("1", "50", "20", "0")];
        let groups = group_by_rate(&lines, RoundingMode::Arithmetic).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].rate, d("20"));
        assert_eq!(groups[0].base, d("250.00"));
        assert_eq!(groups[0].vat, d("50.00"));
    }

    #[test]
    fn groups_mixed_rates_sorted_ascending() {
        // 21% line, 10% line, 0% line — output must be sorted 0, 10, 21.
        let lines = vec![
            line("1", "100", "21", "0"),
            line("1", "100", "10", "0"),
            line("1", "100", "0", "0"),
        ];
        let groups = group_by_rate(&lines, RoundingMode::Arithmetic).unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].rate, d("0"));
        assert_eq!(groups[1].rate, d("10"));
        assert_eq!(groups[2].rate, d("21"));
        assert_eq!(groups[0].vat, d("0.00"));
        assert_eq!(groups[1].vat, d("10.00"));
        assert_eq!(groups[2].vat, d("21.00"));
    }

    #[test]
    fn groups_reconcile_to_document_total() {
        let lines = vec![line("3", "3.33", "20", "0"), line("1", "100", "10", "0")];
        let groups = group_by_rate(&lines, RoundingMode::Arithmetic).unwrap();
        let doc = crate::money::compute_document_total(&lines, RoundingMode::Arithmetic).unwrap();
        let base_sum: Decimal = groups.iter().map(|g| g.base).sum();
        let vat_sum: Decimal = groups.iter().map(|g| g.vat).sum();
        assert_eq!(base_sum, doc.base);
        assert_eq!(vat_sum, doc.vat);
    }

    #[test]
    fn reverse_negates_each_group() {
        let groups = vec![
            VatGroup {
                rate: d("20"),
                base: d("100.00"),
                vat: d("20.00"),
            },
            VatGroup {
                rate: d("10"),
                base: d("50.00"),
                vat: d("5.00"),
            },
        ];
        let reversed = reverse_groups(&groups);
        assert_eq!(reversed[0].base, d("-100.00"));
        assert_eq!(reversed[0].vat, d("-20.00"));
        assert_eq!(reversed[0].rate, d("20"));
        assert_eq!(reversed[1].base, d("-50.00"));
        assert_eq!(reversed[1].vat, d("-5.00"));
    }

    #[test]
    fn reverse_then_sum_nets_to_zero() {
        let groups = vec![VatGroup {
            rate: d("20"),
            base: d("100.00"),
            vat: d("20.00"),
        }];
        let reversed = reverse_groups(&groups);
        assert_eq!(groups[0].vat + reversed[0].vat, Decimal::ZERO);
        assert_eq!(groups[0].base + reversed[0].base, Decimal::ZERO);
    }

    #[test]
    fn empty_lines_yield_no_groups() {
        let groups = group_by_rate(&[], RoundingMode::Arithmetic).unwrap();
        assert!(groups.is_empty());
    }
}
