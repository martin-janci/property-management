# EPIC-ACC-05 — Sales Invoicing · Stories

> Worked exemplar story set for the flagship epic. Format follows the repo story convention (User Story · Acceptance
> Criteria G/W/T · Technical Notes · UI/UX Notes · Test Cases · Definition of Done). Covers all of `UC-ACC-05.1–.20`.
> The same pattern applies to the other epics — replicate per `epics.md`.

**Conventions:** `STORY-ACC-05-NNN`. A document's lifecycle status: `draft → issued → sent → (partially_)paid / overdue / cancelled`.
**Shared Definition of Done (all stories):** acceptance criteria pass; unit + integration tests green; multi-tenant isolation verified; audit-log entry written on mutation; i18n strings externalized; mobile parity where the flow is mobile-flagged; no regression in VAT/rounding math.

---

## STORY-ACC-05-001 — Create & issue an invoice
*Covers UC-ACC-05.1, .9, .17*

**User Story:** As a **User**, I want to create and issue an invoice to a customer, so that I can request payment with a compliant, numbered document.

**Acceptance Criteria**
- **Given** an active company with a configured numbering series, **when** I create an invoice, select a contact, and set issue/due/supply dates, **then** the document is saved as `draft` with all customer and company details auto-filled.
- **Given** a valid draft with at least one line, **when** I issue it, **then** it receives the next gapless number from its series, status becomes `issued`, and the issued document is immutable except via a corrective document.
- **Given** an issued invoice, **when** I preview it, **then** a print-ready PDF renders with branding, totals, payment details, and legally required fields.
- **Given** a draft missing a contact or any line, **when** I attempt to issue, **then** issuance is blocked with field-level validation errors.

**Technical Notes:** numbering allocation must be atomic and gap-free under concurrency (sequence per series/period). Issued snapshots persist company/contact data so later edits to master records don't mutate historical documents.
**UI/UX Notes:** single-screen editor; "Save draft" vs "Issue"; status badge; mobile-flagged.
**Test Cases:** concurrent issue → no duplicate/gap numbers; issue with empty lines blocked; PDF contains all mandatory fields; issued doc is read-only.

---

## STORY-ACC-05-002 — Compose lines with discounts, VAT, rounding & precision
*Covers UC-ACC-05.2, .3, .19, .20*

**User Story:** As a **User**, I want to add line items with quantities, prices, discounts and VAT, so that the invoice totals and tax are calculated correctly.

**Acceptance Criteria**
- **Given** the line editor, **when** I add a catalog item, **then** description, unit price, unit, and VAT rate prefill; **and when** I enter an ad-hoc line, **then** I can set all fields manually.
- **Given** lines with quantities and prices, **when** totals compute, **then** net, VAT (grouped per rate), and gross are correct and recomputed live on any change.
- **Given** a line or document discount, **when** applied, **then** it reduces the taxable base before VAT and is shown explicitly.
- **Given** configured precision (up to 4 decimals) and total-rounding rules, **when** the document totals, **then** per-line precision and the rounding adjustment line both honor configuration.

**Technical Notes:** single deterministic money/VAT calculation module shared by web, mobile, and API; document-level vs line-level VAT computation modes; banker's vs arithmetic rounding configurable.
**UI/UX Notes:** live totals panel with VAT breakdown by rate.
**Test Cases:** mixed-rate VAT grouping; discount-before-VAT; rounding line appears/nets to zero; 4-dp precision retained.

---

## STORY-ACC-05-003 — Issue in foreign currency
*Covers UC-ACC-05.4*

**User Story:** As a **User**, I want to issue an invoice in a foreign currency, so that I can bill international customers correctly.

**Acceptance Criteria**
- **Given** a currency other than base, **when** I create the invoice, **then** I can set or auto-fetch the exchange rate for the supply date.
- **Given** a foreign-currency document, **when** it totals, **then** amounts show in document currency with the base-currency equivalent and rate recorded for VAT/reporting.
- **Given** no rate is available, **when** I issue, **then** I am prompted to enter one (issuance blocked without a rate).

**Technical Notes:** persist rate + source on the document; base-currency conversion drives reporting and VAT records. Depends on FX source (EPIC-ACC-13.7).
**Test Cases:** rate auto-fetch by date; manual override; base-equivalent on PDF and in reports.

---

## STORY-ACC-05-004 — Proforma / advance + tax settlement document
*Covers UC-ACC-05.5, .6*

**User Story:** As a **User**, I want to issue a proforma/advance request and later settle it with a tax document, so that I can take prepayments compliantly.

**Acceptance Criteria**
- **Given** a contact, **when** I issue a proforma/advance, **then** a non-tax request-for-payment is produced (no VAT liability yet) with its own numbering.
- **Given** a paid advance, **when** I create the tax/settlement document, **then** VAT is recognized on the advance and the final invoice deducts the settled advance from the amount due.
- **Given** a final invoice with a linked settled advance, **when** totals compute, **then** the remaining balance is correct and the documents are linked (UC-ACC-05.15).

**Technical Notes:** advance VAT recognition on payment; settlement deduction logic; chain proforma → payment → tax document → final invoice.
**Test Cases:** advance with/without VAT mode; partial advance settlement; double-settlement prevented.

---

## STORY-ACC-05-005 — Credit note / corrective document
*Covers UC-ACC-05.7*

**User Story:** As a **User**, I want to issue a credit note against an invoice, so that I can correct or cancel it without altering the original.

**Acceptance Criteria**
- **Given** an issued invoice, **when** I create a corrective document, **then** it references the original, supports full or partial correction, and reverses the corresponding VAT.
- **Given** a credit note, **when** issued, **then** the original remains immutable, balances/receivables update, and both documents show the link.
- **Given** a fully credited invoice, **when** viewed, **then** its effective payable is zero and status reflects the correction.

**Technical Notes:** signed corrective amounts; VAT reversal flows into VAT records (EPIC-ACC-10); receivables recompute (EPIC-ACC-09).
**Test Cases:** partial vs full credit; VAT reversal correct; original untouched; aging updates.

---

## STORY-ACC-05-006 — Payment QR / code on documents
*Covers UC-ACC-05.8*

**User Story:** As a **User**, I want a scannable payment code on the invoice, so that customers can pay quickly and correctly.

**Acceptance Criteria**
- **Given** an issued invoice with a bank account, **when** the PDF/link renders, **then** a payment QR encodes amount, currency, account, and a structured payment reference.
- **Given** a foreign-currency or zero-balance document, **when** rendered, **then** the code reflects the actual amount due (or is omitted when nothing is owed).

**Technical Notes:** standards-based payment-code generation; reference matches the value used by auto-match (EPIC-ACC-08.6).
**Test Cases:** QR decodes to correct amount/reference; omitted when paid; correct on partial balance.

---

## STORY-ACC-05-007 — Send invoice by email
*Covers UC-ACC-05.10*

**User Story:** As a **User**, I want to email an invoice to a customer from a template, so that delivery is one click and on-brand.

**Acceptance Criteria**
- **Given** an issued invoice and a contact email, **when** I send, **then** a templated email goes out with the PDF (and/or link), and status advances to `sent`.
- **Given** a send, **when** it completes, **then** the event is recorded in the contact's communication history (UC-ACC-03.8) with timestamp and recipient.
- **Given** a send failure/bounce, **when** it occurs, **then** the user is notified and can retry; status does not falsely show `sent`.
- **Given** a company with custom SMTP configured, **when** I send, **then** the message uses that SMTP.

**Technical Notes:** async send + delivery-status callback; template variables; optional custom SMTP per company.
**Test Cases:** template render; bounce handling; history entry; custom-SMTP path.

---

## STORY-ACC-05-008 — Send via shareable link & pay online
*Covers UC-ACC-05.11*

**User Story:** As a **User**, I want to share a secure link where the customer can view and pay the invoice, so that I get paid faster without attachments.

**Acceptance Criteria**
- **Given** an issued invoice, **when** I generate a share link, **then** a unique, hard-to-guess, optionally expiring URL renders a read-only view of the document.
- **Given** online payment is enabled (EPIC-ACC-09.2), **when** the customer opens the link, **then** a Pay action initiates a gateway payment; on success the invoice auto-marks paid (UC-ACC-09.3).
- **Given** a revoked/expired link, **when** opened, **then** access is denied.

**Technical Notes:** capability-token URLs, no auth required to view; rate-limit/abuse protection; gateway webhook settles the document.
**Test Cases:** link entropy/expiry/revocation; view records no PII leakage; pay→auto-settle.

---

## STORY-ACC-05-009 — Export invoice (PDF / ISDOC / XML)
*Covers UC-ACC-05.12*

**User Story:** As a **User/Accountant/System**, I want to export documents in standard electronic formats, so that they integrate with other systems and the accountant's tools.

**Acceptance Criteria**
- **Given** any issued document, **when** I export, **then** I can produce PDF and a standard structured format (e.g., ISDOC/XML) that validates against its schema.
- **Given** a bulk selection, **when** I export, **then** a combined archive is produced (ties to UC-ACC-05.18).
- **Given** the public API, **when** a document is requested, **then** the same export formats are available (EPIC-ACC-14).

**Technical Notes:** schema-valid XML; deterministic PDF; shared export service across UI/API.
**Test Cases:** XML schema validation; round-trip into a consumer; bulk archive integrity.

---

## STORY-ACC-05-010 — Attachments & tags
*Covers UC-ACC-05.13, .14*

**User Story:** As a **User**, I want to attach files and tag documents, so that I keep supporting evidence together and can filter/report.

**Acceptance Criteria**
- **Given** a document, **when** I attach a file, **then** it is stored, virus-scanned, size/type-validated, and downloadable from the document.
- **Given** documents, **when** I apply tags, **then** I can filter and report lists by tag (feeds EPIC-ACC-12).

**Technical Notes:** object storage with per-tenant scoping; tag taxonomy reused across document types.
**Test Cases:** disallowed type rejected; attachment tenant-isolated; tag filter accuracy.

---

## STORY-ACC-05-011 — Link related documents
*Covers UC-ACC-05.15*

**User Story:** As a **User**, I want related documents linked (proforma ↔ invoice ↔ credit note ↔ delivery note), so that I can navigate the full chain.

**Acceptance Criteria**
- **Given** documents created via conversion or settlement, **when** I view one, **then** all linked documents are listed and navigable with their relationship type.
- **Given** a linked chain, **when** I open it, **then** a visualization shows the relationships and current statuses.

**Technical Notes:** typed relationship edges; cycle-safe; drives advance settlement (05.6) and conversions (EPIC-ACC-06).
**Test Cases:** link integrity across conversions; visualization correctness; deletion rules on linked drafts.

---

## STORY-ACC-05-012 — Duplicate invoice
*Covers UC-ACC-05.16*

**User Story:** As a **User**, I want to duplicate an existing invoice, so that I can quickly create similar ones.

**Acceptance Criteria**
- **Given** any document, **when** I duplicate it, **then** a new `draft` is created with copied lines/terms, fresh dates, no number, and no payment/links carried over.
- **Given** a duplicate, **when** I issue it, **then** it allocates its own number independently.

**Technical Notes:** deep-copy of lines, shallow reference to contact/catalog; never copy status/payments/number.
**Test Cases:** dates reset; status `draft`; no inherited payments/links.

---

## STORY-ACC-05-013 — Bulk document actions
*Covers UC-ACC-05.18*

**User Story:** As a **User/Owner**, I want to act on many documents at once (send, export, print, tag), so that I save time on routine batches.

**Acceptance Criteria**
- **Given** a multi-selection, **when** I choose a bulk action, **then** it applies to all eligible items and reports a per-item success/failure summary.
- **Given** mixed-eligibility selection (e.g., a draft cannot be "sent"), **when** I run the action, **then** ineligible items are skipped with a clear reason, not silently dropped.

**Technical Notes:** async batch with progress + partial-failure reporting; idempotent retries.
**Test Cases:** partial-failure summary; ineligible-skip reasons; large-batch progress.

---

## Story Index → Use-Case Coverage

| Story | Use cases |
|-------|-----------|
| 001 Create & issue | 05.1, 05.9, 05.17 |
| 002 Lines/VAT/rounding | 05.2, 05.3, 05.19, 05.20 |
| 003 Foreign currency | 05.4 |
| 004 Proforma/advance settlement | 05.5, 05.6 |
| 005 Credit note | 05.7 |
| 006 Payment QR | 05.8 |
| 007 Send by email | 05.10 |
| 008 Shareable link + pay | 05.11 |
| 009 Export PDF/ISDOC/XML | 05.12 |
| 010 Attachments & tags | 05.13, 05.14 |
| 011 Document linking | 05.15 |
| 012 Duplicate | 05.16 |
| 013 Bulk actions | 05.18 |

All 20 `UC-ACC-05.*` use cases covered by 13 stories.
