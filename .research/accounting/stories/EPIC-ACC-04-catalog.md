# EPIC-ACC-04 — Product & Price-List Catalog · Stories

> Covers `UC-ACC-04.1–.8`. **Shared DoD:** AC pass · tests green · per-company isolation · audit-log on mutation · i18n externalized.

---

## STORY-ACC-04-001 — Item CRUD with VAT & unit
*Covers UC-ACC-04.1, .3*

**User Story:** As a **User**, I want to maintain catalog items (goods/services), so that I add consistent lines to documents.

**Acceptance Criteria**
- **Given** the catalog, **when** I create an item with name, code, description, unit, and VAT rate, **then** it's saved and selectable on document lines.
- **Given** an item, **when** I edit/deactivate it, **then** existing documents that used it are unaffected (historical snapshot on the line).

**Technical Notes:** document lines snapshot item values at insert time; deactivate ≠ delete when referenced.
**Test Cases:** item selectable on line; edit doesn't mutate past documents; goods vs service.

## STORY-ACC-04-002 — Pricing: levels, net/gross & discounts
*Covers UC-ACC-04.2, .5*

**User Story:** As a **User**, I want price levels, net/gross pricing, and discounts, so that pricing is correct per customer.

**Acceptance Criteria**
- **Given** an item, **when** I define multiple price levels (net and/or gross), **then** the correct level applies based on the contact's default (EPIC-ACC-03.9).
- **Given** item- or contact-level discounts, **when** added to a document, **then** they reduce the taxable base before VAT and show explicitly.

**Technical Notes:** net↔gross derivation uses the item's VAT rate; level resolution order documented.
**Test Cases:** level selection by contact; discount-before-VAT; net/gross round-trip.

## STORY-ACC-04-003 — Categories & tags
*Covers UC-ACC-04.4*

**User Story:** As a **User**, I want to categorize/tag items, so that I can organize and report on the catalog.

**Acceptance Criteria**
- **Given** items, **when** I assign categories/tags, **then** I can filter the catalog and reports by them.

**Test Cases:** filter accuracy; reporting breakdown by category.

## STORY-ACC-04-004 — Import / export price list
*Covers UC-ACC-04.6*

**User Story:** As an **Owner**, I want to import/export the price list, so that I onboard and maintain it in bulk.

**Acceptance Criteria**
- **Given** a file, **when** I import items, **then** rows validate, duplicates (by code) flag, and a result summary shows.
- **Given** the catalog, **when** I export, **then** I get a complete xlsx/csv.

**Test Cases:** import validation/dedupe by code; export completeness.

## STORY-ACC-04-005 — Stock link & inline quick-add
*Covers UC-ACC-04.7, .8*

**User Story:** As a **User**, I want to link items to stock and quick-add items from a document line, so that I track inventory and keep flow fast.

**Acceptance Criteria**
- **Given** an item, **when** I mark it stock-tracked, **then** issuing/receiving documents move stock (EPIC-ACC-11).
- **Given** the line editor, **when** I type an unknown item, **then** I can create it inline and it's saved to the catalog.

**Technical Notes:** stock link is the join to EPIC-ACC-11; inline-create persists a full catalog item.
**Test Cases:** stock-tracked flag drives movement; inline-created item persists & reusable.

---

## Coverage
| Story | UCs |
|-------|-----|
| 001 Item CRUD + VAT/unit | 04.1, 04.3 |
| 002 Pricing & discounts | 04.2, 04.5 |
| 003 Categories & tags | 04.4 |
| 004 Import/export | 04.6 |
| 005 Stock link & quick-add | 04.7, 04.8 |
