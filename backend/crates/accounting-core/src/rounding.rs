//! Deterministic rounding per company rounding mode (UC-ACC-02.11 / 05.20). OWNED BY: BE-Invoicing.
//!
//! Pure functions: no I/O. This is the lowest layer of the deterministic
//! money/VAT engine — `money` and `vat` round every intermediate value through
//! here so the totals are bit-for-bit reproducible across web/mobile/API.
//!
//! Rounding modes map 1:1 to `acc_company_settings.rounding_mode`
//! (`{arithmetic, bankers, none}`):
//! - `Arithmetic` → half-away-from-zero (the everyday "round half up", correct
//!   for negative credit-note amounts too).
//! - `Bankers`    → half-to-even (IFRS / statistical bias-free rounding).
//! - `None`       → keep full precision (no rounding at all).

use crate::types::RoundingMode;
use rust_decimal::{Decimal, RoundingStrategy};

/// Number of decimal places money is stored at in the schema
/// (`invoice.total_amount NUMERIC(18,2)`).
pub const MONEY_DP: u32 = 2;

/// Maximum per-line precision the editor allows (UC-ACC-05.20: "up to 4 dp").
pub const MAX_LINE_DP: u32 = 4;

/// Round `value` to `dp` decimal places using the given mode.
///
/// - `Arithmetic` → half-away-from-zero (`MidpointAwayFromZero`). For positive
///   amounts this is the familiar "round half up"; for negatives (credit notes)
///   it rounds the magnitude up, which keeps `round(-x) == -round(x)`.
/// - `Bankers` → half-to-even (`MidpointNearestEven`).
/// - `None` → returns `value` unchanged (full precision retained, UC-ACC-05.20).
pub fn round(value: Decimal, dp: u32, mode: RoundingMode) -> Decimal {
    match mode {
        RoundingMode::Arithmetic => {
            value.round_dp_with_strategy(dp, RoundingStrategy::MidpointAwayFromZero)
        }
        RoundingMode::Bankers => {
            value.round_dp_with_strategy(dp, RoundingStrategy::MidpointNearestEven)
        }
        RoundingMode::None => value,
    }
}

/// Round a monetary amount to 2 decimal places per mode (the common case for
/// stored `NUMERIC(18,2)` columns). `RoundingMode::None` still snaps to 2 dp
/// because the database column cannot hold more — "no rounding" only means we
/// skip the *cash* rounding step, not that we can store sub-cent amounts.
pub fn round_money(value: Decimal, mode: RoundingMode) -> Decimal {
    match mode {
        // Even in "none" mode the persisted column is NUMERIC(18,2); round to the
        // representable precision deterministically (half-away-from-zero) so the
        // Rust-side total always equals what Postgres stores.
        RoundingMode::None => {
            value.round_dp_with_strategy(MONEY_DP, RoundingStrategy::MidpointAwayFromZero)
        }
        _ => round(value, MONEY_DP, mode),
    }
}

/// Compute the cash-rounding adjustment that brings `total` to the nearest whole
/// currency unit (UC-ACC-05.20). Returns the signed delta to ADD to `total` to
/// reach the rounded grand total, i.e. `round_to_whole(total) - total`.
///
/// This is the value of the "rounding adjustment" line shown on the document.
/// When `total` is already whole (or `mode == None`, meaning cash rounding is
/// disabled) the delta is exactly zero and no adjustment line is needed.
///
/// Note: SK legacy 0.05 cash rounding is intentionally NOT special-cased here —
/// post-2009 EUR invoicing rounds to whole cents at the line level and only the
/// printed/cash grand total rounds to the whole unit. Whole-unit rounding is the
/// configurable behaviour the schema models; a future 0.05 mode can be added via
/// a new `RoundingMode` variant (contract-note) without touching callers.
pub fn rounding_adjustment(total: Decimal, mode: RoundingMode) -> Decimal {
    if mode == RoundingMode::None {
        return Decimal::ZERO;
    }
    let rounded = round(total, 0, mode);
    rounded - total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    // --- round() -----------------------------------------------------------

    #[test]
    fn arithmetic_rounds_half_up() {
        assert_eq!(round(d("1.005"), 2, RoundingMode::Arithmetic), d("1.01"));
        assert_eq!(round(d("1.004"), 2, RoundingMode::Arithmetic), d("1.00"));
        assert_eq!(round(d("2.5"), 0, RoundingMode::Arithmetic), d("3"));
    }

    #[test]
    fn arithmetic_is_symmetric_for_negatives() {
        // Credit notes carry negative amounts; round(-x) must equal -round(x).
        assert_eq!(round(d("-1.005"), 2, RoundingMode::Arithmetic), d("-1.01"));
        assert_eq!(round(d("-2.5"), 0, RoundingMode::Arithmetic), d("-3"));
    }

    #[test]
    fn bankers_rounds_half_to_even() {
        assert_eq!(round(d("2.5"), 0, RoundingMode::Bankers), d("2"));
        assert_eq!(round(d("3.5"), 0, RoundingMode::Bankers), d("4"));
        assert_eq!(round(d("0.125"), 2, RoundingMode::Bankers), d("0.12"));
        assert_eq!(round(d("0.135"), 2, RoundingMode::Bankers), d("0.14"));
    }

    #[test]
    fn none_keeps_full_precision() {
        assert_eq!(round(d("1.23456"), 2, RoundingMode::None), d("1.23456"));
    }

    #[test]
    fn round_respects_4dp_precision() {
        assert_eq!(round(d("1.234567"), 4, RoundingMode::Arithmetic), d("1.2346"));
    }

    // --- round_money() -----------------------------------------------------

    #[test]
    fn round_money_snaps_to_two_dp() {
        assert_eq!(round_money(d("1.005"), RoundingMode::Arithmetic), d("1.01"));
        assert_eq!(round_money(d("1.235"), RoundingMode::Bankers), d("1.24"));
        // even "none" snaps to the storable 2 dp
        assert_eq!(round_money(d("1.236"), RoundingMode::None), d("1.24"));
    }

    // --- rounding_adjustment() ---------------------------------------------

    #[test]
    fn rounding_adjustment_up_and_down() {
        // 100.40 → 100.00 : delta -0.40
        assert_eq!(
            rounding_adjustment(d("100.40"), RoundingMode::Arithmetic),
            d("-0.40")
        );
        // 100.60 → 101.00 : delta +0.40
        assert_eq!(
            rounding_adjustment(d("100.60"), RoundingMode::Arithmetic),
            d("0.40")
        );
    }

    #[test]
    fn rounding_adjustment_nets_to_zero_on_whole_total() {
        assert_eq!(
            rounding_adjustment(d("100.00"), RoundingMode::Arithmetic),
            Decimal::ZERO
        );
    }

    #[test]
    fn rounding_adjustment_disabled_when_mode_none() {
        // No cash rounding when the company opted out.
        assert_eq!(
            rounding_adjustment(d("100.40"), RoundingMode::None),
            Decimal::ZERO
        );
    }

    #[test]
    fn rounding_adjustment_then_total_is_whole() {
        let total = d("100.40");
        let adj = rounding_adjustment(total, RoundingMode::Arithmetic);
        assert_eq!(total + adj, d("100.00"));
    }
}
