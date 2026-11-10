//! Accounting system export services (Story 61.2).
//!
//! Supports POHODA XML and Money S3 CSV exports.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::io::Write;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AccountingError {
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Export error: {0}")]
    Export(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unsupported system: {0}")]
    UnsupportedSystem(String),
}

// ============================================
// Common Types
// ============================================

/// Invoice for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportInvoice {
    pub number: String,
    pub date: NaiveDate,
    pub due_date: NaiveDate,
    pub variable_symbol: Option<String>,
    pub partner: Partner,
    pub items: Vec<InvoiceItem>,
    pub payment_type: PaymentType,
    pub currency: String,
    pub note: Option<String>,
}

/// Invoice partner (customer/supplier).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partner {
    pub name: String,
    pub street: Option<String>,
    pub city: Option<String>,
    pub zip: Option<String>,
    pub country: Option<String>,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub ic_dph: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

/// Invoice line item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceItem {
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    pub unit_price: f64,
    pub vat_rate: VatRate,
    pub account_code: Option<String>,
    pub cost_center: Option<String>,
}

/// VAT rate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VatRate {
    /// Zero VAT (non-taxable)
    Zero,
    /// Reduced rate (10% in CZ, 10% in SK)
    Reduced,
    /// First reduced rate (15% in CZ, currently not used in SK)
    FirstReduced,
    /// Standard rate (21% in CZ, 20% in SK)
    Standard,
    /// Custom rate
    Custom(f64),
}

impl VatRate {
    /// Get the VAT percentage for Czech Republic.
    pub fn cz_percentage(&self) -> f64 {
        match self {
            VatRate::Zero => 0.0,
            VatRate::Reduced => 10.0,
            VatRate::FirstReduced => 15.0,
            VatRate::Standard => 21.0,
            VatRate::Custom(rate) => *rate,
        }
    }

    /// Get the VAT percentage for Slovakia.
    pub fn sk_percentage(&self) -> f64 {
        match self {
            VatRate::Zero => 0.0,
            VatRate::Reduced => 10.0,
            VatRate::FirstReduced => 10.0, // SK doesn't have 15% rate
            VatRate::Standard => 20.0,
            VatRate::Custom(rate) => *rate,
        }
    }
}

/// Payment type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PaymentType {
    BankTransfer,
    Cash,
    Card,
    DirectDebit,
    Other,
}

/// Payment for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPayment {
    pub date: NaiveDate,
    pub amount: f64,
    pub currency: String,
    pub invoice_number: Option<String>,
    pub variable_symbol: Option<String>,
    pub payment_type: PaymentType,
    pub bank_account: Option<String>,
    pub note: Option<String>,
}

/// Export validation result.
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        }
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.is_valid = false;
        self.errors.push(error);
        self
    }

    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }
}

// ============================================
// POHODA XML Export
// ============================================

/// POHODA XML exporter for Czech accounting software.
pub struct PohodaExporter {
    /// Company ICO (identification number).
    pub company_ico: String,
    /// Application name for header.
    pub app_name: String,
}

impl PohodaExporter {
    /// Create a new POHODA exporter.
    pub fn new(company_ico: String) -> Self {
        Self {
            company_ico,
            app_name: "PropertyManagement".to_string(),
        }
    }

    /// Validate invoices for POHODA export.
    pub fn validate_invoices(&self, invoices: &[ExportInvoice]) -> ValidationResult {
        let mut result = ValidationResult::valid();

        for (i, invoice) in invoices.iter().enumerate() {
            if invoice.number.is_empty() {
                result = result.with_error(format!("Invoice {}: number is required", i + 1));
            }

            if invoice.partner.name.is_empty() {
                result = result.with_error(format!("Invoice {}: partner name is required", i + 1));
            }

            if invoice.items.is_empty() {
                result =
                    result.with_error(format!("Invoice {}: at least one item is required", i + 1));
            }

            for (j, item) in invoice.items.iter().enumerate() {
                if item.description.is_empty() {
                    result = result.with_error(format!(
                        "Invoice {} item {}: description is required",
                        i + 1,
                        j + 1
                    ));
                }
            }

            // Warnings
            if invoice.partner.ico.is_none() && invoice.partner.dic.is_none() {
                result =
                    result.with_warning(format!("Invoice {}: partner has no ICO or DIC", i + 1));
            }
        }

        result
    }

    /// Export invoices to POHODA XML format.
    pub fn export_invoices<W: Write>(
        &self,
        writer: &mut W,
        invoices: &[ExportInvoice],
    ) -> Result<(), AccountingError> {
        let validation = self.validate_invoices(invoices);
        if !validation.is_valid {
            return Err(AccountingError::Validation(validation.errors.join("; ")));
        }

        // Write XML header
        writeln!(writer, r#"<?xml version="1.0" encoding="Windows-1250"?>"#)?;
        writeln!(
            writer,
            r#"<dat:dataPack xmlns:dat="http://www.stormware.cz/schema/version_2/data.xsd"
            xmlns:inv="http://www.stormware.cz/schema/version_2/invoice.xsd"
            xmlns:typ="http://www.stormware.cz/schema/version_2/type.xsd"
            version="2.0" id="exp001" ico="{}" application="{}" note="Property Management Export">"#,
            self.company_ico, self.app_name
        )?;

        for invoice in invoices {
            self.write_invoice(writer, invoice)?;
        }

        writeln!(writer, "</dat:dataPack>")?;

        Ok(())
    }

    fn write_invoice<W: Write>(
        &self,
        writer: &mut W,
        invoice: &ExportInvoice,
    ) -> Result<(), AccountingError> {
        writeln!(
            writer,
            r#"  <dat:dataPackItem version="2.0" id="inv_{}">"#,
            invoice.number
        )?;
        writeln!(writer, "    <inv:invoice version=\"2.0\">")?;

        // Invoice header
        writeln!(writer, "      <inv:invoiceHeader>")?;
        writeln!(
            writer,
            "        <inv:invoiceType>issuedInvoice</inv:invoiceType>"
        )?;
        writeln!(writer, "        <inv:number>")?;
        writeln!(
            writer,
            "          <typ:numberRequested>{}</typ:numberRequested>",
            escape_xml(&invoice.number)
        )?;
        writeln!(writer, "        </inv:number>")?;

        if let Some(vs) = &invoice.variable_symbol {
            writeln!(
                writer,
                "        <inv:symVar>{}</inv:symVar>",
                escape_xml(vs)
            )?;
        }

        writeln!(writer, "        <inv:date>{}</inv:date>", invoice.date)?;
        writeln!(
            writer,
            "        <inv:dateDue>{}</inv:dateDue>",
            invoice.due_date
        )?;

        // Payment type
        let payment_type = match invoice.payment_type {
            PaymentType::BankTransfer => "draft",
            PaymentType::Cash => "cash",
            PaymentType::Card => "creditCard",
            PaymentType::DirectDebit => "directDebit",
            PaymentType::Other => "other",
        };
        writeln!(
            writer,
            "        <inv:paymentType>{}</inv:paymentType>",
            payment_type
        )?;

        // Partner address
        writeln!(writer, "        <inv:partnerIdentity>")?;
        writeln!(writer, "          <typ:address>")?;
        writeln!(
            writer,
            "            <typ:company>{}</typ:company>",
            escape_xml(&invoice.partner.name)
        )?;

        if let Some(street) = &invoice.partner.street {
            writeln!(
                writer,
                "            <typ:street>{}</typ:street>",
                escape_xml(street)
            )?;
        }
        if let Some(city) = &invoice.partner.city {
            writeln!(
                writer,
                "            <typ:city>{}</typ:city>",
                escape_xml(city)
            )?;
        }
        if let Some(zip) = &invoice.partner.zip {
            writeln!(writer, "            <typ:zip>{}</typ:zip>", escape_xml(zip))?;
        }
        if let Some(ico) = &invoice.partner.ico {
            writeln!(writer, "            <typ:ico>{}</typ:ico>", escape_xml(ico))?;
        }
        if let Some(dic) = &invoice.partner.dic {
            writeln!(writer, "            <typ:dic>{}</typ:dic>", escape_xml(dic))?;
        }

        writeln!(writer, "          </typ:address>")?;
        writeln!(writer, "        </inv:partnerIdentity>")?;

        if let Some(note) = &invoice.note {
            writeln!(writer, "        <inv:note>{}</inv:note>", escape_xml(note))?;
        }

        writeln!(writer, "      </inv:invoiceHeader>")?;

        // Invoice items
        writeln!(writer, "      <inv:invoiceDetail>")?;

        for item in &invoice.items {
            writeln!(writer, "        <inv:invoiceItem>")?;
            writeln!(
                writer,
                "          <inv:text>{}</inv:text>",
                escape_xml(&item.description)
            )?;
            writeln!(
                writer,
                "          <inv:quantity>{}</inv:quantity>",
                item.quantity
            )?;
            writeln!(
                writer,
                "          <inv:unit>{}</inv:unit>",
                escape_xml(&item.unit)
            )?;
            writeln!(
                writer,
                "          <inv:rateVAT>{}</inv:rateVAT>",
                self.vat_rate_to_pohoda(&item.vat_rate)
            )?;

            // Home currency
            writeln!(writer, "          <inv:homeCurrency>")?;
            writeln!(
                writer,
                "            <typ:unitPrice>{:.2}</typ:unitPrice>",
                item.unit_price
            )?;
            writeln!(writer, "          </inv:homeCurrency>")?;

            if let Some(code) = &item.account_code {
                writeln!(writer, "          <inv:accounting>")?;
                writeln!(
                    writer,
                    "            <typ:ids>{}</typ:ids>",
                    escape_xml(code)
                )?;
                writeln!(writer, "          </inv:accounting>")?;
            }

            writeln!(writer, "        </inv:invoiceItem>")?;
        }

        writeln!(writer, "      </inv:invoiceDetail>")?;

        // Invoice summary
        writeln!(writer, "      <inv:invoiceSummary>")?;
        writeln!(
            writer,
            "        <inv:roundingDocument>math2one</inv:roundingDocument>"
        )?;
        writeln!(
            writer,
            "        <inv:roundingVAT>noneEveryRate</inv:roundingVAT>"
        )?;
        writeln!(writer, "      </inv:invoiceSummary>")?;

        writeln!(writer, "    </inv:invoice>")?;
        writeln!(writer, "  </dat:dataPackItem>")?;

        Ok(())
    }

    fn vat_rate_to_pohoda(&self, rate: &VatRate) -> &'static str {
        match rate {
            VatRate::Zero => "none",
            VatRate::Reduced => "low",
            VatRate::FirstReduced => "third",
            VatRate::Standard => "high",
            VatRate::Custom(_) => "high", // Custom rates map to standard
        }
    }

    /// Export payments to POHODA XML format.
    pub fn export_payments<W: Write>(
        &self,
        writer: &mut W,
        payments: &[ExportPayment],
    ) -> Result<(), AccountingError> {
        writeln!(writer, r#"<?xml version="1.0" encoding="Windows-1250"?>"#)?;
        writeln!(
            writer,
            r#"<dat:dataPack xmlns:dat="http://www.stormware.cz/schema/version_2/data.xsd"
            xmlns:bnk="http://www.stormware.cz/schema/version_2/bank.xsd"
            xmlns:typ="http://www.stormware.cz/schema/version_2/type.xsd"
            version="2.0" id="exp001" ico="{}" application="{}" note="Property Management Export">"#,
            self.company_ico, self.app_name
        )?;

        for (i, payment) in payments.iter().enumerate() {
            writeln!(
                writer,
                r#"  <dat:dataPackItem version="2.0" id="pay_{}">"#,
                i + 1
            )?;
            writeln!(writer, "    <bnk:bank version=\"2.0\">")?;
            writeln!(writer, "      <bnk:bankHeader>")?;
            writeln!(writer, "        <bnk:bankType>receipt</bnk:bankType>")?;
            writeln!(
                writer,
                "        <bnk:dateStatement>{}</bnk:dateStatement>",
                payment.date
            )?;

            if let Some(vs) = &payment.variable_symbol {
                writeln!(
                    writer,
                    "        <bnk:symVar>{}</bnk:symVar>",
                    escape_xml(vs)
                )?;
            }

            writeln!(writer, "        <bnk:homeCurrency>")?;
            writeln!(
                writer,
                "          <typ:priceSum>{:.2}</typ:priceSum>",
                payment.amount
            )?;
            writeln!(writer, "        </bnk:homeCurrency>")?;

            if let Some(note) = &payment.note {
                writeln!(writer, "        <bnk:note>{}</bnk:note>", escape_xml(note))?;
            }

            writeln!(writer, "      </bnk:bankHeader>")?;
            writeln!(writer, "    </bnk:bank>")?;
            writeln!(writer, "  </dat:dataPackItem>")?;
        }

        writeln!(writer, "</dat:dataPack>")?;

        Ok(())
    }
}

// ============================================
// Money S3 CSV Export
// ============================================

/// Money S3 CSV exporter for Slovak accounting software.
pub struct MoneyS3Exporter {
    /// CSV separator (semicolon for Slovak locale).
    pub separator: char,
    /// Decimal separator.
    pub decimal_separator: char,
}

impl MoneyS3Exporter {
    /// Create a new Money S3 exporter with default settings.
    pub fn new() -> Self {
        Self {
            separator: ';',
            decimal_separator: ',',
        }
    }

    /// Validate invoices for Money S3 export.
    pub fn validate_invoices(&self, invoices: &[ExportInvoice]) -> ValidationResult {
        let mut result = ValidationResult::valid();

        for (i, invoice) in invoices.iter().enumerate() {
            if invoice.number.is_empty() {
                result = result.with_error(format!("Invoice {}: number is required", i + 1));
            }

            if invoice.partner.name.is_empty() {
                result = result.with_error(format!("Invoice {}: partner name is required", i + 1));
            }

            if invoice.items.is_empty() {
                result =
                    result.with_error(format!("Invoice {}: at least one item is required", i + 1));
            }

            // Validate number format (Money S3 has specific requirements)
            if invoice.number.len() > 20 {
                result = result.with_warning(format!(
                    "Invoice {}: number exceeds 20 characters, may be truncated",
                    i + 1
                ));
            }
        }

        result
    }

    /// Export invoices to Money S3 CSV format.
    pub fn export_invoices<W: Write>(
        &self,
        writer: &mut W,
        invoices: &[ExportInvoice],
    ) -> Result<(), AccountingError> {
        let validation = self.validate_invoices(invoices);
        if !validation.is_valid {
            return Err(AccountingError::Validation(validation.errors.join("; ")));
        }

        // Write header line for invoice headers
        writeln!(
            writer,
            "Typ{}Cislo{}Datum{}DatumSplatnosti{}Firma{}ICO{}DIC{}Ulice{}Mesto{}PSC{}Celkem{}Mena{}Poznamka",
            self.separator, self.separator, self.separator, self.separator,
            self.separator, self.separator, self.separator, self.separator,
            self.separator, self.separator, self.separator, self.separator
        )?;

        for invoice in invoices {
            // Calculate total
            let total: f64 = invoice
                .items
                .iter()
                .map(|item| {
                    let subtotal = item.quantity * item.unit_price;
                    let vat = subtotal * item.vat_rate.sk_percentage() / 100.0;
                    subtotal + vat
                })
                .sum();

            writeln!(
                writer,
                "FV{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
                self.separator,
                escape_csv(&invoice.number, self.separator),
                self.separator,
                invoice.date.format("%d.%m.%Y"),
                self.separator,
                invoice.due_date.format("%d.%m.%Y"),
                self.separator,
                escape_csv(&invoice.partner.name, self.separator),
                self.separator,
                escape_csv(invoice.partner.ico.as_deref().unwrap_or(""), self.separator),
                self.separator,
                escape_csv(invoice.partner.dic.as_deref().unwrap_or(""), self.separator),
                self.separator,
                escape_csv(
                    invoice.partner.street.as_deref().unwrap_or(""),
                    self.separator
                ),
                self.separator,
                escape_csv(
                    invoice.partner.city.as_deref().unwrap_or(""),
                    self.separator
                ),
                self.separator,
                escape_csv(invoice.partner.zip.as_deref().unwrap_or(""), self.separator),
                self.separator,
                self.format_number(total),
                self.separator,
                invoice.currency,
                self.separator,
                escape_csv(invoice.note.as_deref().unwrap_or(""), self.separator),
            )?;
        }

        Ok(())
    }

    /// Export invoice items to Money S3 CSV format.
    pub fn export_invoice_items<W: Write>(
        &self,
        writer: &mut W,
        invoices: &[ExportInvoice],
    ) -> Result<(), AccountingError> {
        // Write header for items
        writeln!(
            writer,
            "CisloFaktury{}Popis{}Mnozstvi{}MJ{}CenaJednotka{}SazbaDPH{}CelkemBezDPH{}DPH{}Stredisko{}Ucet",
            self.separator, self.separator, self.separator, self.separator,
            self.separator, self.separator, self.separator, self.separator,
            self.separator
        )?;

        for invoice in invoices {
            for item in &invoice.items {
                let vat_rate = item.vat_rate.sk_percentage();
                let subtotal = item.quantity * item.unit_price;
                let vat = subtotal * vat_rate / 100.0;

                writeln!(
                    writer,
                    "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
                    escape_csv(&invoice.number, self.separator),
                    self.separator,
                    escape_csv(&item.description, self.separator),
                    self.separator,
                    self.format_number(item.quantity),
                    self.separator,
                    escape_csv(&item.unit, self.separator),
                    self.separator,
                    self.format_number(item.unit_price),
                    self.separator,
                    self.format_number(vat_rate),
                    self.separator,
                    self.format_number(subtotal),
                    self.separator,
                    self.format_number(vat),
                    self.separator,
                    escape_csv(item.cost_center.as_deref().unwrap_or(""), self.separator),
                    self.separator,
                    escape_csv(item.account_code.as_deref().unwrap_or(""), self.separator),
                )?;
            }
        }

        Ok(())
    }

    /// Export payments to Money S3 CSV format.
    pub fn export_payments<W: Write>(
        &self,
        writer: &mut W,
        payments: &[ExportPayment],
    ) -> Result<(), AccountingError> {
        // Write header
        writeln!(
            writer,
            "Typ{}Datum{}Castka{}Mena{}CisloFaktury{}VariabilniSymbol{}TypPlatby{}Ucet{}Poznamka",
            self.separator,
            self.separator,
            self.separator,
            self.separator,
            self.separator,
            self.separator,
            self.separator,
            self.separator
        )?;

        for payment in payments {
            let payment_type = match payment.payment_type {
                PaymentType::BankTransfer => "Prevod",
                PaymentType::Cash => "Hotovost",
                PaymentType::Card => "Karta",
                PaymentType::DirectDebit => "Inkaso",
                PaymentType::Other => "Ine",
            };

            writeln!(
                writer,
                "Prijem{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
                self.separator,
                payment.date.format("%d.%m.%Y"),
                self.separator,
                self.format_number(payment.amount),
                self.separator,
                payment.currency,
                self.separator,
                escape_csv(
                    payment.invoice_number.as_deref().unwrap_or(""),
                    self.separator
                ),
                self.separator,
                escape_csv(
                    payment.variable_symbol.as_deref().unwrap_or(""),
                    self.separator
                ),
                self.separator,
                payment_type,
                self.separator,
                escape_csv(
                    payment.bank_account.as_deref().unwrap_or(""),
                    self.separator
                ),
                self.separator,
                escape_csv(payment.note.as_deref().unwrap_or(""), self.separator),
            )?;
        }

        Ok(())
    }

    fn format_number(&self, value: f64) -> String {
        let formatted = format!("{:.2}", value);
        if self.decimal_separator == ',' {
            formatted.replace('.', ",")
        } else {
            formatted
        }
    }
}

impl Default for MoneyS3Exporter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================
// Helper Functions
// ============================================

/// Escape XML special characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Escape CSV field.
fn escape_csv(s: &str, separator: char) -> String {
    if s.contains(separator) || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_invoice() -> ExportInvoice {
        ExportInvoice {
            number: "FV2024001".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            due_date: NaiveDate::from_ymd_opt(2024, 2, 15).unwrap(),
            variable_symbol: Some("2024001".to_string()),
            partner: Partner {
                name: "Test Company s.r.o.".to_string(),
                street: Some("Test Street 123".to_string()),
                city: Some("Prague".to_string()),
                zip: Some("11000".to_string()),
                country: Some("CZ".to_string()),
                ico: Some("12345678".to_string()),
                dic: Some("CZ12345678".to_string()),
                ic_dph: None,
                email: Some("test@example.com".to_string()),
                phone: None,
            },
            items: vec![InvoiceItem {
                description: "Property management fee".to_string(),
                quantity: 1.0,
                unit: "month".to_string(),
                unit_price: 1000.0,
                vat_rate: VatRate::Standard,
                account_code: Some("602001".to_string()),
                cost_center: Some("001".to_string()),
            }],
            payment_type: PaymentType::BankTransfer,
            currency: "CZK".to_string(),
            note: Some("Monthly management fee".to_string()),
        }
    }

    #[test]
    fn test_pohoda_export_validation() {
        let exporter = PohodaExporter::new("12345678".to_string());
        let invoice = sample_invoice();

        let result = exporter.validate_invoices(&[invoice]);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_pohoda_export_invalid() {
        let exporter = PohodaExporter::new("12345678".to_string());
        let mut invoice = sample_invoice();
        invoice.number = "".to_string();

        let result = exporter.validate_invoices(&[invoice]);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_pohoda_xml_export() {
        let exporter = PohodaExporter::new("12345678".to_string());
        let invoice = sample_invoice();

        let mut output = Vec::new();
        exporter.export_invoices(&mut output, &[invoice]).unwrap();

        let xml = String::from_utf8(output).unwrap();
        assert!(xml.contains("<?xml version"));
        assert!(xml.contains("FV2024001"));
        assert!(xml.contains("Test Company s.r.o."));
    }

    #[test]
    fn test_money_s3_export_validation() {
        let exporter = MoneyS3Exporter::new();
        let invoice = sample_invoice();

        let result = exporter.validate_invoices(&[invoice]);
        assert!(result.is_valid);
    }

    #[test]
    fn test_money_s3_csv_export() {
        let exporter = MoneyS3Exporter::new();
        let invoice = sample_invoice();

        let mut output = Vec::new();
        exporter.export_invoices(&mut output, &[invoice]).unwrap();

        let csv = String::from_utf8(output).unwrap();
        assert!(csv.contains("FV2024001"));
        assert!(csv.contains("Test Company s.r.o."));
    }

    #[test]
    fn test_vat_rates() {
        assert_eq!(VatRate::Standard.cz_percentage(), 21.0);
        assert_eq!(VatRate::Standard.sk_percentage(), 20.0);
        assert_eq!(VatRate::Reduced.cz_percentage(), 10.0);
        assert_eq!(VatRate::Zero.cz_percentage(), 0.0);
    }
}
