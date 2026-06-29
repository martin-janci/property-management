/**
 * Hand-typed fallback types for @ppt/accounting-api-client.
 *
 * These mirror the accounting-server REST surface (CONTRACT §5e) and the db
 * models (CONTRACT §6) so accounting-web can typecheck BEFORE the generated
 * hey-api client exists. Phase 2 generates `./generated` from the server's
 * OpenAPI; `src/index.ts` then re-exports the generated types and these become
 * the legacy fallback. Field names follow serde's emitted casing (snake_case,
 * matching the Rust db models).
 *
 * Covers UC-ACC-03 (contacts) and UC-ACC-05 (sales invoicing) — the screens
 * shipped in the MVP.
 */

// ── Shared scalars ──────────────────────────────────────────────────────────
/** ISO-8601 timestamp string (serde RFC3339). */
export type Timestamp = string;
/** Decimal serialized as a string to avoid float precision loss. */
export type DecimalString = string;
export type Uuid = string;

// ── Contacts (UC-ACC-03) ─────────────────────────────────────────────────────
export type ContactKind = 'customer' | 'supplier' | 'both';

/**
 * Contact projection as emitted by accounting-server (base `contact` columns
 * plus the 00195 extension columns — see AccContactExt in CONTRACT §6).
 */
export interface Contact {
  id: Uuid;
  tenant_id: Uuid;
  name: string;
  email?: string | null;
  phone?: string | null;
  ico?: string | null;
  dic?: string | null;
  vat_id?: string | null;
  // 00195 extension columns
  kind: ContactKind;
  is_active: boolean;
  default_currency?: string | null;
  default_price_level?: string | null;
  default_payment_terms_days?: number | null;
  default_discount_pct?: DecimalString | null;
  default_language?: string | null;
  vat_verified_at?: Timestamp | null;
  vat_verified_valid?: boolean | null;
  credit_limit?: DecimalString | null;
  notes?: string | null;
  merged_into_id?: Uuid | null;
  created_at: Timestamp;
  updated_at: Timestamp;
}

/** POST /contacts and PATCH /contacts/{id} body (matches `ContactRequest`). */
export interface ContactRequest {
  name: string;
  email?: string | null;
  phone?: string | null;
  ico?: string | null;
  dic?: string | null;
  vat_id?: string | null;
  kind?: ContactKind;
  is_active?: boolean;
  default_currency?: string | null;
  default_payment_terms_days?: number | null;
  default_language?: string | null;
  notes?: string | null;
}

// ── Invoices (UC-ACC-05) ─────────────────────────────────────────────────────
export type InvoiceStatus =
  | 'draft'
  | 'issued'
  | 'sent'
  | 'paid'
  | 'partially_paid'
  | 'overdue'
  | 'cancelled';

export type DocType = 'invoice' | 'proforma' | 'advance' | 'credit_note';

/** A single invoice line. */
export interface InvoiceItem {
  id: Uuid;
  invoice_id: Uuid;
  description: string;
  quantity: DecimalString;
  unit_price: DecimalString;
  vat_rate: DecimalString;
  line_total?: DecimalString | null;
  created_at?: Timestamp;
  updated_at?: Timestamp;
}

/**
 * Invoice projection (base `invoice` columns + 00196 extension — see
 * AccInvoiceExt in CONTRACT §6). `status` is emitted as a string.
 */
export interface Invoice {
  id: Uuid;
  tenant_id: Uuid;
  contact_id?: Uuid | null;
  number?: string | null;
  status: InvoiceStatus;
  currency: string;
  issue_date?: Timestamp | null;
  due_date?: Timestamp | null;
  subtotal?: DecimalString | null;
  vat_total?: DecimalString | null;
  total?: DecimalString | null;
  note?: string | null;
  // 00196 extension columns
  doc_type: DocType;
  corrected_invoice_id?: Uuid | null;
  advance_settlement?: DecimalString | null;
  settled_advance_id?: Uuid | null;
  exchange_rate?: DecimalString | null;
  base_currency_amount?: DecimalString | null;
  numbering_series_id?: Uuid | null;
  bank_account_id?: Uuid | null;
  created_at: Timestamp;
  updated_at: Timestamp;
}

/** A line on a create/update invoice request (matches `CreateInvoiceItem`). */
export interface InvoiceItemInput {
  description: string;
  quantity: DecimalString | number;
  unit_price: DecimalString | number;
  vat_rate: DecimalString | number;
}

/** POST /invoices body (matches `CreateInvoice` from db::models::accounting). */
export interface CreateInvoice {
  contact_id?: Uuid | null;
  currency?: string;
  issue_date?: string | null;
  due_date?: string | null;
  note?: string | null;
  doc_type?: DocType;
  items: InvoiceItemInput[];
}

/** PATCH /invoices/{id} body (matches `UpdateInvoice`). */
export interface UpdateInvoice {
  contact_id?: Uuid | null;
  currency?: string;
  issue_date?: string | null;
  due_date?: string | null;
  note?: string | null;
  items?: InvoiceItemInput[];
}

/** Full invoice detail (invoice + its items), the GET /invoices/{id} shape. */
export interface InvoiceWithItems extends Invoice {
  items?: InvoiceItem[];
}
