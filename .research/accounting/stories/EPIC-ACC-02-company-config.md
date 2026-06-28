# EPIC-ACC-02 — Company & Document Configuration · Stories

> Covers `UC-ACC-02.1–.11`. These settings drive every document; correctness here prevents whole classes of downstream bugs.
> **Shared DoD:** AC pass · tests green · per-company isolation · audit-log on change · i18n externalized.

---

## STORY-ACC-02-001 — Company profile & branding
*Covers UC-ACC-02.1, .4*

**User Story:** As an **Owner**, I want to set my company identity and branding once, so that every document is correct and on-brand.

**Acceptance Criteria**
- **Given** company settings, **when** I save legal name, registration IDs, VAT status, addresses, and contact details, **then** they populate document headers/footers automatically.
- **Given** a logo and color choice, **when** I apply a document design, **then** generated PDFs/links reflect the branding.
- **Given** invalid/missing mandatory identity fields, **when** I try to issue a document, **then** I'm warned which fields are required for compliance.

**Technical Notes:** identity snapshot is copied onto issued documents (historical immutability).
**Test Cases:** branding on PDF; mandatory-field guard; identity change doesn't mutate past documents.

## STORY-ACC-02-002 — Tax / VAT mode
*Covers UC-ACC-02.2*

**User Story:** As an **Owner/Accountant**, I want to configure my tax/VAT mode, so that documents and reports behave correctly for my situation.

**Acceptance Criteria**
- **Given** settings, **when** I choose VAT-payer vs non-payer, **then** documents show/hide VAT and totals compute accordingly everywhere.
- **Given** tax-records vs accounting mode, **when** set, **then** available reports and document fields adapt.
- **Given** a mid-period mode change, **when** applied, **then** existing issued documents are unaffected and the change is dated/audited.

**Technical Notes:** mode is the master switch feeding EPIC-ACC-05 (VAT display) and EPIC-ACC-10 (VAT outputs).
**Test Cases:** non-payer hides VAT end-to-end; switch dated; historical docs unchanged.

## STORY-ACC-02-003 — Gapless numbering series
*Covers UC-ACC-02.3*

**User Story:** As an **Owner**, I want numbering series per document type and year, so that my documents are legally sequential.

**Acceptance Criteria**
- **Given** a document type, **when** I define a series (prefix/format/reset cadence), **then** issuing allocates the next number from it.
- **Given** concurrent issuance, **when** two documents are issued at once, **then** numbers are unique and **gapless** (no skips, no duplicates).
- **Given** a yearly reset, **when** the period rolls over, **then** numbering restarts per configuration.

**Technical Notes:** atomic sequence allocation (DB-level) under concurrency — a legal requirement, not best-effort.
**Test Cases:** concurrency stress → no gaps/dupes; format honored; yearly reset.

## STORY-ACC-02-004 — Document defaults, email templates & languages
*Covers UC-ACC-02.5, .6, .7*

**User Story:** As an **Owner**, I want default texts, email templates, and languages, so that sending documents is one click and consistent.

**Acceptance Criteria**
- **Given** defaults (payment terms, footers, notes), **when** I create a document, **then** they prefill and remain editable.
- **Given** email templates with variables, **when** I send a document/reminder, **then** the rendered email uses the template.
- **Given** multiple languages, **when** I set a document/contact language, **then** the document and email render in that language.

**Technical Notes:** template variables resolved from document/contact/company; per-document language override.
**Test Cases:** default prefill + override; template variable rendering; language switch on PDF + email.

## STORY-ACC-02-005 — Units, VAT rates, currency & rounding
*Covers UC-ACC-02.8, .9, .11*

**User Story:** As an **Owner**, I want to manage units, VAT rates, base currency, and rounding, so that line math and tax are consistent.

**Acceptance Criteria**
- **Given** settings, **when** I define units and VAT rates, **then** they're selectable on catalog items and document lines.
- **Given** a base currency and rounding rules, **when** documents total, **then** per-line precision and total rounding follow configuration.
- **Given** a VAT rate change, **when** applied, **then** new documents use it while issued documents keep their original rate.

**Technical Notes:** feeds the shared money/VAT module (EPIC-ACC-05.2); rounding mode (arithmetic/banker's) configurable.
**Test Cases:** rate selectable; rounding honored; historical rate immutability.

## STORY-ACC-02-006 — Bank accounts on documents
*Covers UC-ACC-02.10*

**User Story:** As an **Owner**, I want to set which bank account/payment details show on documents, so that customers pay to the right place.

**Acceptance Criteria**
- **Given** one or more bank accounts, **when** I mark a default, **then** issued documents show its details and drive the payment QR (EPIC-ACC-05.8).
- **Given** a foreign-currency document, **when** issued, **then** the matching-currency account is used where configured.

**Technical Notes:** account selection links to auto-match reference (EPIC-ACC-08.6) and QR (05.8).
**Test Cases:** default account on PDF; currency-matched account; QR uses correct account.

---

## Coverage
| Story | UCs |
|-------|-----|
| 001 Profile & branding | 02.1, 02.4 |
| 002 Tax/VAT mode | 02.2 |
| 003 Numbering series | 02.3 |
| 004 Defaults/templates/languages | 02.5, 02.6, 02.7 |
| 005 Units/VAT/currency/rounding | 02.8, 02.9, 02.11 |
| 006 Bank accounts on documents | 02.10 |
