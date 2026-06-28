# PRD-ACC-001 — Online Invoicing & Light-Accounting Product

> **Product code:** `ACC` · **Status:** Draft · **Provenance:** Generalized from competitor analysis of a CZ/SK online
> invoicing SaaS; vendor-neutral, functionality only.
> **Companions:** [`epics.md`](epics.md) (16 epics) · [`use-cases.md`](use-cases.md) (~140 `UC-ACC-XX.Y`) · [`stories/`](stories/).

---

## 1. Overview

A standalone, cloud-based product that lets freelancers, sole traders, and small businesses **invoice, get paid, track
costs, and stay tax-compliant** without an accounting background — while giving their accountant 24/7 remote access to
clean, exportable data. The product spans the full document cycle (quote → order → delivery → invoice → payment),
expense capture with AI/OCR, automated bank reconciliation, statutory VAT outputs, light inventory, reporting, and a
public API, delivered on web and mobile.

It is positioned as **light accounting / tax-records** software (document-centric), not a full double-entry general
ledger. The accountant remains the owner of the statutory close; the product produces the source documents, VAT
outputs, and a clean export.

## 2. Problem Statement

Small businesses lose time and money on billing admin: manual invoice creation, chasing overdue payments, re-keying
supplier receipts, reconciling bank statements by hand, and assembling VAT filings. Existing desktop accounting tools
are heavy and accountant-oriented; spreadsheets don't scale, lack compliance, and create error-prone handoffs.

**The job to be done:** *"Let me bill a customer in under a minute, see what I'm owed, capture my costs without typing
them, and hand my accountant everything they need — from my phone, compliantly."*

## 3. Goals & Success Metrics

| Goal | Metric | Target |
|------|--------|--------|
| Fast onboarding | Time from sign-up to first sent invoice | < 5 min |
| Frictionless billing | Median time to create a repeat invoice | < 60 s |
| Get paid sooner | Reduction in average overdue days after dunning enabled | −30% |
| Hands-off reconciliation | Share of incoming payments auto-matched | > 85% |
| Cost capture without typing | OCR/AI field accuracy on uploaded documents | > 90% |
| Compliance confidence | VAT periods filed from product without correction | > 95% |
| Accountant satisfaction | Clean-export imports needing no manual fixup | > 90% |
| Retention | Active companies renewing after 12 months | > 70% |

## 4. User Stories (epic-level)

- **As an Owner**, I want to register and operate several companies under one login, so I can manage all my entities in one place. *(EPIC-ACC-01)*
- **As an Owner**, I want my company identity, numbering, and branding configured once, so every document is correct and on-brand. *(EPIC-ACC-02)*
- **As a User**, I want a contact directory with registry autocomplete and VAT verification, so I create documents fast and error-free. *(EPIC-ACC-03)*
- **As a User**, I want a reusable price list, so line items, prices, and VAT are consistent. *(EPIC-ACC-04)*
- **As a User**, I want to issue, send, and track professional invoices (incl. proformas, credit notes, foreign currency), so I get paid. *(EPIC-ACC-05)*
- **As a User**, I want quotes/orders/delivery notes that convert into invoices, so I cover the whole sales cycle. *(EPIC-ACC-06)*
- **As a User**, I want to capture supplier costs by photo/upload with AI extraction, so expenses are recorded in seconds. *(EPIC-ACC-07)*
- **As a User**, I want cash and bank documents with automatic payment matching, so my books reconcile themselves. *(EPIC-ACC-08)*
- **As an Owner**, I want online pay-by-link and automatic overdue reminders, so collection is faster and hands-off. *(EPIC-ACC-09)*
- **As an Owner/Accountant**, I want one-click VAT outputs (return, control/recap statement, EC sales list), so filing is painless. *(EPIC-ACC-10)*
- **As a User**, I want light stock tracking driven by my documents, so I sell what I have and know my movements. *(EPIC-ACC-11)*
- **As an Owner**, I want a dashboard and reports plus an accountant export, so I make decisions and hand off cleanly. *(EPIC-ACC-12)*
- **As an Owner**, I want recurring invoices and automation, so revenue runs without daily effort. *(EPIC-ACC-13)*
- **As an Owner**, I want a public API and connectors (e-shop, bank, gateway), so data flows in and out automatically. *(EPIC-ACC-14)*
- **As a User**, I want mobile apps and notifications, so I work anywhere and never miss a paid/overdue event. *(EPIC-ACC-15)*
- **As an Owner**, I want secure, backed-up, exportable, GDPR-compliant data, so I can trust the platform. *(EPIC-ACC-16)*

## 5. Requirements

### 5.1 Functional Requirements

Each use case in [`use-cases.md`](use-cases.md) is a discrete, testable functional requirement. They map 1:1 as
**`FR-ACC-XX.Y` ⇔ `UC-ACC-XX.Y`** (same numbering). The headline FRs per epic:

| FR group | Epic | Headline functional requirements |
|----------|------|----------------------------------|
| **FR-ACC-01** | Accounts & Access | Registration/login; multi-company (agendas) with strict isolation; user invite + RBAC; accountant access; subscription/plan; 2FA; audit log; GDPR export/delete. |
| **FR-ACC-02** | Configuration | Company profile; VAT/tax-mode config; gapless numbering series; document designs + branding; default texts/terms; email templates; localization; units; VAT rates; rounding rules. |
| **FR-ACC-03** | Contacts & CRM | CRUD + merge; registry autocomplete; VAT verification; multiple addresses; per-contact defaults; transaction & communication history; import/export; receivables/credit. |
| **FR-ACC-04** | Catalog | Item CRUD; price levels (net/gross); VAT & unit; categories/tags; discounts; import/export; stock link; inline quick-add. |
| **FR-ACC-05** | Sales Invoicing | Invoice/proforma/advance-settlement/credit-note issuance; multi-currency; discounts/VAT/rounding/precision; payment QR; PDF/ISDOC/XML; email + shareable link; attachments/tags; document linking; duplicate; status lifecycle; bulk actions. |
| **FR-ACC-06** | Quotes/Orders/Delivery | Quote + acceptance; sales order; delivery note; conversions quote→order→delivery→invoice; fulfillment status. |
| **FR-ACC-07** | Purchases & Expenses | Received-invoice/expense entry; upload inbox; AI/OCR extraction; review & post; categorization; supplier link; attachments; payable tracking. |
| **FR-ACC-08** | Cash & Bank | Cash receipts/payments; bank accounts; statement import; auto-match; manual match/unmatch; multiple accounts; POS/fiscal ingest; reconciliation. |
| **FR-ACC-09** | Payments & Collections | Manual payment; online pay-by-link; auto-settle on gateway payment; confirmations; dunning rules + escalation; aging; partial/overpayment; refunds. |
| **FR-ACC-10** | VAT & Tax | VAT records; VAT return; control/recap statement; EC sales list; reverse-charge; cross-border/OSS; per-document regime; e-file export; period summary; non-payer mode; VAT rounding. |
| **FR-ACC-11** | Inventory | Stock item/warehouse; stock-in/out via documents; levels; movement history; valuation; low-stock flags; adjustments. |
| **FR-ACC-12** | Reporting | Dashboard; cashflow; sales; aging; VAT report; list exports (xlsx/csv); accountant export package; date-range filtering; profit overview. |
| **FR-ACC-13** | Automation | Recurring invoices (idempotent); scheduled reminders; continuous auto-match; auto thank-you; rule-based defaults; FX auto-refresh. |
| **FR-ACC-14** | Integrations & API | Authenticated REST API; API-key/OAuth management; webhooks; e-shop/bank/gateway connectors; accounting-software export; data import; rate limits/quotas. |
| **FR-ACC-15** | Mobile & Notifications | Native iOS/Android; mobile invoice create/send; receipt scan; in-app notifications; offline draft+sync; mobile dashboard. |
| **FR-ACC-16** | Platform | 2FA; RBAC enforcement; daily backups + restore; data export/migration; data import; GDPR DSARs; audit trail; legislative/template updates; session security. |

### 5.2 Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| **NFR-ACC-01** | Multi-tenancy | Strict per-company data isolation (e.g., row-level security); zero cross-company leakage is a release gate. |
| **NFR-ACC-02** | Security | Argon2-class password hashing; 2FA; encryption in transit and at rest; secret management for connector credentials; session lockout on suspicious login. |
| **NFR-ACC-03** | Compliance | GDPR (DSAR export/erasure, retention); statutory VAT formats kept current per jurisdiction; legally gapless document numbering. |
| **NFR-ACC-04** | Localization | Multi-language UI + documents; locale-aware number/date/currency formatting; jurisdiction-pluggable tax rules. |
| **NFR-ACC-05** | Availability & Durability | Automatic daily backups with tested restore; target ≥ 99.9% uptime; no data loss on a single-node failure. |
| **NFR-ACC-06** | Performance | Document list/search and dashboard load < 2 s P95 at SMB data volumes; PDF render < 3 s. |
| **NFR-ACC-07** | Scalability | Linear scaling to many tenants and documents; async processing for OCR, recurring runs, bank sync. |
| **NFR-ACC-08** | Correctness | Deterministic VAT/rounding/currency math with documented rules; reconciliation must always balance. |
| **NFR-ACC-09** | Auditability | Immutable change history for documents and settings; who/what/when on every mutation. |
| **NFR-ACC-10** | API | Versioned, rate-limited, quota-enforced public API; idempotent writes; webhook retry with backoff. |
| **NFR-ACC-11** | Portability | Full data export at any time (no lock-in); standard electronic document formats (PDF, ISDOC/XML, CSV/XLSX). |
| **NFR-ACC-12** | Usability | Non-accountant can issue a first invoice unaided; mobile parity for core flows; accessible (WCAG-aligned). |

## 6. Out of Scope (v1)

- Full statutory **double-entry general ledger** and period close (the product feeds the accountant's tools; it is not the ledger of record).
- **Payroll**, HR, and employee expense workflows.
- **Bank-specific certifications / direct payment initiation** beyond generic statement + pay-by-link connectors.
- **Any integration with, or data import from, the analyzed source product** (explicitly excluded — clean-room functionality only).
- Advanced **manufacturing / multi-warehouse logistics** (inventory is intentionally "light").
- Jurisdiction tax engines beyond the launch market(s) — built **pluggable**, but only launch locales ship in v1.

## 7. Dependencies

| Dependency | Used by | Notes |
|------------|---------|-------|
| Business/VAT registry lookup | FR-ACC-03 | External; cache + graceful degradation when unavailable. |
| Bank statement/payment connectors | FR-ACC-08, -14 | Per-bank formats/feeds; normalize to a common model. |
| Payment gateway(s) | FR-ACC-09, -14 | Pay-by-link, webhooks for settlement. |
| OCR/AI extraction service | FR-ACC-07, -15 | Metered; per-plan free quota + paid overage. |
| Exchange-rate source | FR-ACC-05, -13 | Daily refresh for foreign-currency documents. |
| E-file/statutory format definitions | FR-ACC-10 | Per-jurisdiction XML schemas; versioned. |
| Email/SMTP (and custom SMTP) | FR-ACC-05, -09 | Deliverability; per-company custom SMTP option. |

## 8. Timeline (phased by priority)

| Phase | Scope | Epics |
|-------|-------|-------|
| **P1 — MVP** | Register → configure → contacts + catalog → issue/send/track invoices → secure, exportable data | ACC-01, -02, -03, -04, -05, -16 |
| **P2 — Money in/out** | Expenses + inbox, cash & bank + auto-match, payments & dunning, VAT outputs, reporting | ACC-06, -07, -08, -09, -10, -12 |
| **P3 — Scale & reach** | Inventory, automation/recurring, API & connectors, mobile & notifications | ACC-11, -13, -14, -15 |

---

### Traceability

`PRD-ACC-001` → `EPIC-ACC-01..16` → `UC-ACC-01..16` (`FR-ACC-XX.Y` ⇔ `UC-ACC-XX.Y`) → `STORY-ACC-XX-NNN`.
This PRD is the input the BMAD `create-epics-and-stories` workflow consumes (FRs in §5.1, NFRs in §5.2).
