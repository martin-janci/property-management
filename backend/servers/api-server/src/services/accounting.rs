//! Accounting service for bank statements and payment matching.

use chrono::{NaiveDate, Utc};
use common::errors::AppError;
use db::models::accounting::{
    BankStatement, BankStatementLine, Invoice, InvoiceStatus, PaymentMatch, PaymentMatchState,
};
use db::repositories::AccountingRepository;
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::PgConnection;
use std::io::Cursor;
use std::str::FromStr;
use uuid::Uuid;

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
}

impl AccountingService {
    pub fn new(repo: AccountingRepository) -> Self {
        Self { repo }
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

        // Create statement
        let statement = self
            .repo
            .create_bank_statement_rls(
                &mut *executor,
                tenant_id,
                filename,
                None,
                "CZ0000000000000000000000".to_string(), // Default IBAN if not in CSV
            )
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
                .create_bank_statement_line_rls(&mut *executor, line)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        // Trigger matcher
        self.run_payment_matcher(&mut *executor, tenant_id, statement.id)
            .await?;

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
            .list_bank_statement_lines_rls(&mut *executor, statement_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let open_invoices = self
            .repo
            .list_invoices_rls(&mut *executor)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Filter for open invoices (Issued, PartiallyPaid)
        let open_invoices: Vec<Invoice> = open_invoices
            .into_iter()
            .filter(|inv| {
                inv.status == InvoiceStatus::Issued || inv.status == InvoiceStatus::PartiallyPaid
            })
            .collect();

        let threshold = Decimal::from_str("0.5").unwrap();

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

            for invoice in &open_invoices {
                let mut confidence = Decimal::ZERO;

                // 1. Variable Symbol match (highest weight)
                if let (Some(l_vs), Some(i_vs)) = (&line.variable_symbol, &invoice.variable_symbol)
                {
                    if l_vs == i_vs && !l_vs.is_empty() {
                        confidence += Decimal::from_str("0.8").unwrap();
                    }
                }

                // 2. Amount match (corroboration)
                let remaining_amount = invoice.total_amount - invoice.paid_amount;
                if line.amount == remaining_amount {
                    confidence += Decimal::from_str("0.15").unwrap();
                } else if line.amount > Decimal::ZERO && line.amount <= remaining_amount {
                    confidence += Decimal::from_str("0.05").unwrap();
                }

                // TODO: Date proximity, IBAN match

                if confidence >= threshold {
                    scored.push((invoice, confidence));
                }
            }

            // Retain only the candidates at the highest confidence (ties kept).
            let Some(best_confidence) = scored.iter().map(|(_, c)| *c).max() else {
                continue;
            };

            let mut suggested_any = false;
            for (invoice, confidence) in scored.iter().filter(|(_, c)| *c == best_confidence) {
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
        _tenant_id: Uuid,
        match_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let p_match = self
            .repo
            .find_payment_match_rls(&mut *executor, match_id)
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
        match_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let p_match = self
            .repo
            .find_payment_match_rls(&mut *executor, match_id)
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
