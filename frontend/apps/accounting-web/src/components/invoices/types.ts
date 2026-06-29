/**
 * TypeScript shapes for the accounting-server invoices surface (EPIC-ACC-05).
 *
 * These mirror the serde JSON emitted by the Rust db models the routes return:
 *  - GET /invoices, GET /invoices/{id}, POST .../issue → `AccInvoiceExt`
 *  - POST /invoices, PATCH /invoices/{id}, POST .../duplicate → base `Invoice`
 *  - GET /invoices/{id}/items → `InvoiceItem[]`
 *  - request bodies → `CreateInvoice` / `UpdateInvoice` / `IssueInvoiceRequest`
 *
 * Field names match the Rust struct fields (serde default = snake_case for the
 * extension projections; the base models serialize their snake_case field
 * names verbatim). Decimal/NaiveDate/DateTime all serialize as strings.
 *
 * CONTRACT NOTE: when `@ppt/accounting-api-client` is generated (FE-Scaffold),
 * Phase 2 may replace this module with the generated types. See
 * contract-notes/fe-screens.md.
 */

export type InvoiceStatusValue =
  | 'draft'
  | 'issued'
  | 'sent'
  | 'paid'
  | 'partially_paid'
  | 'overdue'
  | 'cancelled';

/** Named VAT rate type accepted by `CreateInvoiceItem.vat_rate_type`. */
export type VatRateType = 'zero' | 'reduced' | 'first_reduced' | 'standard';

/** Base issued invoice — db::models::accounting::Invoice. */
export interface Invoice {
  id: string;
  tenant_id: string;
  contact_id: string;
  number: string;
  issue_date: string;
  due_date: string;
  taxable_supply_date: string | null;
  currency: string;
  total_amount: string;
  base_amount: string;
  vat_amount: string;
  variable_symbol: string | null;
  status: InvoiceStatusValue;
  paid_amount: string;
  created_at: string;
  updated_at: string;
}

/** Extended invoice projection — db::models::acc_invoicing_ext::AccInvoiceExt. */
export interface AccInvoiceExt {
  id: string;
  tenant_id: string;
  contact_id: string;
  number: string;
  issue_date: string;
  due_date: string;
  taxable_supply_date: string | null;
  currency: string;
  total_amount: string;
  base_amount: string;
  vat_amount: string;
  variable_symbol: string | null;
  /** Status as String on the ext projection (relaxed CHECK). */
  status: string;
  paid_amount: string;
  created_at: string;
  updated_at: string;
  // 00196 extension columns
  doc_type: string;
  corrected_invoice_id: string | null;
  advance_settlement: string;
  settled_advance_id: string | null;
  exchange_rate: string | null;
  base_currency_amount: string | null;
  numbering_series_id: string | null;
  bank_account_id: string | null;
}

/** Line item — db::models::accounting::InvoiceItem. */
export interface InvoiceItem {
  id: string;
  invoice_id: string;
  tenant_id: string;
  description: string;
  qty: string;
  unit_price: string;
  vat_rate: string;
  total_amount: string;
  base_amount: string;
  vat_amount: string;
}

/** Create payload for a line — db::models::accounting::CreateInvoiceItem. */
export interface CreateInvoiceItem {
  description: string;
  qty: string;
  unit_price: string;
  vat_rate: string;
  vat_rate_type?: VatRateType | null;
}

/** Create payload for an invoice — db::models::accounting::CreateInvoice. */
export interface CreateInvoice {
  tenant_id: string;
  contact_id: string;
  number: string;
  issue_date: string;
  due_date: string;
  taxable_supply_date?: string | null;
  currency: string;
  variable_symbol?: string | null;
  status?: InvoiceStatusValue | null;
  items: CreateInvoiceItem[];
  country?: string | null;
}

/** Update payload — db::models::accounting::UpdateInvoice. */
export interface UpdateInvoice {
  contact_id?: string;
  number?: string;
  issue_date?: string;
  due_date?: string;
  taxable_supply_date?: string | null;
  currency?: string;
  variable_symbol?: string | null;
  status?: InvoiceStatusValue;
  paid_amount?: string;
}

/** Issue request — accounting-server routes::invoices::IssueInvoiceRequest. */
export interface IssueInvoiceRequest {
  numbering_series_id?: string | null;
}
