# mef-BIT Paperclip hand-off — ACC (Invoicing & Accounting product)

> **Purpose:** Paste this into a **`mefistos`** claude-fleet session (mef-BIT company) to create the ACC epic + MVP
> tasks in Paperclip. The author session could not write to mef directly (mef uses better-auth, no operator key;
> writes happen only by driving a mef session). **mef was unreachable at hand-off time** — fire when the tunnel is up.

---

## 🚧 Guardrails (read first)

- **Target company = mef `BIT` "32bit branch"** (`977c0706-8e09-400e-9f6e-76e764e81cd0`), the active implementation fleet.
  **NOT htz `PAP`.** A prior attempt created this epic on htz-PAP (`PAP-303` + children) — **all cancelled.** Do not repeat.
- Confirm you are on mef-BIT before any write: this BIT company is **archived on htz**, live on mef.
- This is a **clean-room** product: copy iDoklad-class *functionality*, **no API integration with, or data import from,
  iDoklad**; the product name must not be iDoklad. (Board mandate PAP-204.)

## What to create

A parent **epic** + the **P1/MVP** task graph below. Use the `paperclip` skill for the API mechanics (issue document
with key `plan`, blockers, reassignment) and `paperclip-converting-plans-to-tasks` for assignment discipline.

**Before assigning:** look up mef-BIT's agent roster + specialties (`get_workspace_agents` / equivalent) and assign each
task to the **best-fit specialty** — do not default everything to one agent. Specialty hints are given per task; treat
them as hints, match to real agents. Surface any gap (missing skill → hire/decision) instead of papering over it.

### Parent epic
**Title:** `ACC — Invoicing & Accounting product (iDoklad-class, standalone)`
**Summary:** Standalone CZ/SK online invoicing + light-accounting SaaS. Separate backend `accounting-server (:8082)`
sharing core (`common`/`api-core`/`db`) with `api-server` — mirrors `reality-server`. Separate frontend
`@ppt/accounting-web` (Next.js + next-intl). Full functional spec: see `.research/accounting/` on branch
`feature/tender-austin-fde5e4` (commit/push to make it fetchable here): `README.md`, `PRD-ACC-001`, `epics.md`
(17 epics), `use-cases.md` (~140 UC), `architecture.md`, `stories/EPIC-ACC-05-*`.

### MVP task graph (P1) — wire `blockedByIssueIds`, don't write "blocked by" in prose

| # | Task | Specialty | Blocked by |
|---|------|-----------|------------|
| **T1** | `EPIC-ACC-17` Scaffold `crates/accounting-core` + `servers/accounting-server` from the `reality-server` skeleton; depend on `common`/`api-core`/`db`; bind `:8082`; health + OpenAPI + `/swagger-ui`; observability via `api-core`. | Backend / Rust | — |
| **T2** | `EPIC-ACC-01` Accounts & access: validate **api-server-issued JWTs** as a resource server (api-core extractors); multi-company (agendas) with RLS isolation; RBAC; 2FA; audit log; GDPR export/delete. | Backend / Rust | T1 |
| **T3** | `EPIC-ACC-02` Company & document config: profile, VAT/tax mode, gapless numbering series, document designs/branding, default texts, email templates, units, VAT rates, rounding. | Backend / Rust | T1 |
| **T4** | `EPIC-ACC-03` Contacts & CRM: CRUD+merge, registry autocomplete, VAT verification, multiple addresses, per-contact defaults, history, import/export, receivables. | Backend / Rust | T1 |
| **T5** | `EPIC-ACC-04` Price-list catalog: items (goods/services), price levels net/gross, VAT+unit, categories/tags, discounts, import/export, stock link, inline quick-add. | Backend / Rust | T1 |
| **T6** | `EPIC-ACC-05` Sales invoicing (revenue core): invoices/proforma+advance settlement/credit notes, multi-currency, discounts/VAT/rounding/precision, payment QR, PDF/ISDOC/XML, email + shareable link, attachments/tags, linking, duplicate, status lifecycle, bulk. **(13 G/W/T stories already written — see `stories/EPIC-ACC-05-sales-invoicing.md`.)** | Backend / Rust | T2, T3, T4, T5 |
| **T7** | `EPIC-ACC-16` Platform: 2FA, RBAC enforcement, daily backups+restore, data export/migration+import, GDPR DSARs, audit trail, session security. | Backend / DevOps | T1 |
| **T8** | `@ppt/accounting-web` scaffold (Next.js + next-intl sk/cs, modeled on `reality-web`) + generate `@ppt/accounting-api-client` (hey-api) from the server OpenAPI. Invoice UI follows T6. | Frontend | T1 (UI for invoices → T6) |
| **T9** | Deploy wiring: compose service, caddy route, `CORS_ALLOWED_ORIGINS`, `pmctl` worktree target → `accounting.dev.*`, port `:8082`. | DevOps | T1 |

**Parallelization:** T2–T5, T7, T9 all start once T1 is `done`. T6 waits on T2–T5. T8 starts after T1 (invoice screens after T6). Assign parallel branches concurrently (agents allow concurrent runs).

### Migration note (don't miss)
The accounting MVP currently lives **embedded in `api-server`** as `/api/v1/accounting/*` on `origin/dev` (with
`accounting.tsp`, migrations `00183/00184`). T6/T1 must **lift those handlers into `accounting-server`** and **remove
the old route from `api-server`** once green — avoid a dual-serve window in prod. (Note: dev/main are disjoint.)

### Follow-ups (name as known next epics — do **not** over-split now)
- **P2 (money in/out):** `EPIC-ACC-06` quotes/orders/delivery · `-07` purchases/expenses + AI-OCR inbox · `-08` cash & bank + auto-match · `-09` payments & dunning · `-10` VAT outputs (return, control/recap, EC sales list, OSS) · `-12` reporting/dashboard + accountant export.
- **P3 (scale & reach):** `-11` inventory · `-13` automation/recurring · `-14` public API & connectors · `-15` mobile & notifications.

## Done = 
Parent epic exists on **mef-BIT**; T1–T9 created with correct `blockedByIssueIds`; each assigned to a specialty-matched
mef-BIT agent; P2/P3 captured as named follow-ups; link back the plan doc. Report the created issue IDs.
