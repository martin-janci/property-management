//! Tests for accounting repository (PAP-208).

#[cfg(test)]
mod tests {
    use crate::models::accounting::{Contact, CreateInvoice, CreateInvoiceItem, InvoiceStatus};
    use crate::repositories::accounting::AccountingRepository;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use sqlx::Row;
    use std::str::FromStr;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_create_invoice_with_totals(pool: sqlx::PgPool) {
        let repo = AccountingRepository::new(pool.clone());

        // Setup: Create an organization
        let org_id: uuid::Uuid =
            sqlx::query("INSERT INTO organizations (name, slug) VALUES ($1, $2) RETURNING id")
                .bind("Test Org")
                .bind("test-org")
                .fetch_one(&pool)
                .await
                .unwrap()
                .get("id");

        let tenant_id = org_id;

        // Setup: Create a contact
        let contact: Contact = sqlx::query_as::<_, Contact>(
            "INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING *",
        )
        .bind(tenant_id)
        .bind("Test Contact")
        .fetch_one(&pool)
        .await
        .unwrap();

        let create_data = CreateInvoice {
            tenant_id,
            contact_id: contact.id,
            number: "INV-2026-001".to_string(),
            issue_date: NaiveDate::from_ymd_opt(2026, 6, 13).unwrap(),
            due_date: NaiveDate::from_ymd_opt(2026, 6, 27).unwrap(),
            taxable_supply_date: Some(NaiveDate::from_ymd_opt(2026, 6, 13).unwrap()),
            currency: "CZK".to_string(),
            variable_symbol: Some("2026001".to_string()),
            status: Some(InvoiceStatus::Draft),
            country: Some("SK".to_string()),
            items: vec![
                CreateInvoiceItem {
                    description: "Consulting".to_string(),
                    qty: Decimal::from(10),
                    unit_price: Decimal::from(100),
                    vat_rate: Decimal::ZERO, // Should be overridden by vat_rate_type
                    vat_rate_type: Some(crate::models::accounting::VatRate::Standard),
                },
                CreateInvoiceItem {
                    description: "Hardware".to_string(),
                    qty: Decimal::from(1),
                    unit_price: Decimal::from(500),
                    vat_rate: Decimal::from(10),
                    vat_rate_type: None,
                },
            ],
        };

        let mut conn = pool.acquire().await.unwrap();
        let invoice = repo
            .create_invoice_rls(&mut *conn, create_data)
            .await
            .unwrap();

        // SK Standard is 20%
        // 10 * 100 = 1000 base, 200 VAT, 1200 total
        // 1 * 500 = 500 base, 50 VAT, 550 total
        // Grand totals: 1500 base, 250 VAT, 1750 total

        assert_eq!(invoice.base_amount, Decimal::from_str("1500.00").unwrap());
        assert_eq!(invoice.vat_amount, Decimal::from_str("250.00").unwrap());
        assert_eq!(invoice.total_amount, Decimal::from_str("1750.00").unwrap());
        assert_eq!(invoice.number, "INV-2026-001");
        assert_eq!(invoice.status, InvoiceStatus::Draft);

        // Verify items
        let items = repo
            .find_invoice_items_rls(&pool, invoice.id)
            .await
            .unwrap();
        assert_eq!(items.len(), 2);

        let consulting = items
            .iter()
            .find(|i| i.description == "Consulting")
            .unwrap();
        assert_eq!(consulting.vat_rate, Decimal::from(20));
        assert_eq!(
            consulting.base_amount,
            Decimal::from_str("1000.00").unwrap()
        );
        assert_eq!(consulting.vat_amount, Decimal::from_str("200.00").unwrap());
        assert_eq!(
            consulting.total_amount,
            Decimal::from_str("1200.00").unwrap()
        );
    }
}
