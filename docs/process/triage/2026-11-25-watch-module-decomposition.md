# Watch-Module Decomposition Triage — 2026-11-25

Owner role: `pm-scrum-master` · Priority: medium · Sprint: *Hardening & Refactor
— Modules Under Watch*

This note decomposes the four god-modules currently under watch — `vote.rs`,
`sso.rs`, `financial.rs`, `lease.rs` — into a concrete split plan, naming the
natural module seams, the guard tests that must exist (or be added) **before**
code moves, and the delivery owner and sequence for each. It is the coordination
artifact for the "decompose watch modules" workstream; the refactors themselves
are delivered by **pm-backend + pm-qa** (per the role plan), one module per PR.

Like [`2026-11-20-churn-hotspot-triage.md`](2026-11-20-churn-hotspot-triage.md),
this is a **planning artifact only** — no runtime code changes accompany it, and
backlog state under `.research/**` is intentionally left untouched. The sibling
change that lands with this note is the free quarantine burn-down (3 stale
registry rows removed — see *Quarantine burn-down* below).

## How to read the classification

Each module gets a **primary next action** (the same two-way split as the churn
triage):

- **Refactor (split now)** — behavior is already pinned by tests at the right
  layer (DB-backed integration suites and/or in-file unit tests), so the split
  is a structural move guarded by the current suite. Low behavioral risk.
- **Guard-then-refactor** — a security- or correctness-sensitive path is *not*
  pinned at the right layer. Add a small, targeted characterization guard first,
  then split. The rationale names the exact path to pin.

A "watch module" earns its place by **size × churn**, not by being buggy. The
payoff of the split is editing friction removed, not a defect fixed — so every
split must be behavior-preserving and the cases must run unchanged across it.

## Modules under watch

| Module | Path | LoC | `pub fn` | In-file tests | External suites | Primary action |
|---|---|---:|---:|---:|---|---|
| `vote.rs` (repo) | `backend/crates/db/src/repositories/vote.rs` | 2771 | 56 | 5 | 5 (`vote_*_tests.rs`) | **Refactor** |
| `sso.rs` (route) | `backend/servers/reality-server/src/routes/sso.rs` | 2632 | 15 | 46 | 1 (`sso_authz_tests.rs`) | **Refactor** |
| `financial.rs` (route) | `backend/servers/api-server/src/routes/financial.rs` | 2597 | 1 router + handlers | 6 | 3 (`financial_*_tests.rs`) | **Guard-then-refactor** |
| `lease.rs` (repo) | `backend/crates/db/src/repositories/lease.rs` | 2502 | 76 | 0 | 6 (`lease*_tests.rs`) | **Guard-then-refactor** |

> Sizes captured 2026-11-25 on `dev` (`3a9251fea4`). Re-measure before claiming a
> split PR; if a module has already shrunk below the watch threshold, drop it
> from the list instead of splitting for its own sake.

### 1. `vote.rs` — Refactor (split the repository by domain)

**Classification: Refactor.** A single 2.7k-line `impl VoteRepository` with 56
methods spanning six independent domains. The behavior is pinned at the right
layer — five DB-backed integration suites already exist
(`vote_cast_regression_tests.rs`, `vote_delegation_eligibility_tests.rs`,
`vote_eligible_snapshot_tests.rs`, `vote_reminder_dedup_tests.rs`,
`vote_sub_resource_org_scope_tests.rs`) — so the split is guarded by the current
suite and carries low behavioral risk.

Split the one `impl` into domain submodules behind a thin `vote/mod.rs` that
re-exports `VoteRepository` (keep the public type and method names identical so
no caller changes):

- `vote/polls.rs` — `create_poll_rls`, `find_poll_by_id*`, `get_questions*`,
  `add_question` / `update_question` / `delete_question`,
  `list_polls_by_building_rls`, `list`, `create` / `create_vote`, `find_by_id*`,
  `publish` / `cancel` / `close` / `delete`.
- `vote/eligibility.rs` — `check_eligibility*`, `user_may_vote_for_unit`,
  `get_eligible_owner_user_ids`, `question_belongs_to_vote`,
  `comment_belongs_to_vote`.
- `vote/casting.rs` — `cast_vote*`, `get_user_response*`, `get_poll_results_rls`.
- `vote/comments.rs` — `list_comments*`, `add_comment`, `list_replies*`,
  `hide_comment`.
- `vote/results.rs` — `calculate_results`, `get_results`,
  `generate_report_data`, `get_participation_report`, `count_*`.
- `vote/lifecycle.rs` — `activate_scheduled_votes`, `close_expired_votes`,
  the `*_notification` / `mark_*_notified` / `find_votes_due_for_reminder`
  schedule-and-notify helpers, `expire`-style batch jobs.
- `vote/audit.rs` — `create_audit_entry*`, `get_audit_log`.

No guard to add first: the integration suites cross the method boundaries the
split touches. **The split is its own guard** because the cases run unchanged.

### 2. `sso.rs` — Refactor (extract flows + lift the test module)

**Classification: Refactor.** This route is the most heavily *in-file*-tested of
the four (46 unit tests, ~1000 LoC of tests — ~38 % of the file) and already
carries explicit section banners. Two moves, in order:

1. **Lift the test module out** into `sso/tests/` submodules grouped by the
   banners that already exist (`pending-session prune`, `capacity-cap`,
   `PM-role derivation security (#2249)`, `cookie security (P0-12)`,
   `SSO consumer security (#820 / PR #921)`). This alone removes ~1000 LoC of
   editing friction from the hot path and is pure structural movement — the
   tests run identically.
2. **Extract the production flows** behind a thin `sso/mod.rs`:
   - `sso/web.rs` — *Web SSO Flow* (banner @62).
   - `sso/mobile.rs` — *Mobile Deep-Link SSO* (banner @571).
   - `sso/session.rs` — *Session Management* (banner @728) + *Token Exchange &
     Session Sync* (Story 96.3, banner @1016).
   - `sso/cookies.rs` — *Cookie helpers* (banner @1540) + the cookie-security
     helpers the P0-12 tests pin.
   - `sso/types.rs` — *Types* (banner @780) + *Helper Functions* (banner @848).

Behavior is pinned both in-file and by `sso_authz_tests.rs`. The
**PM-role-derivation (#2249)** and **cookie-security (P0-12)** paths are
security-sensitive, but they are exactly the paths the in-file tests already
cover densely — move the test with the code it guards in the same commit so the
guard never lags the split.

### 3. `financial.rs` — Guard-then-refactor (split the route by resource)

**Classification: Guard-then-refactor.** A 2.6k-line route module already
banner-organized by resource (Account / Fee Schedule / Invoice / Payment /
Reminder-Late-Fee / Report / Financial Statement handlers). The seams are
obvious; the gap is the **coverage layer**. Only 6 in-file tests back the whole
module, and the external suites are happy-path + one IDOR suite
(`financial_happy_path_tests.rs`, `financial_reports_happy_path_tests.rs`,
`financial_cross_org_idor_tests.rs`).

**Guard first (do before moving code):** the money-touching, cross-org-sensitive
paths are Payment and Invoice. `financial_cross_org_idor_tests.rs` pins
cross-org denial but not the per-handler RLS-connection routing a split is most
likely to silently drop (cf. PR #472 dropping `RlsConnection` on 5 handlers —
the exact failure mode a route split invites). Add 2–3 DB-backed
characterization tests asserting that an invoice/payment write **on the RLS
connection** succeeds for the owning org and that a cross-org write is denied at
the DB layer (not just the handler), **then** split:

- `financial/types.rs` — *Request/Response Types* (banner @257).
- `financial/accounts.rs` — *Account Handlers* (@509).
- `financial/fee_schedules.rs` — *Fee Schedule Handlers* (@719).
- `financial/invoices.rs` — *Invoice Handlers* (@862).
- `financial/payments.rs` — *Payment Handlers* (@1229) + *Reminder/Late Fee*
  (@1586).
- `financial/reports.rs` — *Report Handlers* (@1663) + *Financial Statement
  Handlers* (Story 11.7, @1685).

### 4. `lease.rs` — Guard-then-refactor (highest guard priority)

**Classification: Guard-then-refactor — this is the riskiest of the four.** A
2.5k-line `impl LeaseRepository` with 76 methods and **zero in-file tests**. The
module has a deliberate two-layer shape: `*_rls` methods that take an executor
(the RLS-safe path) and thin legacy wrappers that call them on the pool. Six
external suites exist (`leases_authz_tests.rs`, `lease_review_rbac_tests.rs`,
`lease_send_for_signature_rbac_tests.rs`, `lease_units_join_tests.rs`,
`lease_abstraction_forms_tests.rs`, `leases_core_backfill_batch1_tests.rs`), but
coverage is uneven across the six domains and the **scope-helper** methods
(`lease_org_rls`, `template_org_rls`, `application_scope_rls`,
`screening_scope_rls`, `payment_scope_rls`) are the tenancy-isolation gate — a
split that mis-wires one of them is a cross-org data leak, not a compile error.

**Guard first:** before moving anything, confirm each of the five `*_scope_rls`
helpers is pinned by an authz test asserting cross-org denial; add the missing
ones (`leases_authz_tests.rs` is the right home). Only then split behind
`lease/mod.rs`:

- `lease/scope.rs` — the five `*_scope_rls` helpers (move first, with their new
  guards, as the foundation every other submodule calls).
- `lease/applications.rs` — `*_application*` (create/find/update/submit/review/
  delete/list/count), both `_rls` and legacy wrappers.
- `lease/screening.rs` — `*_screening*` (initiate/submit_consent/start/
  update_result/get_screenings).
- `lease/templates.rs` — `*_template*` (create/find/update/list/count/delete).
- `lease/leases.rs` — core lease CRUD + lifecycle (create/find/update/
  mark_sent_for_signature/terminate/renew/list/count/get_with_details,
  `expire_ended_leases`).
- `lease/amendments.rs` — `create_amendment*`, `list_amendments*`.
- `lease/payments.rs` — `generate_payment_schedule*`, `record_payment*`,
  `get_overdue_payments*`, `list_upcoming_payments*`, reminders
  (`create_reminder*`, `list_pending_reminders*`).
- `lease/reporting.rs` — `get_expiration_overview*`, `get_statistics*`.

## Suggested sequencing

Lowest-risk first; one module per PR, `pm-backend` + `pm-qa` pairing:

1. **`sso.rs`** — lift tests out, then extract flows. Pure structural; best
   in-file guard of the four. Lowest risk → do first to validate the thin-`mod`
   re-export pattern the other three reuse.
2. **`vote.rs`** — domain split guarded by the existing five integration suites.
3. **`financial.rs`** — add the 2–3 RLS/IDOR characterization tests, then split
   by resource.
4. **`lease.rs`** — pin the five `*_scope_rls` helpers first (highest guard
   priority), then split with `lease/scope.rs` as the foundation. Do last.

Each PR must keep the public type and method names identical (re-export from the
new `mod.rs`) so no caller outside the module changes — that keeps the diff
reviewable and the split verifiably behavior-preserving.

## Quarantine burn-down

Per the [burn-down cadence](../../../.github/quarantine/cadence.md) (#3774), the
free "registry hygiene" step was taken alongside this note:

- **3 stale registry rows removed** from `.github/quarantine/registry.toml` —
  their tests have already been released (no `#[ignore]`), so the rows were pure
  census drift:
  - `…/rag_similarity_search_tests.rs::rag_retrieval_correctness`
  - `…/rag_similarity_search_tests.rs::search_documents_pgvector_model_filter_provenance`
  - `…/integrations_marketplace_reviews_tests.rs::get_feature_stats_returns_200`
- Registry now **198 rows**, matching the live `#[ignore]` count
  (`check-quarantine-age.py --summary` → `total=198`). The gate's stale-row
  WARNs are cleared and it passes (`PASS: 198 quarantined test(s)`).

The substantive burn-down (pm-backend's BIT-440 cluster, pm-devops Redis lane,
pm-security TOCTOU test) is unchanged and remains owned per the cadence doc;
`need_per_week` holds at ~16 for the 2027-02-19 deadline. This note does not bump
any deadline — a bump is a cadence failure, not a cadence tool.
