# PPT Project State

_Generated: 2026-07-07 — daily PM rotation (Scrum Master + pm-security; routine refresh). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 12 → 0 (epic-9 → epic-10a next)._

## Executive summary

- **Steady week: 18 merged PRs, mostly hardening + refactors.** No production incidents this run; `dev` compile is green (all 18 merges post-date the last dev-red incident and none reintroduced it). Merged mix: 6 refactor / 4 test-coverage / 4 dx / 3 security / 1 bug. Headline shipments: PR #2100 (rental repo split into cohesive sub-modules), PR #2120 (outage authz on DB-role not JWT claim), PR #2096 (quick-xml XXE hard-ban via cargo-deny), PR #2118 (mobile RN screens wired to real API — Meters/Leases/Forms/Threads).
- **New security findings this rotation (pm-security deep review):** TWO high-severity issues surfaced from otherwise-clean PRs.
  1. **Rental repo split preserved the manual-org-filter pattern** (guests.rs / bookings.rs use `&self.pool` + hand-written `WHERE organization_id = $N`, alongside PUBLIC unscoped twins). Production api-server is BYPASSRLS/table-owner, so FORCE RLS on rental_guests/rental_bookings is a paper policy — the `_for_org` filter is the ONLY tenant guard. Guest ID/passport/DOB PII at stake.
  2. **`get_accounting_metrics` still returns positional 6-tuple** with adjacent same-type Decimal fields (revenue↔receivables, expenses↔payables). PR #2117 fixed the SlovakAccountingExport::new boundary but left the tuple source unchanged. Silent field-swap = wrong Slovak tax filing.
- **9 new follow-up issues (#2121-#2129)** all labeled `follow-up, from-merged-review` — all tracked in action-list. pm-security escalated priorities on #2122 (accounting tuple) and #2127 (deny.toml CODEOWNERS advisory-only) to `high`.
- **Positive verifications this run:** PR #2120 outages.rs fully DB-role-gated on all mutation handlers; PR #2096 quick-xml XXE pin bidirectionally enforced (Cargo.toml + deny.toml + CI); PR #2118 mobile RN screens go through the shared `useApiQuery` hook (auth interceptor centralized).
- **No untriaged issues, no stalled-review signals, no risky-churn signals this window.**
- **Backlog upkeep:** action-list pruned from 20 to 15 items (dropped 5 low-value churn-hotspot/refactor items already stabilized post-merge); added 1 new pm-security action.
- **Coverage upkeep:** epic-9 (TOTP MFA) rechecked — story 9-1 remains done (68 totp/mfa hits in routes/mfa.rs, ppt-web TwoFactorAuthPage still present); no MFA-touching PRs this run. `scan_kind` flipped `deep → upkeep`; `last_checked` set to 2026-07-07.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"**. Coverage: **40/49 stories done** (81.6%), 9 partial.

| Epic | Sprint-status | Coverage (done/total) | Notes |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6) | 6/6 done | Sprint-status still stale — coverage says all 6 done; mobile-comments UI is the residual gap on 6-3 |
| 7A — Basic Document Management | in-progress (2/5) | 2/5 done, 3 partial | 7a-2 folder-org CI still red (#1316), 7a-3/7a-4 (backend done, mobile UI residual), 7a-5 sharing sprint-status stale |
| 8A — Basic Notification Preferences | done | 2/3 done, 1 partial | 8a-3 mobile OS push (FCM/APNs) deferred |
| 10A — OAuth Provider Foundation | in-progress (0/3) | 3/3 done | Sprint-status materially stale — coverage says all done |
| 10B — Platform Administration | in-progress (7/7) | 7/7 done | Aligned |
| 80 — Dispute Resolution | partial (1/3) | 2/3 done, 1 partial | 80-2 wizard redesign still in-progress |
| 79 — Frontend Integration | (not tracked) | 4/4 done | Aligned |
| 81 — Reports | (not tracked) | 1/2 done, 1 partial | 81-1 create-schedule route still missing |
| 82 — Mobile (Reality KMP) | (not tracked) | 4/5 done, 1 partial | 82-5 inquiries+account partial (Android/KMP thread UI, Account editing stubs) |
| 83 — Portal Integrations | (not tracked) | 3/3 done | Aligned |
| 84 — Backend Infra | (not tracked) | 5/5 done | Aligned |
| 85 — Mobile Build Pipeline | (not tracked) | 1/2 done, 1 partial | 85-1 env-setup story doc still in-progress |
| 8a / 9 / 10a / 10b | (see above) | 3/3, 1/1, 3/3, 7/7 | All done |

## Shipped since last run (cursor #2093, 18 PRs)

**Security hardening (3):**
- **#2120** — Outage mutations on DB role, not JWT claim (verified full-coverage across create/update/delete/start/resolve/cancel) [pm-backend]
- **#2111** — deny.toml CODEOWNERS gate (advisory-only — see risks/actions) [pm-devops]
- **#2096** — quick-xml XXE pin via cargo-deny hard ban (bidirectionally enforced) [pm-security]

**Test-coverage (4):**
- **#2116** — Cover include_system=true tie-break in template pagination test [pm-tech-lead]
- **#2113** — Enum-sync guard hardening (still #[cfg(test)]-only — see #2124) [pm-backend]
- **#2098** — Enum sync guard for currency/country lists [pm-backend]
- **#2097** — Un-quarantine #1771 soft-delete unread invariant [pm-backend]

**Refactor (6):**
- **#2100** — Rental repository split into cohesive sub-modules (bookings/calendar/connections/guest_documents/guests/ical/oauth/reports/statistics) — see high-severity risk on RLS backstop [pm-backend]
- **#2118** — Wire mobile RN Meters/Leases/Forms/Threads screens to api-server [pm-frontend]
- **#2117** — Named-field input for SlovakAccountingExport::new (accounting tuple mixup left on get_accounting_metrics — see #2122) [pm-backend]
- **#2099** — Enforce SlovakAccountingExport honesty invariant by construction [pm-backend]
- **#2095** — Extract + unit-cover assign_fault recipient guards [pm-backend]
- **#2094** — Reuse shared seed_org fixture in favorite_alert_read idempotency test [pm-qa]

**DX / infra (4):**
- **#2119** — Restore executable bit on check-ignore-reason.sh + recurrence gate [pm-devops]
- **#2114** — Wire MCP-push size guard + action-list reconcile into dispatcher Phase 6 (still a no-op as wired — see #2126) [pm-devops]
- **#2112** — Refresh stale BIT-351 quarantine docstrings [pm-tech-lead]
- **#2101** — Triage doc: MCP-push fallback state-fidelity analysis (issue #1014) [pm-devops]

**Bug (1):**
- **#2115** — ListingDetail auth change resets screen to spinner; test hoisted paths [pm-frontend]

## What's next (top 5 actions)

1. **[high — new] Add AccountingMetrics named-field struct for get_accounting_metrics return type** — pm-backend. Escalated from #2122 by pm-security 2026-07-07. Blocks any Slovak-market accounting-export customer go-live.
2. **[high — new] Remove or gate unscoped rental repo methods (`find_guest_by_id`/`update_guest`/`register_guest`/`delete_guest`/`find_booking_by_id`) OR migrate rental/guests.rs+bookings.rs to RlsConnection pattern** — pm-backend. New from pm-security 2026-07-07 rotation; guest PII tenant isolation is convention-only today.
3. **[high — escalated] Promote deny.toml CODEOWNERS gate to a CI-blocking check + flip GH branch-protection "Require review from Code Owners"** — pm-devops. #2127; escalated by pm-security 2026-07-07.
4. **[medium] Triage still-open PR #1797 (OCR auth + rental PII manager-gate)** — pm-security. Compounds finding #2; rebase against post-split rental/ tree or close.
5. **[medium] Strengthen enum_sync_guard: HashSet set-equality (not just length) + move exhaustiveness match out of `#[cfg(test)]`** — pm-backend. #2124; sharpened by pm-security 2026-07-07.

## Blockers

- **Slovak-market accounting export blocked** on `get_accounting_metrics` tuple refactor (finding #2 above).
- **Guest PII tenant isolation** relies on convention only (finding #1) — a single unscoped-method callsite added to routes/rentals.rs could leak cross-tenant guest data.

## Risks — new/escalated this run

| ID | Sev | Note |
|---|---|---|
| pm-security-rental-repo-rls-backstop-absent | high | NEW. Rental split preserved manual-org-filter pattern; no DB backstop |
| pm-security-accounting-metrics-tuple-transposition | high | NEW. `get_accounting_metrics` still positional 6-tuple |
| pm-security-enum-sync-guard-test-only | medium | NEW. `#[cfg(test)]`-only + only length check |
| pm-security-codeowners-advisory-only | medium | NEW. No workflow enforces "Require review from Code Owners" |
| pm-security-pr-1797-stale-ocr-rental-pii | medium | NEW. Compounds #1; PR open >90d |

Full risk list: `.research/management/risks.json` (47 total).

## Role focus today: **pm-security** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, last 2026-05-27, 41d stale): subagent deep review returned 5 findings + 5 action items. Two HIGH-severity findings (rental RLS backstop, accounting metrics tuple) are now in risks.json + action-list.json. Full details in `.research/scratch/phase16-pm-security.json`.
- **pm-scrum-master** (always-on): synthesis above. Headline = steady hardening/refactor week; coverage steady at 40/49; sprint-status.yaml materially stale for Epic 6/7A/10A vs coverage — write a `pm-scrum-master-sync-sprint-status` action if stale-drift persists next run.

## Coverage (upkeep — 2026-07-07)

- **coverage.json refresh mode:** `scan_kind = upkeep` (2026-07-07T02:15:00Z). Epic-9 (TOTP MFA) evidence rechecked — story 9-1-totp-2fa-setup remains done. Rest of coverage.json untouched this run.
- **Merged-PR → story mapping:** No merged PR in the 18-PR window unambiguously advanced any partial story to done. PR #2115 (ListingDetail auth) touches epic-82 story 82-4 which is already done. PR #2118 (mobile RN screens) touches Epic 6-5 and 7a-2/7a-3 flows but the change is a mobile-UI wiring not a story-scope advance.
- **Next epic upkeep:** epic-10a (OAuth) — coverage_cursor advances 12 → 0 (mod 13).

## Coverage rotation (rolling upkeep order)

epic-10a → epic-10b → epic-6 → epic-79 → epic-7a → epic-80 → epic-81 → epic-82 → epic-83 → epic-84 → epic-85 → epic-8a → epic-9 → epic-10a (loop)
