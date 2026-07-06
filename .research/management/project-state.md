# PPT Project State

_Generated: 2026-07-06 — daily PM rotation (Scrum Master + pm-security; routine refresh). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 12 → 0 (wrap)._

## Executive summary

- **Hardening flood, no new epic movement.** All 14 merged PRs since the last routine run are follow-ups from post-merge review (`follow-up` + `from-merged-review` labels; issues #2082–#2087, #2102–#2110). No new epic/story surface shipped — this run's picture is "burn the review-follow-up backlog to zero" rather than "sprint velocity".
- **One real security bug closed.** #2120 (issue #2107) moved outage-mutation authorization from JWT `role` claim to DB-validated `organization_members.role_type` on 6 handlers in `backend/servers/api-server/src/routes/outages.rs`. Second time a JWT-vs-DB-role trust gap has surfaced after `security-rls-migration-residual` was marked `done` (2026-05-23) — the audit needs to widen. See pm-security below.
- **quick-xml XXE pin is now hard-enforced** (#2096 + #2111): `backend/deny.toml` bans every version other than `=0.41.0` and CODEOWNERS gates `deny.toml`. The advisory `.github/workflows/pinned-dependency-guard.yml` was removed; enforcement moved to the existing `cargo-deny` CI job. No path for a silent pin cross.
- **Accounting-export honesty invariant fully sealed** (#2099 → #2117): smart constructor is now the only way to build `SlovakAccountingExport`, taking a named-field input struct so `total_revenue`↔`total_receivables` and `total_expenses`↔`total_payables` can no longer be transposed at a call site. Closes the #2030 chain.
- **Test un-quarantines + guards** (#2097, #2098, #2113, #2116): #1771 soft-delete unread invariant is now a live DB-backed test (BIT-351 quarantine lifted); currency/country enum-sync guards now catch a variant added to the enum but omitted from the canonical wire list (compile-time exhaustiveness via `Self::ALL`); template pagination now covers the default `include_system=true` tie-break through BitmapOr/Sort.
- **Dispatcher hardening** (#2114): MCP-push size guard + action-list reconcile now run in Phase 6 before push; T26 promoted from `warn` to `fail`. #1014 truncation vector closed.
- **Mobile-native ListingDetail auth-transition fix** (#2115): ViewModel re-key was `remember(sessionToken, listingId)` — an auth change reset the screen to the full-screen spinner. Fix: keyed on `listingId` only; a new `updateAuth(sessionToken)` rebuilds auth-scoped repositories without resetting the loaded listing.
- **Mobile RN screens wired to live API** (#2118): Meters/Leases/Forms/Threads screens moved off mock data onto `@ppt/api-client`. 12 files touched, 4 new test files.
- **Exec-bit drift recurrence gate** (#2119): `.github/workflows/script-exec-bit-gate.yml` now fails PRs whose `.github/workflows/scripts/*.sh` files aren't 100755. Structural fix for the #2081/#2110 push-via-API-flattens-mode class.

## Sprint progress

Sprint scope from the last active window (Epic 6 / 7A / 8A / 10A) is unchanged — no in-flight story landed this run. Delivery is currently review-debt-driven, not sprint-story-driven. Recommend the Scrum Master call out sprint status in the next brief when a real sprint story merges.

| Epic | Tracked status | Delta this run |
|---|---|---|
| 10A — OAuth Provider Foundation | in-progress | no change; 3 stories still `ready-for-dev`. pm-security flags this as the highest-risk unstarted slice. |
| Accounting / regional compliance | in-progress | honesty invariant sealed (#2030 chain closed via #2117); enum-sync guards hardened (#2113). |
| Mobile-native (Reality KMP) | in-progress | ListingDetail auth-transition regression fixed (#2115). |
| Mobile RN (property management) | in-progress | Meters/Leases/Forms/Threads wired to live API + tests (#2118). |
| Backend authz hardening | in-progress | outages moved to DB-validated role (#2120); JWT-role residue audit still open. |

## pm-security — rotation deep-dive (2026-07-06)

### Shipped-this-week that we care about
- **#2120** — 6 outage-mutation handlers now gate on `RlsConnection::role().is_manager()` (DB-validated) rather than the JWT `role` claim. Removes a stale-claim / privilege-escalation vector.
- **#2096 + #2111** — `quick-xml` XXE/billion-laughs pin enforced via `cargo-deny` hard bans, code-owner-gated `deny.toml`.
- **#2117 + #2099** — accounting-export smart constructor with named-field input kills same-type-transposition on financial data.
- **#2098 + #2113** — currency/country enum-sync guard hardened with compile-time exhaustiveness; reduces silent drift on input-validation allow-lists.
- **#2114** — dispatcher push-payload size bounded; input-surface hygiene.

### Still worrying
- The outage-handler fix (#2120) is the **second** time a JWT-claim-vs-DB-role trust gap has surfaced after `security-rls-migration-residual` was marked `done` on 2026-05-23 — that closure covered voting/market_pricing/faults/notif_prefs/reports but missed outages. A repo-wide sweep for remaining `TenantExtractor::role` (JWT claim) call sites outside the migrated set is needed, not just outages.
- OAuth Provider Foundation (epic-10a: authorization-server, client-registration, token-management) is still `ready-for-dev` — no stories started this sprint despite a security-relevant surface (PKCE/token issuance) and a backlog history of closed-unmerged PKCE hardening PRs (#908).
- No PR this batch touched `TenantExtractor`/JWT trust-boundary code directly besides #2120 — good, but means the audit above is still open and unverified.

### Top-3 next asks
1. Run a repo-wide sweep for remaining `TenantExtractor::role` (JWT-claim) authorization checks outside `outages.rs` and file issues per hit — dependency: pm-security, ref PR #2120 / issue #2107.
2. Confirm `security-rls-migration-residual` backlog item's "done" scope actually enumerated all mutation handlers (not just the 5 named domains) before treating it as closed — `.research/backlog.md` line 62.
3. Prioritize epic-10a OAuth Provider stories (10a-1/2/3, all `ready-for-dev`) for next sprint given the PKCE-hardening history (#908) and the fact no OAuth code shipped this rotation — owner: pm-security + backend.

### Confidence
medium — grounded in direct file reads of `outages.rs` and `deny.toml` confirming the described fixes; smaller security-adjacent changes may be under-weighted (14 PRs total, not every one diffed).

## Rolling delivery indicators

- **PRs merged since last routine run:** 14 (all follow-ups; none are new sprint stories).
- **Open action-list items:** ~10 (churn hotspots, coverage gaps, review-follow-ups).
- **Risks open:** 42 (unchanged this run — dispatcher-maintained).
- **Backlog vectors added this run:** see `backlog.md` deltas.
- **Cursors advanced:** `pm_cursor.next_index` 5 → 6 (pm-data next); `coverage_cursor.next_index` 12 → 0 (wrap — all 13 distinct epics covered once).
