# ACC Invoicing/Accounting MVP — Build CONTRACT (Phase 0b)

This file is the source of truth for the 7 parallel coders. The full skeleton
**compiles green** with every handler / domain fn / repo method a `todo!()`
stub. Each leaf file is owned by **exactly one** coder. Shared/wiring files,
migrations, and db model structs are **Foundation-owned** — coders MUST NOT edit
them.

**Build command (Verify/Foundation only):**
```
cd /mnt/sda4/projects/github.com/martin-janci/property-management/.claude/worktrees/acc-mvp/backend && \
CARGO_TARGET_DIR=/mnt/sda4/projects/github.com/martin-janci/property-management/.claude/worktrees/acc-mvp/.cargo-target-acc \
cargo build -p db -p accounting-core -p accounting-server
```

---

## 0. THE RULE (every coder)

1. **Edit ONLY the files in your OWNED FILE LIST below.** Never touch a file
   owned by another role or a shared/wiring file (lib.rs, mod.rs, main.rs,
   state.rs, Cargo.toml, migrations, db model structs).
2. **Code against this CONTRACT, not the compiler.** You may NOT run any
   build/compile/test/clippy/sqlx/migrate/git command. Phase 2 Verify builds.
3. If you need a contract/shared-type/new-column change, **DO NOT edit a shared
   file**. Append the request to `contract-notes/<your-role>.md` and code around
   it (add per-epic types in your OWN files). Phase 2 reconciles.
4. Replace `todo!()` bodies in your files with real implementations; keep the
   public signatures stable. Cite `UC-ACC-XX.Y` in code comments.
5. There is **NO sqlx offline metadata** in this repo: wrong column names
   compile and pass CI. Verify every column against the migrations
   (00184 + 00194/00195/00196) — exact names below.

---

## 1. Module / File Map

### Backend crates
```
backend/crates/db/
  migrations/00194_acc_company_config.sql        # Foundation
  migrations/00195_acc_catalog_contacts_ext.sql  # Foundation
  migrations/00196_acc_invoicing_platform.sql    # Foundation
  src/models/mod.rs                              # Foundation (wiring)
  src/models/acc_config.rs                       # Foundation (structs)
  src/models/acc_catalog.rs                      # Foundation (structs)
  src/models/acc_contacts_ext.rs                 # Foundation (structs)
  src/models/acc_invoicing_ext.rs                # Foundation (structs)
  src/models/acc_platform.rs                     # Foundation (structs)
  src/repositories/mod.rs                        # Foundation (wiring)
  src/repositories/acc_accounts.rs               # BE-Accounts
  src/repositories/acc_config.rs                 # BE-Config
  src/repositories/acc_contacts.rs               # BE-Contacts+Catalog
  src/repositories/acc_catalog.rs                # BE-Contacts+Catalog
  src/repositories/acc_invoicing.rs              # BE-Invoicing
  src/repositories/acc_platform.rs               # BE-Platform

backend/crates/accounting-core/
  src/lib.rs            # Foundation (module decls)
  src/errors.rs         # Foundation (stable)
  src/types.rs          # Foundation (stable shared value types)
  src/money.rs          # BE-Invoicing
  src/vat.rs            # BE-Invoicing
  src/rounding.rs       # BE-Invoicing
  src/numbering.rs      # BE-Config
  src/authz.rs          # BE-Platform  (CROSS-CUTTING — everyone calls)
  src/audit.rs          # BE-Platform  (CROSS-CUTTING — mutating handlers call)

backend/servers/accounting-server/
  src/main.rs           # Foundation (OpenApi paths/components, router nest)
  src/lib.rs            # Foundation
  src/state.rs          # Foundation (AppState holds all repos)
  src/observability.rs  # Foundation
  src/routes/mod.rs     # Foundation (api_router() nesting)
  src/routes/health.rs  # Foundation
  src/routes/accounts.rs# BE-Accounts
  src/routes/config.rs  # BE-Config
  src/routes/contacts.rs# BE-Contacts+Catalog
  src/routes/catalog.rs # BE-Contacts+Catalog
  src/routes/invoices.rs# BE-Invoicing
  src/routes/platform.rs# BE-Platform
```

### Frontend
```
frontend/packages/accounting-api-client/**       # FE-Scaffold (NEW package)
frontend/apps/accounting-web/
  src/lib/**, src/providers/**, src/config/**     # FE-Scaffold (infra)
  src/app/[locale]/(app)/**                       # FE-Scaffold owns the (app) group LAYOUT;
                                                  #   FE-Screens owns the per-screen page.tsx files (see §5e)
  package.json                                    # FE-Scaffold (dep add only)
  src/app/[locale]/(app)/invoices/**              # FE-Screens
  src/app/[locale]/(app)/contacts/**              # FE-Screens
  src/components/invoices/**, src/components/contacts/**  # FE-Screens
  messages/sk.json, messages/cs.json              # FE-Screens (ADD keys; do not remove existing)
```

---

## 2. Per-role OWNED FILE LIST (exact absolute paths)

> Worktree root: `/mnt/sda4/projects/github.com/martin-janci/property-management/.claude/worktrees/acc-mvp`

### BE-Accounts (EPIC-ACC-01 — accounts, orgs & access)
- `…/backend/crates/db/src/repositories/acc_accounts.rs`
- `…/backend/servers/accounting-server/src/routes/accounts.rs`

### BE-Config (EPIC-ACC-02 — company & document config + numbering)
- `…/backend/crates/db/src/repositories/acc_config.rs`
- `…/backend/crates/accounting-core/src/numbering.rs`
- `…/backend/servers/accounting-server/src/routes/config.rs`

### BE-Contacts+Catalog (EPIC-ACC-03 + EPIC-ACC-04)
- `…/backend/crates/db/src/repositories/acc_contacts.rs`
- `…/backend/crates/db/src/repositories/acc_catalog.rs`
- `…/backend/servers/accounting-server/src/routes/contacts.rs`
- `…/backend/servers/accounting-server/src/routes/catalog.rs`

### BE-Invoicing (EPIC-ACC-05 — sales invoicing + money/VAT/rounding)
- `…/backend/crates/db/src/repositories/acc_invoicing.rs`
- `…/backend/crates/accounting-core/src/money.rs`
- `…/backend/crates/accounting-core/src/vat.rs`
- `…/backend/crates/accounting-core/src/rounding.rs`
- `…/backend/servers/accounting-server/src/routes/invoices.rs`

### BE-Platform (EPIC-ACC-16 — security/audit/sharing/2FA + cross-cutting authz/audit)
- `…/backend/crates/db/src/repositories/acc_platform.rs`
- `…/backend/crates/accounting-core/src/authz.rs`   ← CROSS-CUTTING (keep signatures STABLE)
- `…/backend/crates/accounting-core/src/audit.rs`   ← CROSS-CUTTING (keep signatures STABLE)
- `…/backend/servers/accounting-server/src/routes/platform.rs`

### FE-Scaffold (SDK package + accounting-web infra)
- `…/frontend/packages/accounting-api-client/**` (NEW — mirror `reality-api-client`)
- `…/frontend/apps/accounting-web/src/lib/**`
- `…/frontend/apps/accounting-web/src/providers/**`
- `…/frontend/apps/accounting-web/src/config/**` (api base URL / env, e.g. `NEXT_PUBLIC_ACCOUNTING_API_URL`)
- `…/frontend/apps/accounting-web/src/app/[locale]/(app)/layout.tsx` (authed shell layout only)
- `…/frontend/apps/accounting-web/package.json` (ADD `@ppt/accounting-api-client` dep only)

### FE-Screens (authenticated invoice/contacts screens + i18n keys)
- `…/frontend/apps/accounting-web/src/app/[locale]/(app)/invoices/**` (page.tsx files)
- `…/frontend/apps/accounting-web/src/app/[locale]/(app)/contacts/**` (page.tsx files)
- `…/frontend/apps/accounting-web/src/components/invoices/**`
- `…/frontend/apps/accounting-web/src/components/contacts/**`
- `…/frontend/apps/accounting-web/messages/sk.json` (ADD keys under a new top-level namespace, e.g. `"app"`; never delete existing marketing/signup keys)
- `…/frontend/apps/accounting-web/messages/cs.json` (same)

**FE boundary note:** FE-Scaffold creates the `(app)` route GROUP and its
`layout.tsx`; FE-Screens creates the page/component files INSIDE `invoices/` and
`contacts/`. They never edit each other's files. If FE-Scaffold's SDK is not yet
generated, FE-Screens codes against the typed surface described in §5 and the
SDK package name `@ppt/accounting-api-client`; integration happens in Phase 2.

---

## 3. CROSS-CUTTING signatures coders MUST CALL

These live in `accounting-core` and are owned by BE-Platform. Signatures are
FIXED — if you need a change, write to `contract-notes/be-platform.md`.

### `accounting_core::authz`  (every route handler calls)
```rust
use common::TenantRole;
use accounting_core::errors::Result; // = std::result::Result<T, AccountingError>

pub fn require_role(actor: TenantRole, needed: TenantRole) -> Result<()>;
pub fn require_role_opt(actor: Option<TenantRole>, needed: TenantRole) -> Result<()>;
pub fn is_admin(actor: TenantRole) -> bool;

pub const MUTATE_MIN_ROLE: TenantRole; // = TenantRole::Manager
pub const READ_MIN_ROLE:   TenantRole; // = TenantRole::Manager
```
Handlers obtain the role from `rls.role()` (a `common::TenantRole`) or
`auth.role: Option<TenantRole>`. Map `Err(AccountingError::Unauthorized(_))` to
`StatusCode::FORBIDDEN`. (You may also keep the inline `rls.has_role(...)` gate
the embedded MVP used — both are acceptable; prefer `authz::require_role` for
consistency.)

### `accounting_core::audit`  (mutating handlers call)
```rust
use db::models::acc_platform::AccAuditLog;
use accounting_core::errors::Result;

pub struct AuditBuilder { /* … */ }
impl AuditBuilder {
    pub fn new(tenant_id: Uuid, actor_id: Option<Uuid>,
               action: impl Into<String>, entity_type: impl Into<String>) -> Self;
    pub fn entity_id(self, id: Uuid) -> Self;
    pub fn diff(self, diff: serde_json::Value) -> Self;
    pub fn build(self) -> AccAuditLog;
}
pub fn record(tenant_id: Uuid, actor_id: Option<Uuid>,
              action: impl Into<String>, entity_type: impl Into<String>,
              entity_id: Option<Uuid>, diff: serde_json::Value) -> Result<AccAuditLog>;
```
A mutating handler builds the entry, then persists it via
`state.platform_repo.record_audit_rls(&mut **rls, entry)` in the SAME
transaction as the mutation (UC-ACC-16.7 append-only).

### Numbering allocation (gapless, UC-ACC-02.3)
Atomic sequence allocation is in the DB repo, NOT in `accounting-core`:
```rust
// db::repositories::acc_config::AccConfigRepository (BE-Config)
pub async fn allocate_next_number_rls<'e, E>(&self, executor: E,
    doc_type: &str, year: i32) -> Result<i64, sqlx::Error>;
```
Then format the printed number purely:
```rust
// accounting_core::numbering (BE-Config)
pub fn format_number(format: &str, parts: &NumberParts<'_>) -> Result<String>;
```
BE-Invoicing's `issue_invoice` handler calls `allocate_next_number_rls` then
`numbering::format_number` then `AccInvoicingRepository::issue_invoice_rls`,
ALL inside one transaction (`rls.begin()` → `tx.commit()`), so concurrent
issuance can never skip or duplicate a number.

### accounting-core money/VAT/rounding (BE-Invoicing owns; callable by others)
```rust
use accounting_core::types::{LineInput, Money, VatGroup, RoundingMode};
money::compute_line(line: &LineInput, mode: RoundingMode) -> Result<Money>;
money::compute_document_total(lines: &[LineInput], mode: RoundingMode) -> Result<Money>;
money::net_from_gross(gross, vat_rate, mode) -> Decimal;
money::gross_from_net(net, vat_rate, mode) -> Decimal;
vat::vat_for_base(base, vat_rate, mode) -> Decimal;
vat::group_by_rate(lines: &[LineInput], mode: RoundingMode) -> Result<Vec<VatGroup>>;
vat::reverse_groups(groups: &[VatGroup]) -> Vec<VatGroup>;
rounding::round(value, dp, mode) -> Decimal;
rounding::round_money(value, mode) -> Decimal;
rounding::rounding_adjustment(total, mode) -> Decimal;
```

---

## 4. Tenant / RLS pattern + REUSE notes

### Tenant model
ACC "company/agenda" **IS** the existing `organizations` tenant. New tables
follow the 00184 pattern EXACTLY:
```sql
tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
ENABLE ROW LEVEL SECURITY; FORCE ROW LEVEL SECURITY;
CREATE POLICY <t>_tenant_isolation ON <t>
  FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());
```
Multi-company switching = active-org selection; accountant access = org
membership. Do NOT invent a separate identity/DB system.

### Repository pattern (generic executor — MIRROR `repositories/accounting.rs`)
Every RLS-aware method takes the connection generically:
```rust
pub async fn list_x_rls<'e, E>(&self, executor: E) -> Result<Vec<X>, sqlx::Error>
where E: Executor<'e, Database = Postgres> { /* sqlx::query_as ... */ }
```
For multi-statement work (create invoice + items, issue, merge), take
`conn: &mut sqlx::PgConnection` / a transaction (see
`AccountingRepository::create_invoice_rls`). RLS context is already set on the
`RlsConnection`; the policy filters rows by `get_current_org_id()` — your
`SELECT` does NOT add a `tenant_id = $` clause (RLS does it), but your `INSERT`
MUST set `tenant_id` to the request tenant.

### Handler pattern (MIRROR api-server `routes/accounting/invoices.rs`)
```rust
pub async fn handler(
    State(state): State<AppState>,
    mut rls: RlsConnection,           // validates JWT + tenant membership, sets RLS context
    auth: AuthUser,                   // user_id, tenant_id: Option<Uuid>, role: Option<TenantRole>
    Json(req): Json<Dto>,
) -> Result<Json<T>, StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let out = state.<repo>.<method>_rls(&mut **rls, ...).await
        .map_err(|e| { tracing::error!("…: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
    rls.release().await;              // ALWAYS release before returning
    Ok(Json(out))
}
```
- `AuthUser` extractor: `api_core::extractors::AuthUser` — fields
  `user_id: Uuid`, `email`, `name`, `tenant_id: Option<Uuid>`, `role: Option<TenantRole>`.
- `RlsConnection` extractor: `api_core::extractors::RlsConnection` — `Deref`s to
  `PgConnection` (use `&mut **rls`); helpers `tenant_id()`, `user_id()`,
  `role()`, `has_role(TenantRole)`, `is_super_admin()`, `begin()` (via Deref to
  `Connection`), `release().await`.
- accounting-server is a RESOURCE server: it validates JWTs issued by
  api-server; it issues NO tokens.

### REUSE — do not reinvent (already in `db`):
- Base models: `db::models::accounting::{Contact, Invoice, InvoiceItem,
  BankStatement, BankStatementLine, PaymentMatch, InvoiceStatus, VatRate,
  PaymentMatchState, CreateInvoice, UpdateInvoice, CreateInvoiceItem,
  UpdateInvoiceItem}`.
- Base repo: `db::repositories::AccountingRepository` (base invoice/contact/
  bank CRUD; `create_invoice_rls` already does the #1522 rounding invariant).
- New models (Foundation-built, REUSE — do not redefine): see §6.

---

## 5. DTO ownership

Each route file DEFINES its own request/response DTOs (`#[derive(Serialize,
Deserialize, ToSchema)]`) inside that file. They are owned by the file's owner.
Foundation has already declared them in main.rs's `components(schemas(...))`,
so coders may add fields but should AVOID renaming/removing a DTO type or the
OpenApi registration breaks (write a contract-note if you must rename).

Already-defined DTOs per file (extend in place; keep type names):
- accounts.rs: `CompanyAccessDto`, `MyCompaniesResponse`, `SwitchCompanyRequest`.
- config.rs: `UpsertCompanySettingsRequest`, `CreateNumberingSeriesRequest`.
- contacts.rs: `ContactRequest`, `MergeContactRequest`.
- catalog.rs: `CatalogItemRequest`, `PriceLevelRequest`.
- invoices.rs: `IssueInvoiceRequest`, `CreateCreditNoteRequest`,
  `SetExchangeRateRequest`, `CreateLinkRequest` (+ reuse `CreateInvoice` /
  `UpdateInvoice` from `db::models::accounting`).
- platform.rs: `CreateShareLinkRequest`, `SharedInvoiceView`, `AddTagRequest`,
  `TwoFactorEnrollResponse`, `TwoFactorConfirmRequest`.

**If you add a NEW DTO type** that must appear in Swagger, you CANNOT edit
main.rs — instead write the new type name to `contract-notes/<role>.md` so
Phase 2 registers it in `components(schemas(...))`. The code still compiles
without registration; only Swagger completeness is affected.

### 5e. FE typed surface (FE-Screens, before SDK lands)
The accounting-server REST surface FE consumes (all under
`<NEXT_PUBLIC_ACCOUNTING_API_URL>/api/v1`, Bearer JWT):
- Contacts: `GET /contacts`, `GET /contacts/{id}`, `POST /contacts`,
  `PATCH /contacts/{id}`, `DELETE /contacts/{id}`.
- Invoices: `GET /invoices`, `GET /invoices/{id}`, `GET /invoices/{id}/items`,
  `POST /invoices`, `PATCH /invoices/{id}`, `DELETE /invoices/{id}`,
  `POST /invoices/{id}/issue`, `POST /invoices/{id}/duplicate`.
Response shapes = the db models in §6 (serde camel/snake as emitted).

---

## 6. New db models (Foundation-built — REUSE, do not redefine)

Column names are EXACT (verify against 00194/00195/00196).

`db::models::acc_config` (00194):
- `AccCompanySettings{ id, tenant_id, legal_name, ico?, dic?, vat_id?, address?,
  email?, phone?, web?, logo_url?, brand_color?, tax_mode, vat_payer,
  base_currency, rounding_mode, default_payment_terms_days, default_invoice_note?,
  default_footer?, default_language, created_at, updated_at }`
  — `tax_mode ∈ {tax_records, accounting}`, `rounding_mode ∈ {arithmetic, bankers, none}`.
- `AccNumberingSeries{ id, tenant_id, doc_type, year, prefix, format, next_value(i64),
  created_at, updated_at }` — `doc_type ∈ {invoice, proforma, advance, credit_note}`,
  unique `(tenant_id, doc_type, year)`.
- `AccUnit{ id, tenant_id, code, name, is_active, created_at, updated_at }`.
- `AccVatRate{ id, tenant_id, name, rate(Decimal), is_default, is_active,
  effective_from?, effective_to?, created_at, updated_at }`.
- `AccBankAccount{ id, tenant_id, label, iban, swift?, bank_name?, currency,
  is_default, is_active, created_at, updated_at }`.
- `AccEmailTemplate{ id, tenant_id, kind, language, subject, body, is_default,
  created_at, updated_at }`.
- `AccDocumentTemplate{ id, tenant_id, name, theme, accent_color?, show_logo,
  is_default, config(serde_json::Value), created_at, updated_at }`.

`db::models::acc_catalog` (00195):
- `AccItemCategory{ id, tenant_id, name, parent_id?, created_at, updated_at }`.
- `AccCatalogItem{ id, tenant_id, code?, name, description?, kind, unit_id?,
  vat_rate(Decimal), unit_price(Decimal), price_is_gross, category_id?,
  stock_tracked, is_active, created_at, updated_at }` — `kind ∈ {goods, service}`.
- `AccPriceLevel{ id, tenant_id, item_id, name, price(Decimal), price_is_gross,
  currency, created_at, updated_at }`.

`db::models::acc_contacts_ext` (00195 — `contact` ALTERs):
- `AccContactAddress{ id, tenant_id, contact_id, kind, label?, street?, city?,
  postal_code?, country?, is_default, created_at, updated_at }` — `kind ∈ {billing, delivery}`.
- `AccContactTag{ id, tenant_id, contact_id, tag, created_at }`.
- `AccContactExt{ <base contact cols>, kind, is_active, default_currency?,
  default_price_level?, default_payment_terms_days?, default_discount_pct?,
  default_language?, vat_verified_at?, vat_verified_valid?, credit_limit?,
  notes?, merged_into_id? }` — use this projection when reading the new columns.
  The base `contact` columns added: `kind` (default 'customer'),
  `is_active`(default true), the `default_*` cols, `vat_verified_*`,
  `credit_limit`, `notes`, `merged_into_id`.

`db::models::acc_invoicing_ext` (00196 — `invoice` ALTERs):
- `AccDocType` enum `{Invoice, Proforma, Advance, CreditNote}` (TEXT snake_case).
- `AccDocumentLink{ id, tenant_id, from_invoice_id, to_invoice_id, relation,
  created_at }` — `relation ∈ {settles, corrects, converts, related}`.
- `AccAttachment{ id, tenant_id, invoice_id?, filename, content_type,
  size_bytes(i64), storage_key, uploaded_by?, created_at }`.
- `AccInvoiceExt{ <base invoice cols, status as String>, doc_type,
  corrected_invoice_id?, advance_settlement(Decimal), settled_advance_id?,
  exchange_rate?(Decimal), base_currency_amount?(Decimal), numbering_series_id?,
  bank_account_id? }`. Invoice `status` CHECK now allows
  `{draft, issued, sent, paid, partially_paid, overdue, cancelled}`.

`db::models::acc_platform` (00196):
- `AccTag{ id, tenant_id, entity_type, entity_id, tag, created_at }`.
- `AccShareLink{ id, tenant_id, invoice_id, token, expires_at?, revoked_at?,
  created_by?, created_at }`.
- `AccAuditLog{ id, tenant_id, actor_id?, action, entity_type, entity_id?,
  diff(serde_json::Value), created_at }` — APPEND-ONLY (no UPDATE/DELETE policy).
- `AccTwoFactor{ id, tenant_id, user_id, secret, enabled, confirmed_at?,
  recovery_codes(serde_json::Value), created_at, updated_at }`.

---

## 7. Migrations added (Foundation)
- `00194_acc_company_config.sql`: acc_company_settings, acc_numbering_series,
  acc_unit, acc_vat_rate, acc_bank_account, acc_email_template,
  acc_document_template.
- `00195_acc_catalog_contacts_ext.sql`: acc_item_category, acc_catalog_item,
  acc_price_level; `contact` ALTERs (kind/is_active/default_*/vat_verified_*/
  credit_limit/notes/merged_into_id); acc_contact_address, acc_contact_tag.
- `00196_acc_invoicing_platform.sql`: `invoice` ALTERs (doc_type/
  corrected_invoice_id/advance_settlement/settled_advance_id/exchange_rate/
  base_currency_amount/numbering_series_id/bank_account_id + relaxed status
  CHECK); acc_document_link, acc_attachment, acc_tag, acc_share_link,
  acc_audit_log, acc_two_factor.

Coders add NO migrations (avoids version collisions). Need a column? Write to
`contract-notes/<role>.md`.

---

## 8. FE SDK generation (FE-Scaffold)
Mirror `frontend/packages/reality-api-client`:
- `package.json` name `@ppt/accounting-api-client`, script
  `"generate": "openapi-ts -i <accounting-server openapi> -o ./src/generated"`,
  devDep `@hey-api/openapi-ts`, dep `@tanstack/react-query`.
- The accounting-server exposes its spec at `/api-docs/openapi.json`
  (Swagger UI at `/swagger-ui`) on `:8082`. Phase 2 wires the actual spec
  input path; FE-Scaffold sets up the package structure + config + a
  `src/config.ts` (base URL from `NEXT_PUBLIC_ACCOUNTING_API_URL`) like
  reality-api-client's `config.ts`.
- accounting-web dev server runs on `:3002` (already in CORS defaults).
