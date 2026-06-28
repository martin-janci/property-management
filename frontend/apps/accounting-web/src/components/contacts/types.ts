/**
 * TypeScript shapes for the accounting-server contacts surface (EPIC-ACC-03).
 *
 * Mirrors the serde JSON of the Rust models:
 *  - GET /contacts, GET /contacts/{id}, POST/PATCH /contacts → `AccContactExt`
 *  - request body → `ContactRequest`
 *
 * CONTRACT NOTE: replaceable by `@ppt/accounting-api-client` generated types in
 * Phase 2. See contract-notes/fe-screens.md.
 */

export type ContactKind = 'customer' | 'supplier' | 'both';

/** Extended contact projection — db::models::acc_contacts_ext::AccContactExt. */
export interface AccContactExt {
  id: string;
  tenant_id: string;
  name: string;
  ico: string | null;
  dic: string | null;
  email: string | null;
  phone: string | null;
  address: string | null;
  iban: string | null;
  created_at: string;
  updated_at: string;
  // 00195 extension columns
  kind: string;
  is_active: boolean;
  default_currency: string | null;
  default_price_level: string | null;
  default_payment_terms_days: number | null;
  default_discount_pct: string | null;
  default_language: string | null;
  vat_verified_at: string | null;
  vat_verified_valid: boolean | null;
  credit_limit: string | null;
  notes: string | null;
  merged_into_id: string | null;
}

/** Create/update payload — accounting-server routes::contacts::ContactRequest. */
export interface ContactRequest {
  name: string;
  kind: string;
  ico?: string | null;
  dic?: string | null;
  email?: string | null;
  phone?: string | null;
  address?: string | null;
  iban?: string | null;
  default_currency?: string | null;
  default_price_level?: string | null;
  default_payment_terms_days?: number | null;
  default_language?: string | null;
}
