//! Line/total money math, deterministic and shared by web/mobile/API (UC-ACC-05.2). OWNED BY: BE-Invoicing.
//!
//! Pure functions: no I/O. The single source of truth for line + document
//! totals so the header always equals the sum of the rounded line rows (mirrors
//! the #1522 invariant in
//! `db::repositories::accounting::create_invoice_rls`).
//!
//! ## The rounding discipline (UC-ACC-05.2/.20)
//! Each line is rounded ONCE here to money precision (2 dp). The document total
//! is the SUM of those already-rounded line values — never a re-sum of full
//! precision inputs — so summing the stored `invoice_item` rows can never drift
//! from the `invoice` header by a cent. Discount reduces the taxable base BEFORE
//! VAT is applied (UC-ACC-05.3).

use crate::errors::{AccountingError, Result};
use crate::rounding::{round_money, MONEY_DP};
use crate::types::{LineInput, Money, RoundingMode};
use rust_decimal::Decimal;

/// 100, used to turn percentages (`vat_rate`, `discount_pct`) into fractions.
fn hundred() -> Decimal {
    Decimal::from(100)
}

/// Validate a single line's inputs (UC-ACC-05.2). Negative quantities/prices are
/// rejected here; signed *credit-note* amounts are produced by negating an
/// already-valid [`Money`] (see `vat::reverse_groups`), not by feeding negative
/// inputs into the line engine.
fn validate_line(line: &LineInput) -> Result<()> {
    if line.qty < Decimal::ZERO {
        return Err(AccountingError::InvalidAmount(format!(
            "line quantity must be >= 0, got {}",
            line.qty
        )));
    }
    if line.unit_price < Decimal::ZERO {
        return Err(AccountingError::InvalidAmount(format!(
            "line unit_price must be >= 0, got {}",
            line.unit_price
        )));
    }
    if line.vat_rate < Decimal::ZERO {
        return Err(AccountingError::InvalidAmount(format!(
            "line vat_rate must be >= 0, got {}",
            line.vat_rate
        )));
    }
    if line.discount_pct < Decimal::ZERO || line.discount_pct > hundred() {
        return Err(AccountingError::InvalidAmount(format!(
            "line discount_pct must be in 0..=100, got {}",
            line.discount_pct
        )));
    }
    Ok(())
}

/// The net (taxable base) of a line BEFORE rounding, with the discount applied
/// to the base first (UC-ACC-05.3). Kept separate so VAT grouping (`vat.rs`)
/// computes the same un-rounded base then rounds consistently.
pub(crate) fn raw_line_base(line: &LineInput) -> Decimal {
    let gross_of_discount = line.qty * line.unit_price;
    let discount_factor = (hundred() - line.discount_pct) / hundred();
    gross_of_discount * discount_factor
}

/// Compute one line's net/vat/gross from quantity, unit price, discount and VAT
/// rate, rounded per `mode`. Discount reduces the taxable base BEFORE VAT
/// (UC-ACC-05.3).
///
/// Rounding order (deterministic, UC-ACC-05.2):
///   1. `base = round_money(qty * unit_price * (1 - discount%))`
///   2. `vat  = round_money(base * rate%)`  — VAT is computed off the *rounded*
///      base so the gross is internally consistent.
///   3. `total = base + vat`               — exact, no extra rounding needed.
pub fn compute_line(line: &LineInput, mode: RoundingMode) -> Result<Money> {
    validate_line(line)?;

    let base = round_money(raw_line_base(line), mode);
    let vat = round_money(base * (line.vat_rate / hundred()), mode);
    let total = base + vat;

    Ok(Money { base, vat, total })
}

/// Compute the document totals by summing the SAME rounded per-line values used
/// for the stored line rows (no full-precision re-sum → no cent drift,
/// UC-ACC-05.2). An empty document totals to zero.
pub fn compute_document_total(lines: &[LineInput], mode: RoundingMode) -> Result<Money> {
    let mut base = Decimal::ZERO;
    let mut vat = Decimal::ZERO;
    let mut total = Decimal::ZERO;

    for line in lines {
        let m = compute_line(line, mode)?;
        base += m.base;
        vat += m.vat;
        total += m.total;
    }

    Ok(Money { base, vat, total })
}

/// Convert a gross (VAT-inclusive) price into its net equivalent given the VAT
/// rate (net↔gross round-trip, UC-ACC-04.2). Used when a catalog item is priced
/// gross (`price_is_gross`) but the document is composed net.
///
/// `net = gross / (1 + rate/100)`, rounded to money precision per `mode`.
pub fn net_from_gross(gross: Decimal, vat_rate: Decimal, mode: RoundingMode) -> Decimal {
    let divisor = Decimal::ONE + (vat_rate / hundred());
    if divisor.is_zero() {
        return Decimal::ZERO;
    }
    round_money(gross / divisor, mode)
}

/// Convert a net price into its gross equivalent given the VAT rate.
///
/// `gross = net * (1 + rate/100)`, rounded to money precision per `mode`.
pub fn gross_from_net(net: Decimal, vat_rate: Decimal, mode: RoundingMode) -> Decimal {
    round_money(net * (Decimal::ONE + (vat_rate / hundred())), mode)
}

/// Multiply a document-currency [`Money`] breakdown by an FX `rate` to obtain
/// the base-currency equivalent persisted on the invoice (`base_currency_amount`,
/// UC-ACC-05.4). Each component is rounded to money precision so the stored
/// base-currency total is self-consistent.
pub fn to_base_currency(money: &Money, rate: Decimal, mode: RoundingMode) -> Money {
    Money {
        base: round_money(money.base * rate, mode),
        vat: round_money(money.vat * rate, mode),
        total: round_money(money.total * rate, mode),
    }
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
    fn simple_line_no_discount() {
        // 2 * 100.00 = 200.00 net, 20% VAT = 40.00, gross 240.00
        let m = compute_line(&line("2", "100.00", "20", "0"), RoundingMode::Arithmetic).unwrap();
        assert_eq!(m.base, d("200.00"));
        assert_eq!(m.vat, d("40.00"));
        assert_eq!(m.total, d("240.00"));
    }

    #[test]
    fn discount_reduces_base_before_vat() {
        // 1 * 100 with 10% discount → base 90.00, VAT 21% = 18.90, gross 108.90
        let m = compute_line(&line("1", "100", "21", "10"), RoundingMode::Arithmetic).unwrap();
        assert_eq!(m.base, d("90.00"));
        assert_eq!(m.vat, d("18.90"));
        assert_eq!(m.total, d("108.90"));
    }

    #[test]
    fn zero_vat_line() {
        let m = compute_line(&line("3", "50", "0", "0"), RoundingMode::Arithmetic).unwrap();
        assert_eq!(m.base, d("150.00"));
        assert_eq!(m.vat, Decimal::ZERO);
        assert_eq!(m.total, d("150.00"));
    }

    #[test]
    fn line_rounds_to_two_dp() {
        // 3 * 3.33 = 9.99 net, 20% = 1.998 → 2.00 (arithmetic)
        let m = compute_line(&line("3", "3.33", "20", "0"), RoundingMode::Arithmetic).unwrap();
        assert_eq!(m.base, d("9.99"));
        assert_eq!(m.vat, d("2.00"));
        assert_eq!(m.total, d("11.99"));
    }

    #[test]
    fn fractional_quantity_4dp() {
        // 1.5 hours * 80.00 = 120.00, 20% = 24.00
        let m = compute_line(&line("1.5", "80.00", "20", "0"), RoundingMode::Arithmetic).unwrap();
        assert_eq!(m.base, d("120.00"));
        assert_eq!(m.vat, d("24.00"));
    }

    #[test]
    fn negative_quantity_rejected() {
        let err = compute_line(&line("-1", "100", "20", "0"), RoundingMode::Arithmetic);
        assert!(matches!(err, Err(AccountingError::InvalidAmount(_))));
    }

    #[test]
    fn discount_over_100_rejected() {
        let err = compute_line(&line("1", "100", "20", "150"), RoundingMode::Arithmetic);
        assert!(matches!(err, Err(AccountingError::InvalidAmount(_))));
    }

    #[test]
    fn document_total_equals_sum_of_rounded_lines() {
        // Two lines each rounding 9.99 net + 2.00 vat. Header must equal the sum
        // of the rounded rows, not a re-sum of full precision (#1522 invariant).
        let lines = vec![
            line("3", "3.33", "20", "0"),
            line("3", "3.33", "20", "0"),
        ];
        let m = compute_document_total(&lines, RoundingMode::Arithmetic).unwrap();
        assert_eq!(m.base, d("19.98"));
        assert_eq!(m.vat, d("4.00"));
        assert_eq!(m.total, d("23.98"));
    }

    #[test]
    fn empty_document_totals_zero() {
        let m = compute_document_total(&[], RoundingMode::Arithmetic).unwrap();
        assert_eq!(m.base, Decimal::ZERO);
        assert_eq!(m.vat, Decimal::ZERO);
        assert_eq!(m.total, Decimal::ZERO);
    }

    #[test]
    fn net_gross_round_trip() {
        // 121.00 gross at 21% → 100.00 net → back to 121.00 gross
        let net = net_from_gross(d("121.00"), d("21"), RoundingMode::Arithmetic);
        assert_eq!(net, d("100.00"));
        let gross = gross_from_net(net, d("21"), RoundingMode::Arithmetic);
        assert_eq!(gross, d("121.00"));
    }

    #[test]
    fn fx_to_base_currency() {
        // 100 EUR base at rate 25.0 → 2500 CZK base
        let money = Money {
            base: d("100.00"),
            vat: d("20.00"),
            total: d("120.00"),
        };
        let conv = to_base_currency(&money, d("25.0"), RoundingMode::Arithmetic);
        assert_eq!(conv.base, d("2500.00"));
        assert_eq!(conv.vat, d("500.00"));
        assert_eq!(conv.total, d("3000.00"));
    }
}
