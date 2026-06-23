# Invoicing & Accounting Product — Epics Catalog

> **Product code:** `ACC` (standalone online invoicing + light-accounting SaaS, e.g. `@ppt/accounting-web`).
> **Provenance:** Generalized from competitor analysis of a CZ/SK online invoicing SaaS. **Vendor-neutral** — no
> dependency on, or integration with, any specific product. Functionality only.
> **Scope:** Self-service invoicing, expenses, contacts/CRM, bank & cash, VAT/tax compliance, light inventory,
> reporting, automation, API, and mobile — targeted at freelancers, sole traders, and SMBs, plus their accountants.
> **Maps to:** detailed use-case catalog in [`use-cases.md`](use-cases.md) (`UC-ACC-XX.Y`).

---

## Actors

| Actor | Description |
|-------|-------------|
| **Owner / Admin** | Account owner. Manages the company/agenda, subscription, users, and global settings. |
| **User (Staff)** | Team member with role-based access (issue documents, manage contacts, etc.). |
| **Accountant** | External collaborator with (often read/export) access; may manage **multiple client companies**; prepares tax filings. |
| **Customer** | Recipient of issued documents. Limited self-service: views/pays an invoice via shared link. |
| **Supplier / Vendor** | Counterparty on purchase/expense documents (data subject, not necessarily a system user). |
| **System / Automation** | Scheduled, non-interactive actor: recurring invoices, reminders, bank sync, exchange-rate refresh, OCR/AI extraction. |
| **Bank** *(external)* | Provides statement feed / payment data. |
| **Payment Gateway** *(external)* | Processes online card/bank payments from a pay-by-link. |
| **Tax Authority** *(external)* | Recipient of VAT/tax filings via export / e-file. |
| **Business / VAT Registry** *(external)* | Source for company autocomplete and VAT-payer verification. |
| **E-shop / External System** *(external)* | Integrates via public API / webhooks. |

**RBAC roles (suggested):** Owner, Admin, Standard user, Accountant, Read-only/Viewer.

---

## Epic ↔ Use-Case Map

| Epic | Title | UC Category | Priority |
|------|-------|-------------|----------|
| `EPIC-ACC-01` | Accounts, Organizations & Access | UC-ACC-01 | P1 |
| `EPIC-ACC-02` | Company & Document Configuration | UC-ACC-02 | P1 |
| `EPIC-ACC-03` | Contacts & CRM | UC-ACC-03 | P1 |
| `EPIC-ACC-04` | Product & Price-List Catalog | UC-ACC-04 | P1 |
| `EPIC-ACC-05` | Sales Invoicing | UC-ACC-05 | P1 |
| `EPIC-ACC-06` | Quotes, Orders & Delivery | UC-ACC-06 | P2 |
| `EPIC-ACC-07` | Purchases & Expenses | UC-ACC-07 | P2 |
| `EPIC-ACC-08` | Cash & Bank | UC-ACC-08 | P2 |
| `EPIC-ACC-09` | Payments & Collections | UC-ACC-09 | P2 |
| `EPIC-ACC-10` | VAT & Tax Compliance | UC-ACC-10 | P2 |
| `EPIC-ACC-11` | Inventory / Stock | UC-ACC-11 | P3 |
| `EPIC-ACC-12` | Reporting, Dashboard & Cashflow | UC-ACC-12 | P2 |
| `EPIC-ACC-13` | Automation & Recurring | UC-ACC-13 | P3 |
| `EPIC-ACC-14` | Integrations & API | UC-ACC-14 | P3 |
| `EPIC-ACC-15` | Mobile & Notifications | UC-ACC-15 | P3 |
| `EPIC-ACC-16` | Platform: Security, Data & Compliance | UC-ACC-16 | P1 |
| `EPIC-ACC-17` | Backend Service (`accounting-server`) & Shared-Core Integration | — (platform) | P1 |

**Suggested MVP (P1):** EPIC-ACC-17 (service skeleton), then -01, -02, -03, -04, -05, -16 — stand up the server sharing
core with `api-server`, then a single user can register a company, configure it, keep contacts and a price list,
issue/send/track invoices, and have secure, backed-up, exportable data.

> **Deployment topology:** `ACC` ships as a **separate `accounting-server` (`:8082`)** that shares the `common` /
> `api-core` / `db` core crates with `api-server`, exactly as `reality-server` does. See [`architecture.md`](architecture.md).

---

## EPIC-ACC-01 — Accounts, Organizations & Access

- **Summary:** Account lifecycle, multi-company (agendas), users, role-based access, and accountant collaboration.
- **Business value:** One login operates many companies; owners delegate safely; accountants get remote 24/7 access without file shuffling.
- **Primary actors:** Owner/Admin, User, Accountant.
- **Stories (→ UC):** registration & login (UC-ACC-01.1, .13); create/switch company (01.2–01.3); invite user & assign role (01.4–01.5); deactivate user (01.6); grant accountant access (01.7); manage subscription/plan (01.8); 2FA (01.9); audit log (01.10); GDPR export/delete (01.11–01.12).
- **Dependencies:** EPIC-ACC-16 (security/RBAC primitives).
- **Key risks:** Cross-company data isolation (tenant leakage); correct permission enforcement per role.

## EPIC-ACC-02 — Company & Document Configuration

- **Summary:** Per-company settings that drive every document: identity, tax mode, numbering, designs, defaults, localization.
- **Business value:** Correct, compliant, on-brand documents with zero per-invoice setup.
- **Primary actors:** Owner/Admin, Accountant.
- **Stories (→ UC):** company profile & logo (UC-ACC-02.1); VAT/tax-mode config — VAT payer vs non-payer, tax-records vs accounting (02.2); numbering series per doc type/year (02.3); document design/template & branding (02.4); default texts, terms, footers (02.5); email templates (02.6); languages/localization (02.7); units of measure (02.8); VAT rates (02.9); bank accounts on documents (02.10); default currency & rounding (02.11).
- **Dependencies:** EPIC-ACC-01.
- **Key risks:** Numbering gaps/duplicates (legal requirement); tax-mode misconfiguration cascading into VAT outputs.

## EPIC-ACC-03 — Contacts & CRM

- **Summary:** Customer/supplier directory with registry autocomplete, VAT verification, per-contact defaults, and history.
- **Business value:** Faster, error-free document creation; a single view of each relationship.
- **Primary actors:** User, Owner/Admin; *external:* Business/VAT Registry.
- **Stories (→ UC):** create/edit/merge contact (UC-ACC-03.1, .5); registry autocomplete (03.2); VAT-payer verification (03.3); multiple billing/delivery addresses (03.4); tags/segments (03.6); transaction history (03.7); communication/email history (03.8); per-contact defaults — currency, price level, terms, discount (03.9); import/export (03.10); receivables/credit per contact (03.11).
- **Dependencies:** EPIC-ACC-14 (registry lookup integration).
- **Key risks:** Stale registry data; duplicate contacts.

## EPIC-ACC-04 — Product & Price-List Catalog

- **Summary:** Reusable catalog of goods/services with pricing, VAT, units, and optional stock link.
- **Business value:** Consistent pricing and tax; one-click line items.
- **Primary actors:** User, Owner/Admin.
- **Stories (→ UC):** create item (UC-ACC-04.1); prices incl. multiple price levels & with/without VAT (04.2); VAT rate & unit (04.3); categorize/tag (04.4); discounts (04.5); import/export (04.6); link to stock (04.7); quick-add from invoice line (04.8).
- **Dependencies:** EPIC-ACC-02 (VAT rates, units); EPIC-ACC-11 (stock link).

## EPIC-ACC-05 — Sales Invoicing

- **Summary:** The core revenue document engine: invoices, advances/proformas + settlement, credit notes, multi-currency, delivery (email/link), and export.
- **Business value:** Get paid faster with professional, compliant, trackable documents.
- **Primary actors:** User, Owner/Admin, Customer.
- **Stories (→ UC):** create invoice & line items (UC-ACC-05.1–.2); discounts/VAT/rounding (05.3, .19–.20); foreign currency w/ exchange rate (05.4); proforma/advance + tax-settlement document (05.5–05.6); credit note/corrective (05.7); payment QR/code (05.8); PDF preview/generate (05.9); send by email (05.10); send via shareable link (05.11); export PDF/ISDOC/XML (05.12); attachments (05.13); tags (05.14); document linking (05.15); duplicate (05.16); status lifecycle (05.17); bulk actions (05.18).
- **Dependencies:** EPIC-ACC-02, -03, -04; EPIC-ACC-09 (payment status); EPIC-ACC-10 (VAT).
- **Key risks:** VAT/rounding correctness; currency rounding; status integrity across linked documents.

## EPIC-ACC-06 — Quotes, Orders & Delivery

- **Summary:** Pre-sale and fulfillment documents that convert downstream into invoices.
- **Business value:** Covers the full sales cycle (quote → order → delivery → invoice) without re-keying.
- **Primary actors:** User, Customer.
- **Stories (→ UC):** price quote/offer + acceptance tracking (UC-ACC-06.1–.2); convert quote→order/invoice (06.3); sales order (06.4); delivery note (06.5); convert order→delivery/invoice (06.6); fulfillment status (06.7).
- **Dependencies:** EPIC-ACC-05; EPIC-ACC-11 (delivery from stock).

## EPIC-ACC-07 — Purchases & Expenses

- **Summary:** Received invoices/expense capture with an inbox and AI/OCR data extraction.
- **Business value:** Cashflow visibility on the cost side; minutes-not-hours document entry.
- **Primary actors:** User, Accountant, System (OCR/AI); Supplier.
- **Stories (→ UC):** record received invoice/bill (UC-ACC-07.1); capture expense (07.2); upload to inbox (07.3); AI/OCR extraction (07.4); review & post (07.5); categorize (07.6); link to supplier (07.7); attach scan/receipt (07.8); payable status & due date (07.9).
- **Dependencies:** EPIC-ACC-03, -10; EPIC-ACC-14 (AI/OCR service).
- **Key risks:** OCR accuracy; duplicate-bill detection.

## EPIC-ACC-08 — Cash & Bank

- **Summary:** Cash register documents, bank accounts, statement import, and automatic payment matching.
- **Business value:** Books that reconcile themselves; less manual matching.
- **Primary actors:** User, System; *external:* Bank, POS/fiscal device.
- **Stories (→ UC):** cash receipt/payment (UC-ACC-08.1–.2); manage registers (08.3); add bank account (08.4); import statement (08.5); auto-match payments (08.6); manual match/unmatch (08.7); multiple accounts/registers (08.8); POS/fiscal receipt integration (08.9); reconcile balances (08.10).
- **Dependencies:** EPIC-ACC-09, -14 (bank connection).
- **Key risks:** Mismatched/duplicate payment pairing; statement-format variability.

## EPIC-ACC-09 — Payments & Collections

- **Summary:** Recording payments, online pay-by-link, confirmations, and overdue dunning.
- **Business value:** Faster collection, fewer overdue receivables, less manual chasing.
- **Primary actors:** User, System, Customer; *external:* Payment Gateway.
- **Stories (→ UC):** record manual payment (UC-ACC-09.1); enable online payment / pay-by-link (09.2); receive online payment & auto-mark paid (09.3); payment confirmation/thank-you (09.4); configure & send overdue reminders (09.5); reminder escalation levels (09.6); receivables aging (09.7); partial/overpayments (09.8); refunds (09.9).
- **Dependencies:** EPIC-ACC-05, -08, -14.

## EPIC-ACC-10 — VAT & Tax Compliance

- **Summary:** VAT records and statutory outputs: VAT return, control/recapitulative statement, EC sales list, reverse-charge, cross-border/OSS.
- **Business value:** Compliance without an accountant for routine periods; clean handoff for complex ones.
- **Primary actors:** Owner/Admin, Accountant; *external:* Tax Authority.
- **Stories (→ UC):** VAT records output/input (UC-ACC-10.1); VAT return per period (10.2); control/recapitulative statement (10.3); EC sales list / intra-EU (10.4); reverse-charge handling (10.5); cross-border / OSS VAT (10.6); per-document VAT regime (10.7); export filings XML/e-file (10.8); VAT summary per period (10.9); non-VAT-payer mode (10.10); VAT rounding/reconciliation (10.11).
- **Dependencies:** EPIC-ACC-02 (tax mode), -05, -07.
- **Key risks:** Legislative correctness & change cadence; jurisdiction-specific filing formats (keep pluggable).

## EPIC-ACC-11 — Inventory / Stock

- **Summary:** Light warehouse: stock items, movements, levels, valuation — driven by sales/delivery/purchase docs.
- **Business value:** Sell what you have; accurate cost of goods; movement traceability.
- **Primary actors:** User, System.
- **Stories (→ UC):** stock item/warehouse (UC-ACC-11.1); stock-in (11.2); stock-out via invoice/delivery (11.3); stock levels (11.4); movement history (11.5); valuation (11.6); low-stock indicators (11.7); adjustments/corrections (11.8).
- **Dependencies:** EPIC-ACC-04, -05, -06.

## EPIC-ACC-12 — Reporting, Dashboard & Cashflow

- **Summary:** At-a-glance overview plus sales/receivables/VAT/cashflow reports and accountant export.
- **Business value:** Decisions from data; painless month-end and accountant handoff.
- **Primary actors:** Owner/Admin, Accountant.
- **Stories (→ UC):** dashboard/overview — revenue, receivables, overdue (UC-ACC-12.1); income/expense cashflow (12.2); sales report by period/customer/item (12.3); receivables/payables aging (12.4); VAT report (12.5); list exports xlsx/csv (12.6); accountant export package (12.7); date-range filtering (12.8); profit overview (12.9).
- **Dependencies:** EPIC-ACC-05, -07, -08, -10.

## EPIC-ACC-13 — Automation & Recurring

- **Summary:** Set-and-forget recurring billing, scheduled reminders, auto-matching, auto-confirmations, exchange-rate refresh.
- **Business value:** Revenue and collections run without daily human action.
- **Primary actors:** System, Owner/Admin.
- **Stories (→ UC):** configure recurring invoice (UC-ACC-13.1); auto-generate & send (13.2); schedule reminders (13.3); auto-match payments (13.4); auto thank-you on payment (13.5); rule-based defaults/templates (13.6); auto-download exchange rates (13.7).
- **Dependencies:** EPIC-ACC-05, -08, -09.
- **Key risks:** Idempotency (no duplicate recurring runs); time-zone/scheduling correctness.

## EPIC-ACC-14 — Integrations & API

- **Summary:** Public REST API, webhooks, and connectors: e-shops, banks, payment gateways, registries, accounting software, exchange rates.
- **Business value:** The product becomes a hub; data flows in/out without manual entry.
- **Primary actors:** Owner/Admin, External System; *external:* Bank, Gateway, Registry.
- **Stories (→ UC):** authenticate & call API (UC-ACC-14.1); manage API keys / OAuth clients (14.2); webhooks/events (14.3); e-shop/external connect (14.4); bank connection (14.5); payment gateway (14.6); export to accounting software (14.7); import data (14.8); rate limits/quotas (14.9).
- **Dependencies:** EPIC-ACC-01 (auth), -16.
- **Key risks:** Quota/abuse control; third-party API stability; secret management.

## EPIC-ACC-15 — Mobile & Notifications

- **Summary:** Mobile apps for on-the-go invoicing and receipt capture, plus in-app notifications.
- **Business value:** Invoice and capture costs anywhere; never miss a paid/overdue event.
- **Primary actors:** User, System.
- **Stories (→ UC):** mobile app iOS/Android (UC-ACC-15.1); create/send invoice on mobile (15.2); scan receipt/document (15.3); in-app notifications — paid, overdue, etc. (15.4); offline draft & sync (15.5); mobile dashboard (15.6).
- **Dependencies:** EPIC-ACC-05, -07, -14.

## EPIC-ACC-16 — Platform: Security, Data & Compliance

- **Summary:** Cross-cutting non-functional foundation: 2FA, RBAC, backups, import/export/migration, GDPR, audit trail, legislative updates.
- **Business value:** Trust — data is safe, portable, compliant, and recoverable.
- **Primary actors:** Owner/Admin, System.
- **Stories (→ UC):** 2FA (UC-ACC-16.1); RBAC (16.2); daily backups (16.3); data export/migration (16.4); data import (16.5); GDPR export/erasure (16.6); audit trail (16.7); legislative/template updates (16.8); session/login security (16.9).
- **Dependencies:** foundational for all epics.
- **Key risks:** Tenant isolation; backup restorability; secret/credential handling.

## EPIC-ACC-17 — Backend Service (`accounting-server`) & Shared-Core Integration

- **Summary:** Stand up a dedicated `accounting-server` (`:8082`) that shares the `common` / `api-core` / `db` core
  crates with `api-server` (mirroring `reality-server`), with accounting domain logic in a new `accounting-core` crate.
- **Business value:** A truly standalone product surface (own release cadence, scaling, API, frontend) without
  re-implementing auth, DB, tenancy, OpenAPI, or observability — they come from shared core.
- **Primary actors:** (platform) Owner/Admin, System; *external:* `api-server` as OAuth/JWT issuer.
- **Stories:** scaffold `crates/accounting-core` + `servers/accounting-server` from the `reality-server` skeleton;
  wire `common`/`api-core`/`db`; bind `:8082`; JWT validation as a resource server (api-server issues); shared-DB
  accounting tables + RLS; OpenAPI + `@ppt/accounting-api-client` (hey-api); deploy (compose/caddy/CORS/`pmctl`
  worktree target); **migrate** `api-server`'s `/api/v1/accounting/*` MVP into the new server; remove the old route.
- **Dependencies:** EPIC-ACC-01 (shared auth), -14 (public API surface), -16 (platform). Decided in [`architecture.md`](architecture.md).
- **Key risks:** Dual-serve window during migration (avoid in prod); shared-DB RLS correctness across two servers;
  keeping `db`/`accounting-core` boundaries clean so a future dedicated-DB split stays possible.
