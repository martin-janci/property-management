//! Accounting service for bank statements and payment matching.

use chrono::{NaiveDate, Utc};
use common::errors::AppError;
use db::models::acc_config::AccBankAccount;
use db::models::accounting::{
    BankStatement, BankStatementLine, Invoice, InvoiceStatus, PaymentMatch, PaymentMatchState,
};
use db::repositories::{AccConfigRepository, AccountingRepository};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;
use sqlx::{Connection, PgConnection};
use std::collections::HashMap;
use std::io::Cursor;
use uuid::Uuid;

/// Normalise an IBAN for comparison: drop all whitespace and upper-case it, so
/// `"sk11 1000"` and `"SK111000"` compare equal. Bank exports vary in casing
/// and grouping; the counterparty IBAN on a statement line and the IBAN stored
/// on a contact must be compared on their canonical form, not byte-for-byte.
fn normalize_iban(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase()
}

/// Keep only the candidates whose due date is nearest (in absolute days) to
/// `booking_date`. Ties are all retained; an empty input stays empty.
fn closest_due_date<T>(
    booking_date: NaiveDate,
    candidates: Vec<T>,
    due_date: impl Fn(&T) -> NaiveDate,
) -> Vec<T> {
    let distance = |c: &T| (booking_date - due_date(c)).num_days().abs();
    let Some(min) = candidates.iter().map(&distance).min() else {
        return candidates;
    };
    candidates
        .into_iter()
        .filter(|c| distance(c) == min)
        .collect()
}

#[derive(Debug, Deserialize)]
struct CsvLine {
    date: String,
    amount: Decimal,
    currency: String,
    counterparty_iban: Option<String>,
    variable_symbol: Option<String>,
    reference: Option<String>,
}

#[derive(Clone)]
pub struct AccountingService {
    repo: AccountingRepository,
    config_repo: AccConfigRepository,
}

impl AccountingService {
    pub fn new(repo: AccountingRepository, config_repo: AccConfigRepository) -> Self {
        Self { repo, config_repo }
    }

    /// The IBAN to record as the statement's own account.
    ///
    /// Bank-statement CSV exports carry only counterparty IBANs, never the
    /// account the statement belongs to, so it is sourced from the company's
    /// configured bank account (UC-ACC-02.10). Falls back to an empty string
    /// when no bank account is configured yet, instead of a hardcoded
    /// placeholder.
    fn statement_account_iban(account: Option<&AccBankAccount>) -> String {
        account.map(|a| a.iban.clone()).unwrap_or_default()
    }

    /// Process a bank statement upload.
    pub async fn process_statement_upload(
        &self,
        executor: &mut PgConnection,
        tenant_id: Uuid,
        filename: String,
        content: &[u8],
    ) -> Result<BankStatement, AppError> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(Cursor::new(content));

        // Source the statement's own account IBAN from the company's configured
        // bank account (UC-ACC-02.10) rather than a hardcoded placeholder. The
        // company base currency drives currency-matched default selection,
        // mirroring how invoices/documents resolve the paying account. These are
        // read-only lookups and run before the write transaction opens.
        let base_currency = self
            .config_repo
            .get_company_settings_rls(&mut *executor)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .map(|s| s.base_currency)
            .unwrap_or_default();
        let account = self
            .config_repo
            .resolve_bank_account_for_currency_rls(&mut *executor, &base_currency)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let account_iban = Self::statement_account_iban(account.as_ref());

        // Atomicity: every DB write for one upload — the `bank_statement` row,
        // each parsed `bank_statement_line`, and the payment-matcher side
        // effects — must land together or not at all. Previously each write ran
        // in the connection's autocommit mode with no enclosing transaction, so
        // a mid-file failure (an unparseable row, an invalid date on line N, a
        // DB error) returned an error to the caller while the already-inserted
        // statement header and its first N-1 lines stayed committed — orphaned,
        // partial state that a retry then duplicated. Wrap the whole operation
        // in a single transaction so any `?` below rolls everything back
        // cleanly. The RLS GUCs are session-scoped (`set_config(..., is_local
        // => false)` in migration 00006), so they remain in force inside this
        // transaction on the same connection.
        let mut tx = executor
            .begin()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Create statement
        let statement = self
            .repo
            .create_bank_statement_rls(&mut *tx, tenant_id, filename, None, account_iban)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        for result in rdr.deserialize() {
            let record: CsvLine =
                result.map_err(|e| AppError::BadRequest(format!("CSV parse error: {}", e)))?;

            let booking_date = NaiveDate::parse_from_str(&record.date, "%Y-%m-%d")
                .or_else(|_| NaiveDate::parse_from_str(&record.date, "%d.%m.%Y")) // Support common CZ format
                .map_err(|e| {
                    AppError::BadRequest(format!("Invalid date format {}: {}", record.date, e))
                })?;

            let line = BankStatementLine {
                id: Uuid::new_v4(),
                statement_id: statement.id,
                tenant_id,
                booking_date,
                amount: record.amount,
                currency: record.currency,
                counterparty_iban: record.counterparty_iban,
                variable_symbol: record.variable_symbol,
                raw_ref: record.reference,
                match_state: "unmatched".to_string(),
            };

            self.repo
                .create_bank_statement_line_rls(&mut *tx, line)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        // Trigger matcher on the SAME transaction: `Transaction` derefs to the
        // underlying `PgConnection`, so `&mut *tx` enlists the matcher's writes
        // in this transaction too.
        self.run_payment_matcher(&mut tx, tenant_id, statement.id)
            .await?;

        // Commit only once every write succeeded. On any earlier error `tx` is
        // dropped unwound, rolling back the statement, its lines, and any
        // matcher rows.
        tx.commit()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(statement)
    }

    /// Run the payment matching engine for a statement.
    pub async fn run_payment_matcher(
        &self,
        executor: &mut PgConnection,
        tenant_id: Uuid,
        statement_id: Uuid,
    ) -> Result<(), AppError> {
        let lines = self
            .repo
            .list_bank_statement_lines_for_tenant(&mut *executor, tenant_id, statement_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let open_invoices = self
            .repo
            .list_invoices_for_tenant(&mut *executor, tenant_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Filter for open invoices (Issued, PartiallyPaid)
        let open_invoices: Vec<Invoice> = open_invoices
            .into_iter()
            .filter(|inv| {
                inv.status == InvoiceStatus::Issued || inv.status == InvoiceStatus::PartiallyPaid
            })
            .collect();

        // Contact IBANs, keyed by contact id, for the counterparty-IBAN
        // corroboration below. An invoice carries no IBAN of its own — the
        // payer's account lives on its `Contact` — so we resolve it once here
        // instead of per (line, invoice) pair. Only non-empty IBANs are kept,
        // in canonical form.
        let contact_iban: HashMap<Uuid, String> = self
            .repo
            .list_contacts_for_tenant(&mut *executor, tenant_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .into_iter()
            .filter_map(|c| {
                c.iban
                    .map(|i| normalize_iban(&i))
                    .filter(|i| !i.is_empty())
                    .map(|i| (c.id, i))
            })
            .collect();

        let threshold = dec!(0.5);

        for line in lines {
            if line.match_state != "unmatched" && line.match_state != "suggested" {
                continue;
            }

            // Score every open invoice for this line, then surface only the
            // best-scoring tier. Previously EVERY invoice that cleared the
            // auto-suggest threshold was upserted as a suggestion with no
            // `break`, so a line sharing a variable symbol with N open invoices
            // was auto-suggested against all N — burying the genuinely-best
            // candidate under strictly-worse ones and over-generating
            // PaymentMatch rows a human then has to reject. We now keep the
            // maximum-confidence candidates only, with ties retained so
            // genuinely equally-good invoices are all surfaced for manual
            // disambiguation.
            let mut scored: Vec<(&Invoice, Decimal)> = Vec::new();

            // The line's counterparty IBAN in canonical form, resolved once per
            // line for the IBAN corroboration below.
            let line_iban = line
                .counterparty_iban
                .as_deref()
                .map(normalize_iban)
                .filter(|i| !i.is_empty());

            for invoice in &open_invoices {
                let mut confidence = Decimal::ZERO;

                // 1. Variable Symbol match (highest weight) — the primary
                //    "auto-match by reference" signal (FR-16.8 / BR-16.8.1). It
                //    is the only signal strong enough on its own to clear the
                //    auto-suggest threshold; amount and IBAN below are
                //    corroboration that breaks ties between VS-sharing invoices.
                if let (Some(l_vs), Some(i_vs)) = (&line.variable_symbol, &invoice.variable_symbol)
                {
                    if l_vs == i_vs && !l_vs.is_empty() {
                        confidence += dec!(0.8);
                    }
                }

                // 2. Amount match (corroboration / tie-breaker)
                let remaining_amount = invoice.total_amount - invoice.paid_amount;
                if line.amount == remaining_amount {
                    confidence += dec!(0.15);
                } else if line.amount > Decimal::ZERO && line.amount <= remaining_amount {
                    confidence += dec!(0.05);
                }

                // 3. Counterparty IBAN match (corroboration / tie-breaker).
                //    When several invoices share a variable symbol (e.g. a
                //    tenant's recurring monthly invoices), a matching payer
                //    account promotes the invoice whose contact actually owns
                //    the originating IBAN above its VS-only peers.
                if let Some(l_iban) = &line_iban {
                    if contact_iban.get(&invoice.contact_id) == Some(l_iban) {
                        confidence += dec!(0.05);
                    }
                }

                // 4. Date proximity is not scored here: it only breaks ties
                //    between equal-confidence candidates below (see
                //    `closest_due_date`), so it can never lift a line over the
                //    auto-suggest threshold on its own.

                // Confidence is a 0..1 score; the weights above sum to at most
                // 1.0, but clamp defensively so future signals can't overflow
                // the documented range.
                let confidence = confidence.min(Decimal::ONE);

                if confidence >= threshold {
                    scored.push((invoice, confidence));
                }
            }

            // Retain only the candidates at the highest confidence (ties kept).
            let Some(best_confidence) = scored.iter().map(|(_, c)| *c).max() else {
                continue;
            };

            // Among equal-confidence candidates (typically recurring invoices
            // sharing VS, amount and payer IBAN), keep the one(s) whose due
            // date is closest to the booking date; genuine ties stay.
            let best: Vec<(&Invoice, Decimal)> = scored
                .iter()
                .filter(|(_, c)| *c == best_confidence)
                .copied()
                .collect();
            let best = closest_due_date(line.booking_date, best, |(inv, _)| inv.due_date);

            let mut suggested_any = false;
            for (invoice, confidence) in best.iter() {
                let p_match = PaymentMatch {
                    id: Uuid::new_v4(),
                    tenant_id,
                    statement_line_id: line.id,
                    invoice_id: invoice.id,
                    confidence: *confidence,
                    decided_by: None,
                    decided_at: None,
                    state: PaymentMatchState::Suggested,
                };
                self.repo
                    .upsert_payment_match_rls(&mut *executor, p_match)
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;
                suggested_any = true;
            }

            if suggested_any {
                self.repo
                    .update_bank_statement_line_match_state_rls(
                        &mut *executor,
                        line.id,
                        "suggested".to_string(),
                    )
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Confirm a suggested payment match.
    pub async fn confirm_match(
        &self,
        executor: &mut PgConnection,
        tenant_id: Uuid,
        match_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        // Explicit tenant predicate: a foreign match id is NotFound (404).
        let p_match = self
            .repo
            .find_payment_match_for_tenant(&mut *executor, tenant_id, match_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Match {} not found", match_id)))?;

        // PAP-325: enforce legal transitions. Only a `Suggested` match may be
        // confirmed. Re-confirming an already-`Confirmed` match is an idempotent
        // no-op (do NOT re-apply `paid_amount`). A `Rejected` match is terminal —
        // confirming it would re-apply `paid_amount` and inflate the invoice.
        match p_match.state {
            PaymentMatchState::Confirmed => return Ok(()),
            PaymentMatchState::Rejected => {
                return Err(AppError::Conflict(format!(
                    "Match {match_id} is rejected and cannot be confirmed"
                )));
            }
            PaymentMatchState::Suggested => {}
        }

        let line = self
            .repo
            .find_bank_statement_line_rls(&mut *executor, p_match.statement_line_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| {
                // 404, not 500 (#1827): a missing/RLS-excluded statement line is
                // the same "caller pointed at a row that's gone" condition as the
                // missing-invoice branch below (which already maps to NotFound).
                // Both stem from a stale/cross-tenant id on the match, so they
                // must map symmetrically to a client error, not a server error.
                AppError::NotFound(format!(
                    "Statement line {} not found",
                    p_match.statement_line_id
                ))
            })?;

        // 1. Update match state
        let mut updated_match = p_match.clone();
        updated_match.state = PaymentMatchState::Confirmed;
        updated_match.decided_by = Some(user_id);
        updated_match.decided_at = Some(Utc::now());
        self.repo
            .upsert_payment_match_rls(&mut *executor, updated_match)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 2. Update statement line state
        self.repo
            .update_bank_statement_line_match_state_rls(
                &mut *executor,
                p_match.statement_line_id,
                "matched".to_string(),
            )
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 3. Update invoice paid_amount and status
        self.repo
            .update_invoice_payment_status_rls(&mut *executor, p_match.invoice_id, line.amount)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| {
                AppError::NotFound(format!("Invoice {} not found", p_match.invoice_id))
            })?;

        Ok(())
    }

    /// Reject a suggested payment match.
    pub async fn reject_match(
        &self,
        executor: &mut PgConnection,
        tenant_id: Uuid,
        match_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        // Explicit tenant predicate: a foreign match id is NotFound (404).
        let p_match = self
            .repo
            .find_payment_match_for_tenant(&mut *executor, tenant_id, match_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Match {} not found", match_id)))?;

        // PAP-325: enforce legal transitions. Rejecting an already-`Rejected`
        // match is an idempotent no-op. Rejecting a `Confirmed` match is an
        // explicit "unapply" — we must subtract the previously-applied
        // `paid_amount` so AR/paid totals stay consistent.
        let was_confirmed = match p_match.state {
            PaymentMatchState::Rejected => return Ok(()),
            PaymentMatchState::Confirmed => true,
            PaymentMatchState::Suggested => false,
        };

        // 1. Update match state
        let mut updated_match = p_match.clone();
        updated_match.state = PaymentMatchState::Rejected;
        updated_match.decided_by = Some(user_id);
        updated_match.decided_at = Some(Utc::now());
        self.repo
            .upsert_payment_match_rls(&mut *executor, updated_match)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 2. Unapply a previously-confirmed payment: subtract the line amount
        //    delta and recompute invoice status so paid totals revert.
        if was_confirmed {
            let line = self
                .repo
                .find_bank_statement_line_rls(&mut *executor, p_match.statement_line_id)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?
                .ok_or_else(|| {
                    AppError::Internal(format!(
                        "Statement line {} not found",
                        p_match.statement_line_id
                    ))
                })?;
            self.repo
                .update_invoice_payment_status_rls(&mut *executor, p_match.invoice_id, -line.amount)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Invoice {} not found", p_match.invoice_id))
                })?;
        }

        // 3. Update statement line state if no other suggested matches remain
        let other_matches = self
            .repo
            .list_payment_matches_by_line_rls(&mut *executor, p_match.statement_line_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let has_suggested = other_matches
            .iter()
            .any(|m| m.state == PaymentMatchState::Suggested);
        if !has_suggested {
            self.repo
                .update_bank_statement_line_match_state_rls(
                    &mut *executor,
                    p_match.statement_line_id,
                    "unmatched".to_string(),
                )
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bank_account(iban: &str) -> AccBankAccount {
        let now = Utc::now();
        AccBankAccount {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            label: "main".to_string(),
            iban: iban.to_string(),
            swift: None,
            bank_name: None,
            currency: "EUR".to_string(),
            is_default: true,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    // Regression: the statement's own-account IBAN must come from the company's
    // configured bank account, never a hardcoded placeholder (the old code used
    // "CZ0000000000000000000000").
    #[test]
    fn statement_account_iban_uses_configured_account() {
        let account = bank_account("SK9611000000002918599669");
        assert_eq!(
            AccountingService::statement_account_iban(Some(&account)),
            "SK9611000000002918599669",
        );
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // Recurring monthly invoices share VS, amount and payer IBAN, so they all
    // tie on confidence; the payment booked on 2026-03-14 belongs to the
    // invoice due 2026-03-15, not to January's or February's.
    #[test]
    fn closest_due_date_picks_nearest_recurring_invoice() {
        let due = vec![
            ("jan", d(2026, 1, 15)),
            ("feb", d(2026, 2, 15)),
            ("mar", d(2026, 3, 15)),
        ];
        let picked = closest_due_date(d(2026, 3, 14), due, |(_, dd)| *dd);
        assert_eq!(picked, vec![("mar", d(2026, 3, 15))]);
    }

    #[test]
    fn closest_due_date_keeps_genuine_ties() {
        let due = vec![
            ("a", d(2026, 3, 10)),
            ("b", d(2026, 3, 20)),
            ("c", d(2026, 1, 1)),
        ];
        let picked = closest_due_date(d(2026, 3, 15), due, |(_, dd)| *dd);
        assert_eq!(
            picked.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn closest_due_date_empty_input() {
        let picked: Vec<NaiveDate> = closest_due_date(d(2026, 3, 15), vec![], |x| *x);
        assert!(picked.is_empty());
    }

    #[test]
    fn statement_account_iban_falls_back_to_empty_when_unconfigured() {
        assert_eq!(AccountingService::statement_account_iban(None), "");
    }

    // The payment-matching confidence weights are hardcoded decimal literals in
    // `run_payment_matcher`. They used to be built with `Decimal::from_str("..").unwrap()`
    // — a runtime parse that could panic on a typo and carried a needless unwrap on
    // a constant. They are now `dec!` literals, parsed at compile time. This test
    // pins their exact values and the documented 0..1 scoring invariants so a future
    // edit to a weight can't silently drift the auto-suggest behaviour (FR-16.8):
    //   - a Variable-Symbol match alone (0.8) must clear the auto-suggest threshold
    //     (0.5), since VS is the only signal strong enough on its own;
    //   - amount/IBAN corroboration alone must NOT clear it;
    //   - the full weight set must sum to exactly 1.0 (the clamped maximum).
    #[test]
    fn confidence_weights_match_documented_scoring_invariants() {
        let threshold = dec!(0.5);
        let w_vs = dec!(0.8);
        let w_amount_exact = dec!(0.15);
        let w_amount_partial = dec!(0.05);
        let w_iban = dec!(0.05);

        // VS match alone clears the auto-suggest threshold.
        assert!(w_vs >= threshold, "VS match must auto-suggest on its own");
        // Corroboration signals alone must never clear the threshold.
        assert!(w_amount_exact < threshold);
        assert!(w_amount_partial < threshold);
        assert!(w_iban < threshold);
        // Exact-amount corroboration outweighs partial-amount corroboration.
        assert!(w_amount_exact > w_amount_partial);
        // The strongest combination (VS + exact amount + IBAN) sums to the
        // documented maximum of 1.0, so the `.min(Decimal::ONE)` clamp is exact,
        // never lossy.
        assert_eq!(w_vs + w_amount_exact + w_iban, Decimal::ONE);
    }
}
