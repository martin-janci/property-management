//! Invoice PDF rendering (UC-ACC-05.9). OWNED BY: BE-Invoicing.
//!
//! Real implementation of the `hooks::DocumentRenderer` seam declared in
//! `routes::invoices::hooks`: renders an issued (or draft-preview) document
//! into a print-ready A4 PDF with `genpdf`, following the same font strategy
//! api-server's `render_invoice_pdf` (#975.6) uses — Liberation Sans loaded
//! from the container font directory, overridable via `ACC_PDF_FONT_DIR` for
//! dev machines where the Linux path does not exist.
//!
//! Layout: supplier/customer blocks, document metadata, line-item table, VAT
//! recap grouped by rate (UC-ACC-10.11 uses the same buckets), totals, and a
//! payment box (IBAN + variable symbol — the auto-match keys of EPIC-ACC-08).
//! Everything money-shaped comes from the STORED rounded row values (#1522
//! invariant), never recomputed here.

use db::models::acc_config::{AccBankAccount, AccCompanySettings};
use db::models::acc_invoicing_ext::AccInvoiceExt;
use db::models::accounting::{Contact, InvoiceItem};
use genpdf::{elements, style::Style, Element};
use rust_decimal::Decimal;

/// Everything the renderer needs, loaded by the handler inside one RLS
/// transaction so the document is rendered from a consistent snapshot.
pub struct InvoiceRenderContext<'a> {
    pub invoice: &'a AccInvoiceExt,
    pub items: &'a [InvoiceItem],
    pub company: Option<&'a AccCompanySettings>,
    pub contact: Option<&'a Contact>,
    pub bank_account: Option<&'a AccBankAccount>,
}

/// Default container font directory — identical to api-server's PDF renderers
/// so one Docker layer serves both binaries.
const DEFAULT_FONT_DIR: &str = "/usr/share/fonts/truetype/liberation";

/// Resolve the font directory (env override first, container default second).
pub fn font_dir() -> String {
    std::env::var("ACC_PDF_FONT_DIR").unwrap_or_else(|_| DEFAULT_FONT_DIR.to_string())
}

/// Human title for a `doc_type` value (UC-ACC-05.5/.7).
fn doc_type_title(doc_type: &str) -> &'static str {
    match doc_type {
        "proforma" => "Proforma invoice",
        "advance" => "Advance invoice",
        "credit_note" => "Credit note",
        _ => "Invoice",
    }
}

/// VAT recap: one row per distinct rate, summed from the STORED per-line
/// values so the recap always reconciles with the item table and the header
/// totals (UC-ACC-10.11). Rows are ordered by rate ascending for a stable
/// document. Pure — unit-tested without fonts.
pub fn vat_summary(items: &[InvoiceItem]) -> Vec<(Decimal, Decimal, Decimal, Decimal)> {
    let mut buckets: Vec<(Decimal, Decimal, Decimal, Decimal)> = Vec::new();
    for item in items {
        match buckets.iter_mut().find(|(rate, ..)| *rate == item.vat_rate) {
            Some((_, base, vat, total)) => {
                *base += item.base_amount;
                *vat += item.vat_amount;
                *total += item.total_amount;
            }
            None => buckets.push((
                item.vat_rate,
                item.base_amount,
                item.vat_amount,
                item.total_amount,
            )),
        }
    }
    buckets.sort_by(|a, b| a.0.cmp(&b.0));
    buckets
}

/// Filename-safe version of a document number (`2026/001` → `2026-001.pdf`).
pub fn pdf_filename(number: &str) -> String {
    let safe: String = number
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = safe.trim_matches('-');
    if trimmed.is_empty() {
        "document.pdf".to_string()
    } else {
        format!("{trimmed}.pdf")
    }
}

/// Render the document to PDF bytes. Errors are strings (mapped to 500 by the
/// handler); the by-far-most-likely failure is a missing font directory on a
/// non-container host — the error text says which path was tried.
pub fn render_invoice_pdf(ctx: &InvoiceRenderContext<'_>) -> Result<Vec<u8>, String> {
    let dir = font_dir();
    let font_family = genpdf::fonts::from_files(&dir, "LiberationSans", None)
        .map_err(|e| format!("Failed to load font LiberationSans from {dir}: {e}"))?;

    let inv = ctx.invoice;
    let title = format!("{} {}", doc_type_title(&inv.doc_type), inv.number);

    let mut doc = genpdf::Document::new(font_family);
    doc.set_title(title.clone());
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    doc.push(elements::Paragraph::new(&title).styled(Style::new().bold().with_font_size(18)));
    if inv.status == "draft" {
        doc.push(
            elements::Paragraph::new("DRAFT — not a tax document")
                .styled(Style::new().bold().with_font_size(11)),
        );
    }
    if inv.status == "cancelled" {
        doc.push(elements::Paragraph::new("CANCELLED").styled(Style::new().bold()));
    }
    doc.push(elements::Break::new(1));

    // Supplier | customer blocks side by side.
    let mut parties = elements::TableLayout::new(vec![1, 1]);
    let mut supplier = elements::LinearLayout::vertical();
    supplier.push(elements::Paragraph::new("Supplier").styled(Style::new().bold()));
    if let Some(c) = ctx.company {
        supplier.push(elements::Paragraph::new(c.legal_name.clone()));
        if let Some(a) = &c.address {
            supplier.push(elements::Paragraph::new(a.clone()));
        }
        if let Some(ico) = &c.ico {
            supplier.push(elements::Paragraph::new(format!("Reg. no (ICO): {ico}")));
        }
        if let Some(dic) = &c.dic {
            supplier.push(elements::Paragraph::new(format!("Tax id (DIC): {dic}")));
        }
        if let Some(vat) = &c.vat_id {
            supplier.push(elements::Paragraph::new(format!("VAT id: {vat}")));
        }
        if !c.vat_payer {
            supplier.push(elements::Paragraph::new("Not a VAT payer"));
        }
    }
    let mut customer = elements::LinearLayout::vertical();
    customer.push(elements::Paragraph::new("Customer").styled(Style::new().bold()));
    if let Some(c) = ctx.contact {
        customer.push(elements::Paragraph::new(c.name.clone()));
        if let Some(a) = &c.address {
            customer.push(elements::Paragraph::new(a.clone()));
        }
        if let Some(ico) = &c.ico {
            customer.push(elements::Paragraph::new(format!("Reg. no (ICO): {ico}")));
        }
        if let Some(dic) = &c.dic {
            customer.push(elements::Paragraph::new(format!("Tax id (DIC): {dic}")));
        }
    }
    parties
        .row()
        .element(supplier.padded(1))
        .element(customer.padded(1))
        .push()
        .map_err(|e| format!("parties table: {e}"))?;
    doc.push(parties);
    doc.push(elements::Break::new(1));

    // Document metadata.
    let mut meta = elements::LinearLayout::vertical();
    meta.push(elements::Paragraph::new(format!(
        "Issue date: {}",
        inv.issue_date
    )));
    meta.push(elements::Paragraph::new(format!(
        "Due date: {}",
        inv.due_date
    )));
    if let Some(tsd) = inv.taxable_supply_date {
        meta.push(elements::Paragraph::new(format!(
            "Taxable supply date: {tsd}"
        )));
    }
    if let Some(vs) = &inv.variable_symbol {
        meta.push(elements::Paragraph::new(format!("Variable symbol: {vs}")));
    }
    if let Some(rate) = inv.exchange_rate {
        meta.push(elements::Paragraph::new(format!(
            "Exchange rate: {rate} ({})",
            inv.currency
        )));
    }
    doc.push(meta);
    doc.push(elements::Break::new(1));

    // Line items.
    let mut table = elements::TableLayout::new(vec![6, 2, 3, 2, 3]);
    table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));
    table
        .row()
        .element(elements::Paragraph::new("Description").styled(Style::new().bold()))
        .element(elements::Paragraph::new("Qty").styled(Style::new().bold()))
        .element(elements::Paragraph::new("Unit price").styled(Style::new().bold()))
        .element(elements::Paragraph::new("VAT %").styled(Style::new().bold()))
        .element(elements::Paragraph::new("Total").styled(Style::new().bold()))
        .push()
        .map_err(|e| format!("item table header: {e}"))?;
    for item in ctx.items {
        table
            .row()
            .element(elements::Paragraph::new(item.description.clone()))
            .element(elements::Paragraph::new(item.qty.to_string()))
            .element(elements::Paragraph::new(format!(
                "{} {}",
                item.unit_price, inv.currency
            )))
            .element(elements::Paragraph::new(item.vat_rate.to_string()))
            .element(elements::Paragraph::new(format!(
                "{} {}",
                item.total_amount, inv.currency
            )))
            .push()
            .map_err(|e| format!("item table row: {e}"))?;
    }
    doc.push(table);
    doc.push(elements::Break::new(1));

    // VAT recap by rate (skipped entirely for non-VAT-payers).
    let vat_payer = ctx.company.map(|c| c.vat_payer).unwrap_or(true);
    if vat_payer && !ctx.items.is_empty() {
        let mut recap = elements::TableLayout::new(vec![3, 3, 3, 3]);
        recap.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));
        recap
            .row()
            .element(elements::Paragraph::new("VAT rate").styled(Style::new().bold()))
            .element(elements::Paragraph::new("Base").styled(Style::new().bold()))
            .element(elements::Paragraph::new("VAT").styled(Style::new().bold()))
            .element(elements::Paragraph::new("Total").styled(Style::new().bold()))
            .push()
            .map_err(|e| format!("vat recap header: {e}"))?;
        for (rate, base, vat, total) in vat_summary(ctx.items) {
            recap
                .row()
                .element(elements::Paragraph::new(format!("{rate} %")))
                .element(elements::Paragraph::new(format!("{base} {}", inv.currency)))
                .element(elements::Paragraph::new(format!("{vat} {}", inv.currency)))
                .element(elements::Paragraph::new(format!(
                    "{total} {}",
                    inv.currency
                )))
                .push()
                .map_err(|e| format!("vat recap row: {e}"))?;
        }
        doc.push(recap);
        doc.push(elements::Break::new(1));
    }

    // Totals — stored header values (#1522: header == Σ rounded line rows).
    let mut totals = elements::LinearLayout::vertical();
    totals.push(elements::Paragraph::new(format!(
        "Base: {} {}",
        inv.base_amount, inv.currency
    )));
    totals.push(elements::Paragraph::new(format!(
        "VAT: {} {}",
        inv.vat_amount, inv.currency
    )));
    if inv.advance_settlement != Decimal::ZERO {
        totals.push(elements::Paragraph::new(format!(
            "Advance settled: -{} {}",
            inv.advance_settlement, inv.currency
        )));
    }
    totals.push(
        elements::Paragraph::new(format!("Total: {} {}", inv.total_amount, inv.currency))
            .styled(Style::new().bold().with_font_size(12)),
    );
    if inv.paid_amount != Decimal::ZERO {
        totals.push(elements::Paragraph::new(format!(
            "Paid: {} {}",
            inv.paid_amount, inv.currency
        )));
        totals.push(
            elements::Paragraph::new(format!(
                "Amount due: {} {}",
                inv.total_amount - inv.paid_amount,
                inv.currency
            ))
            .styled(Style::new().bold()),
        );
    }
    doc.push(totals);
    doc.push(elements::Break::new(1));

    // Payment box — the auto-match keys (EPIC-ACC-08.6).
    if let Some(bank) = ctx.bank_account {
        let mut pay = elements::LinearLayout::vertical();
        pay.push(elements::Paragraph::new("Payment details").styled(Style::new().bold()));
        pay.push(elements::Paragraph::new(format!("IBAN: {}", bank.iban)));
        if let Some(swift) = &bank.swift {
            pay.push(elements::Paragraph::new(format!("SWIFT/BIC: {swift}")));
        }
        if let Some(vs) = &inv.variable_symbol {
            pay.push(elements::Paragraph::new(format!("Variable symbol: {vs}")));
        }
        doc.push(pay);
    }

    if let Some(footer) = ctx.company.and_then(|c| c.default_footer.clone()) {
        doc.push(elements::Break::new(1));
        doc.push(elements::Paragraph::new(footer).styled(Style::new().with_font_size(9)));
    }

    let mut bytes = Vec::new();
    doc.render(&mut bytes)
        .map_err(|e| format!("PDF render failed: {e}"))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    fn item(rate: Decimal, base: Decimal, vat: Decimal) -> InvoiceItem {
        InvoiceItem {
            id: Uuid::nil(),
            invoice_id: Uuid::nil(),
            tenant_id: Uuid::nil(),
            description: "x".into(),
            qty: dec!(1),
            unit_price: base,
            vat_rate: rate,
            total_amount: base + vat,
            base_amount: base,
            vat_amount: vat,
        }
    }

    #[test]
    fn vat_summary_groups_by_rate_and_sorts() {
        let items = vec![
            item(dec!(23), dec!(100), dec!(23)),
            item(dec!(5), dec!(50), dec!(2.50)),
            item(dec!(23), dec!(200), dec!(46)),
        ];
        let recap = vat_summary(&items);
        assert_eq!(recap.len(), 2);
        // Sorted ascending by rate.
        assert_eq!(recap[0], (dec!(5), dec!(50), dec!(2.50), dec!(52.50)));
        assert_eq!(recap[1], (dec!(23), dec!(300), dec!(69), dec!(369)));
    }

    #[test]
    fn vat_summary_reconciles_with_header_sums() {
        let items = vec![
            item(dec!(23), dec!(10.01), dec!(2.30)),
            item(dec!(19), dec!(7.77), dec!(1.48)),
        ];
        let recap = vat_summary(&items);
        let base: Decimal = recap.iter().map(|r| r.1).sum();
        let vat: Decimal = recap.iter().map(|r| r.2).sum();
        let total: Decimal = recap.iter().map(|r| r.3).sum();
        assert_eq!(base, items.iter().map(|i| i.base_amount).sum());
        assert_eq!(vat, items.iter().map(|i| i.vat_amount).sum());
        assert_eq!(total, items.iter().map(|i| i.total_amount).sum());
    }

    #[test]
    fn filename_is_sanitized() {
        assert_eq!(pdf_filename("2026/001"), "2026-001.pdf");
        assert_eq!(pdf_filename("FA-2026 č.7"), "FA-2026---7.pdf");
        assert_eq!(pdf_filename("///"), "document.pdf");
    }

    #[test]
    fn doc_type_titles() {
        assert_eq!(doc_type_title("invoice"), "Invoice");
        assert_eq!(doc_type_title("proforma"), "Proforma invoice");
        assert_eq!(doc_type_title("advance"), "Advance invoice");
        assert_eq!(doc_type_title("credit_note"), "Credit note");
        assert_eq!(doc_type_title("unknown"), "Invoice");
    }
}
