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
                .map_err(|e| AppError::BadRequest(format!("Invalid date format {}: {}", record.date, e)))?;

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
        self.run_payment_matcher(&mut *executor, tenant_id, statement.id).await?;

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

        for line in lines {
            if line.match_state != "unmatched" && line.match_state != "suggested" {
                continue;
            }

            for invoice in &open_invoices {
                let mut confidence = Decimal::ZERO;

                // 1. Variable Symbol match (highest weight)
                if let (Some(l_vs), Some(i_vs)) = (&line.variable_symbol, &invoice.variable_symbol) {
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

                if confidence >= Decimal::from_str("0.5").unwrap() {
                    let p_match = PaymentMatch {
                        id: Uuid::new_v4(),
                        tenant_id,
                        statement_line_id: line.id,
                        invoice_id: invoice.id,
                        confidence,
                        decided_by: None,
                        decided_at: None,
                        state: PaymentMatchState::Suggested,
                    };
                    self.repo
                        .upsert_payment_match_rls(&mut *executor, p_match)
                        .await
                        .map_err(|e| AppError::Database(e.to_string()))?;

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

        if p_match.state == PaymentMatchState::Confirmed {
            return Ok(());
        }

        let line = self
            .repo
            .find_bank_statement_line_rls(&mut *executor, p_match.statement_line_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| {
                AppError::Internal(format!("Statement line {} not found", p_match.statement_line_id))
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
            .map_err(|e| AppError::Database(e.to_string()))?;

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

        // 1. Update match state
        let mut updated_match = p_match.clone();
        updated_match.state = PaymentMatchState::Rejected;
        updated_match.decided_by = Some(user_id);
        updated_match.decided_at = Some(Utc::now());
        self.repo
            .upsert_payment_match_rls(&mut *executor, updated_match)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 2. Update statement line state if no other suggested matches remain
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
