# Invoicing & Accounting Product — Functional Spec (`ACC`)

Generalized, **vendor-neutral** functional specification for a standalone online **invoicing + light-accounting**
SaaS (freelancers / sole traders / SMBs + their accountants). Derived from competitor analysis and abstracted to
functionality only — **no dependency on, or integration with, any specific product**.

## Contents

| Doc | What it is |
|-----|------------|
| [`PRD-ACC-001-invoicing-accounting.md`](PRD-ACC-001-invoicing-accounting.md) | Product requirements — goals, success metrics, FRs (§5.1), NFRs (§5.2), scope, dependencies, phasing. The keystone / BMAD input. |
| [`architecture.md`](architecture.md) | Backend topology decision — separate **`accounting-server` (`:8082`)** sharing `common`/`api-core`/`db` core with `api-server`, like `reality-server`. |
| [`epics.md`](epics.md) | 17 epics (`EPIC-ACC-01..17`) with business value, actors, stories→UC, dependencies, risks + epic↔UC map. |
| [`use-cases.md`](use-cases.md) | ~140 use cases (`UC-ACC-XX.Y`, Actor + Description) across 16 categories, with actor hierarchy. |
| [`stories/`](stories/) | Build-ready Given/When/Then stories for the **full MVP**: ACC-01, -02, -03, -04, -05 (13), -16, -17. |

## Traceability

```
PRD-ACC-001
   └─ EPIC-ACC-01..16
        └─ UC-ACC-XX.Y   (FR-ACC-XX.Y ⇔ UC-ACC-XX.Y)
             └─ STORY-ACC-XX-NNN  (G/W/T acceptance criteria)
```

## Scope at a glance

**In:** accounts/multi-company, configuration, contacts/CRM, price-list catalog, sales invoicing (incl. proforma/advance,
credit notes, multi-currency, QR, email/link), quotes/orders/delivery, purchases/expenses with AI-OCR inbox, cash & bank
with auto-matching, payments & dunning, statutory VAT outputs, light inventory, reporting/dashboard, automation/recurring,
public API & connectors, mobile & notifications, platform security/GDPR/backup.

**Out (v1):** full double-entry general ledger & statutory close, payroll, direct payment initiation/bank certifications,
advanced manufacturing/logistics, and any tie to the analyzed source product.

## Suggested build order

1. **P1 / MVP** — ACC-01, -02, -03, -04, -05, -16
2. **P2 / Money in-out** — ACC-06, -07, -08, -09, -10, -12
3. **P3 / Scale & reach** — ACC-11, -13, -14, -15

## Status / next steps

- [x] Use-case catalog · [x] Epics catalog · [x] PRD · [x] Architecture (separate `accounting-server`)
- [x] **Stories for the full MVP** — ACC-01, -02, -03, -04, -05, -16, -17 (Given/When/Then)
- [x] mef-BIT Paperclip hand-off brief → [`mef-agent-brief.md`](mef-agent-brief.md)
- [ ] **Create the epic in mef-BIT Paperclip** (drive a `mefistos` session — needs claude-fleet + mef reachable)
- [ ] Stories for P2/P3 epics (ACC-06..15) — same template
- [ ] Promote into `docs/` (or the `@ppt/accounting-web` repo) once direction is confirmed
