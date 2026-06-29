# EPIC-ACC-03 — Contacts & CRM · Stories

> Covers `UC-ACC-03.1–.11`. **Shared DoD:** AC pass · tests green · per-company isolation · audit-log on mutation · i18n externalized · mobile parity where flagged.

---

## STORY-ACC-03-001 — Contact CRUD, merge & deactivate
*Covers UC-ACC-03.1, .5*

**User Story:** As a **User**, I want to create and maintain customers/suppliers, so that documents reference accurate parties.

**Acceptance Criteria**
- **Given** the directory, **when** I create a contact (type customer/supplier/both) with identifiers and addresses, **then** it is saved and selectable on documents.
- **Given** duplicates, **when** I merge them, **then** documents/history re-point to the surviving contact with no data loss.
- **Given** an inactive contact, **when** I deactivate it, **then** it's hidden from pickers but its historical documents remain intact.

**Technical Notes:** merge re-keys foreign references transactionally; soft-deactivate, never hard-delete referenced contacts.
**Test Cases:** merge preserves history; deactivated hidden from picker but documents intact; customer+supplier dual role.

## STORY-ACC-03-002 — Registry autocomplete & VAT verification
*Covers UC-ACC-03.2, .3*

**User Story:** As a **User**, I want to autofill from the business registry and verify VAT numbers, so that I create contacts fast and correctly.

**Acceptance Criteria**
- **Given** a company name/ID, **when** I search the registry, **then** official details autofill into the contact.
- **Given** a VAT number, **when** I verify it, **then** validity/registration status is shown and stored with a timestamp.
- **Given** the registry/verification service is down, **when** I look up, **then** I can still save manually and the system degrades gracefully (no hard block).

**Technical Notes:** external dependency (EPIC-ACC-14); cache results; never block contact creation on third-party outage.
**Test Cases:** autofill maps fields; invalid VAT flagged; offline-degradation path.

## STORY-ACC-03-003 — Addresses & per-contact defaults
*Covers UC-ACC-03.4, .9*

**User Story:** As a **User**, I want multiple addresses and per-contact defaults, so that documents prefill correctly per customer.

**Acceptance Criteria**
- **Given** a contact, **when** I add billing and one or more delivery addresses, **then** I can pick which appears on a document.
- **Given** per-contact defaults (currency, price level, payment terms, discount), **when** I create a document for them, **then** those defaults prefill.

**Technical Notes:** defaults feed EPIC-ACC-05 document creation and EPIC-ACC-04 price levels.
**Test Cases:** address selection on document; defaults prefill + override.

## STORY-ACC-03-004 — Tags & segments
*Covers UC-ACC-03.6*

**User Story:** As a **User**, I want to tag/segment contacts, so that I can filter and act in bulk.

**Acceptance Criteria**
- **Given** contacts, **when** I apply tags, **then** I can filter the directory and reports by tag.
- **Given** a segment filter, **when** I run a bulk action (e.g., export), **then** it applies to exactly the filtered set.

**Test Cases:** tag filter accuracy; bulk action scoping.

## STORY-ACC-03-005 — Transaction & communication history
*Covers UC-ACC-03.7, .8*

**User Story:** As a **User/Accountant**, I want a single view of each contact's documents and communications, so that I understand the relationship.

**Acceptance Criteria**
- **Given** a contact, **when** I open it, **then** I see all issued/received documents and payments with status.
- **Given** a contact, **when** I view communication history, **then** I see emails/documents sent with timestamps (source: EPIC-ACC-05.10).

**Test Cases:** history completeness; payment status reflected; comms entries on send.

## STORY-ACC-03-006 — Import/export & receivables
*Covers UC-ACC-03.10, .11*

**User Story:** As an **Owner/Accountant**, I want to import/export contacts and see receivables per contact, so that onboarding and collections are easy.

**Acceptance Criteria**
- **Given** a file, **when** I import contacts, **then** rows validate, duplicates are flagged, and a result summary is shown.
- **Given** the directory, **when** I export, **then** I get a complete xlsx/csv.
- **Given** a contact, **when** I view receivables, **then** outstanding balance (and optional credit limit) are shown and a limit breach can warn at document time.

**Technical Notes:** import dedupe on identifiers; receivables derive from open documents (EPIC-ACC-09.7).
**Test Cases:** import validation/dedupe; export completeness; credit-limit warning.

---

## Coverage
| Story | UCs |
|-------|-----|
| 001 CRUD/merge/deactivate | 03.1, 03.5 |
| 002 Registry & VAT verify | 03.2, 03.3 |
| 003 Addresses & defaults | 03.4, 03.9 |
| 004 Tags & segments | 03.6 |
| 005 History | 03.7, 03.8 |
| 006 Import/export & receivables | 03.10, 03.11 |
