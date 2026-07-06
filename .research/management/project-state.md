# PPT Project State

_Generated: 2026-07-06 — daily PM rotation (Scrum Master + pm-security; routine refresh). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-data next), coverage_cursor idx 12 → 0 (epic-8a re-checked → wraps to epic-10a next)._

## Executive summary

- **Steady systems-hygiene window: 12 PRs merged.** No single breaking incident this cycle. The cluster is anchored by three self-heal / hardening wins:
  1. **`quick-xml` XXE / billion-laughs pin is now a HARD cargo-deny ban** (#2096), with CODEOWNERS gate on `backend/deny.toml` landing right after (#2111) — the pin is no longer bypassable by a lockfile edit.
  2. **Dispatcher `#1014` MCP-push corruption class fully closed** (#2101 triage doc → #2114 wire-in of size-guard + `action-list-reconcile.sh` into Phase 6). Ends a recurring self-inflicted corruption vector for `.research/management/action-list.json`.
  3. **Accounting export honesty invariant is now enforced-by-construction** (`SlovakAccountingExport::compute_partial` refactor #2099); the 15-arg positional `::new` builder follow-up (draft #2117) will close the same-type-transposition risk.
- **Post-merge review opened 5 follow-ups (#2103, #2107, #2108, #2109, #2110)** — 4 already have draft PRs in flight (#2117 → #2103; #2116 → #2109; #2115 → #2108; and #2089 covers #2076 lease.rs drift). `#2110` (exec-bit restore on `check-ignore-reason.sh`) still needs a PR.
- **Security lens (today's rotation) headline:** PR **#1797** ("auth on OCR endpoints + manager-gate rental guest PII reads") has been stale as a draft for **~13 days** on a critical surface (closes #1772 unauthenticated OCR + #1766 guest PII exposure). No security-sensitive work is more urgent this sprint.
- **Second security concern:** issue **#2107** — the outages happy-path tests bypass real login/authz via a fabricated JWT with a role that mismatches the DB role. Masks real breakage in test signal.
- **Coverage/sprint-status drift:** sprint-status.yaml has promoted 7a-3/7a-4/8a-3 to done since the 2026-07-02 deep scan, but `coverage.json` (`scan_kind=upkeep`) still classifies 8a-3 as `partial` (mobile OS push deferred). Reconcile in the next deep scan.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=1/4 (8A only; sprint-status is optimistic — coverage classifier still holds 8a-3 partial pending mobile OS push).

| Epic | Tracked status | Real status (from coverage + activity) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 3/6 stories complete (6-1/6-3/6-6); web UI for 6-2/6-4/6-5 still in flight |
| 7A — Basic Document Management | in-progress | 2/5 stories complete (7a-1 done 2026-06-17; 7a-4 done 2026-07-04); 7a-2 CI-red, 7a-3/7a-5 sprint-status lagging |
| 8A — Basic Notification Preferences | done (sprint-status) | 8a-1/8a-2 done; 8a-3 promoted in sprint-status but coverage still holds partial (FCM/APNs deferred) |
| 10A — OAuth Provider Foundation | in-progress | 0/3 stories complete; #481 (revoked-token bypass) + #487 (MFA rate-limit) test-hardening gates still open |

## Shipped since last run (12 PRs, cursor 2026-07-05T03:10:00Z)

- **#2094** — Reuse shared `seed_org` fixture in favorite_alert_read idempotency test [pm-qa]
- **#2095** — Extract + unit-cover `assign_fault_recipients` guards [pm-backend]
- **#2096** — Enforce quick-xml XXE pin via cargo-deny HARD ban [security/devops]
- **#2097** — Un-quarantine #1771 soft-delete unread invariant (behavioural coverage back on) [pm-qa]
- **#2098** — Enum sync guard for `SupportedCurrency` + `CountryCode` across 3-source triangle [pm-backend]
- **#2099** — Enforce `SlovakAccountingExport` honesty invariant by construction [pm-backend/accounting]
- **#2100** — Churn-hotspot split of `rental.rs` (2933-line monolith → cohesive sub-modules) [pm-backend]
- **#2101** — Triage doc for `#1014` MCP-push corruption class [pm-tech-lead]
- **#2111** — Gate `backend/deny.toml` with CODEOWNERS [pm-devops]
- **#2112** — Refresh stale BIT-351 quarantine docstrings after #2097 [pm-backend]
- **#2113** — Enum-sync guard now fails on variant missing from canonical list [pm-backend]
- **#2114** — Wire MCP-push size-guard + action-list reconcile into dispatcher Phase 6 (root-cause fix for #1014) [pm-devops]

## What's next (top 5 actions)

1. **[high] Land PR #1797 out of draft (OCR auth + rental guest PII manager-gate)** — pm-security. Closes #1772/#1766; 13-day staleness on a security-critical surface is itself the blocker.
2. **[high] Fix issue #2107 fabricated-JWT test bypass** — pm-security / rust-backend. Outages happy-path tests must exercise the real login/authz flow; DB-role and JWT-role must be verified in parity.
3. **[high] Complete 79-2 auth-flow SSO/JWT/cookie sign-off** — pm-security. This is the top pending pm-security action per coverage.json since the 2026-07-02 deep scan.
4. **[medium] Land the 4 in-flight follow-up drafts (#2115, #2116, #2117, #2089)** — pm-backend / pm-qa. Closes 4 of 5 post-merge review issues in one sweep; #2117 replaces the 15-arg positional `::new` builder that #2099 left as a decorative smell.
5. **[medium] Add OAuth Provider (10a) PKCE + refresh-rotation + introspection tests** — pm-security / rust-backend. Closes `pm-security-oauth-10a-untested-security-contract` before any epic-10a promotion.

## Blockers

- **PR #1797** (OCR auth + rental guest PII gate) — 13-day stale draft, security-sensitive. Owner: pm-security.
- **7a-2-folder-organization** — reverted from done to review; `document_folder_tests` CI red (PR #1316 round 1). Owner: pm-backend.
- **80-2-dispute-filing-flow** — held at partial pending the 5-step wizard redesign product decision. Owner: pm-frontend.
- **test_hardening_batch (thb-2026-05-25)** — issues #480/#481/#482/#484/#487 still open, gating 8a-3, 10a-1/10a-2/10a-3, and 7a-5 promotions. Owner: pm-backend / pm-frontend.

## Role focus today: **pm-security** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, last 2026-05-27 → 40d stale before this run): 6 new next_actions appended to `action-list.json`; 5 new risks appended to `risks.json`; 2 new decisions in `decisions.md` (queued). Full role JSON in `.research/management/roles/pm-security.md`. Headline: PR #1797 stale draft on security-critical surface; #2107 fabricated-JWT masks real drift; 79-2 auth-flow sign-off still owed; OAuth 10a untested-security-contract risk still open. Wins: quick-xml XXE hard-ban + CODEOWNERS gate; accounting export honesty invariant enforced-by-construction.
- **pm-scrum-master** (always-on): produced the delivery synthesis above; headline = steady hygiene sweep (12 merged, 4 follow-ups already in-flight); coverage vs sprint-status drift on epic-8a needs a deep-scan reconcile.

## Coverage (upkeep — 2026-07-06)

- `coverage.json` refreshed with `scan_kind="upkeep"`. Rotating epic-8a re-checked at coverage cursor idx 12; no story-level status changes this window (no merged PR touched epic-8a). `last_checked` bumped to 2026-07-06 on all 3 epic-8a stories.
- **Systemic epic-8a divergence:** sprint-status.yaml marks epic-8a done, but coverage classifier still holds 8a-3 partial (mobile FCM/APNs stub swallows failures + pipeline dispatch serial). Deep scan will reconcile.
- **Deep scan of 2026-07-02 baseline:** 49 stories · 40 done · 9 partial · 0 not-started. Dominant remaining vectors: (a) promotion lag on epic-7A documents; (b) mobile OS push (8a-3); (c) mobile-native iOS Account + Android/KMP inquiry UI (82-5); (d) missing `POST /reports/schedules` (81-1).
