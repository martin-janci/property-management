# PPT Project State

_Generated: 2026-07-06 — daily PM rotation (Scrum Master always-on + pm-security rotation slot). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-data next), coverage_cursor idx 12 → 0 (epic-9 refreshed → epic-10a next)._

## Executive summary

- **Zero sprint-story velocity this window.** All 21 merged PRs (2026-07-05→06) were research/infra/security/refactor track — none map to an active sprint epic story. Highest-signal merges: **#2120** (outages DB-role authz fix), **#2118** (mobile RN Meters/Leases/Forms/Threads wired to api-server), **#2100** (rental repository split), **#1979** (outages happy-path auth test hardening), **#2097** (messaging soft-delete unread invariant un-quarantined).
- **Sprint spine is materially stale vs. `coverage.json` (2026-07-02 deep scan).** `sprint-status.yaml` shows epic-10a at 0/3 with all three `ready-for-dev` — coverage shows all three `done` with merged-PR evidence. epic-10b similarly shows in-progress but coverage shows 7/7. 7a-3 / 7a-4 code-complete but unflipped. Reconciliation is the top scrum-master action this cycle.
- **CRITICAL security finding (pm-security rotation, idx 5, 40 days stale):** PR #2120 fixed the JWT-role-vs-DB-role authz bug in outages.rs only. The same root cause — `TenantExtractor.role` always resolves to Guest because the production login flow never populates `AuthUser.role` — still gates manager-only mutations/overrides in **~9 other handler files** across `documents/{core,folders,shares,versions}.rs`, `announcements/{comments,engagement,lifecycle,crud,ai_draft,stats}.rs`, `templates.rs`, `granular_notifications.rs`. Epic 6 / 7A stories marked `done` this sprint likely fail closed for real managers in production.
- **Two stalled security-relevant drafts:** **#1797** (auth on OCR endpoints + manager-gate rental guest PII, 13d stale, live PII exposure) and **#1812** (reality_portal repository split, 12d stale, `needs-human-review`).
- **Supply-chain governance advance:** PR #2111 wires CODEOWNERS review on `backend/deny.toml`; PR #2096 hard-bans quick-xml XXE via cargo-deny. Both effective only if dev branch protection enforces "Require Code Owners review" (unverifiable in-repo, tracked as risk).
- **Zero untriaged issues.** All 19 recently updated issues are closed follow-ups (from-merged-review label).

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=1/6 (spine value; **coverage.json shows materially more done** but spine not reconciled).

| Epic | Spine status | Real status (from coverage + activity) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | Multiple stories done in coverage; JWT-role bug class puts manager-only mutations at risk of production failure |
| 7A — Basic Document Management | in-progress | 2/5 spine-done; 7a-3/7a-4 code-complete in coverage but unflipped; 7a-2 blocked on CI red PR #1316; 7a-5 web share-panel parity unconfirmed |
| 8A — Basic Notification Preferences | done | 3/3 spine-done — but 8a-3 partial in coverage: mobile OS push (FCM/APNs) deferred + push_fanout BLPOP drain unimplemented (#484) |
| 10A — OAuth Provider Foundation | in-progress (0/3) | Coverage: 3/3 done (PRs #1197/#1168/#661/#1025); test-hardening gates #481 (RFC 9700 revocation) + #487 (MFA rate-limit) block spine flip |
| 10B — Platform Administration | in-progress | Coverage: 7/7 done (10b-5 via #793/#854; 10b-7 via backend+admin-web+mobile+#844); spine stale |
| 80 — Dispute Resolution | partial | 1/3; 80-2 blocked on 5-step wizard redesign + AC-4 draft auto-save |

## Shipped since last run (window 2026-07-05T03:10Z → 2026-07-06, 21 PRs)

- **#2120** — security-fix: authorize outage mutations on DB role, not JWT claim [pm-security]
- **#2118** — mobile RN Meters/Leases/Forms/Threads wired to api-server (clears backlog `code-review-mobile-rn-screens-mock-data`) [pm-frontend]
- **#2115** — fix: ListingDetail auth change resets screen to spinner (reality-mobile) [pm-frontend]
- **#2119** — restore executable bit on check-ignore-reason.sh [chore]
- **#2117** — named-field input for SlovakAccountingExport::new [pm-backend]
- **#2116** — cover include_system=true tie-break in template pagination test [pm-qa]
- **#2114** — wire action-list reconcile + MCP-push size guard into dispatcher Phase 6 [research]
- **#2113** — drive enum-sync guard off enum variant set [pm-backend]
- **#2112** — refresh stale BIT-351 quarantine docstrings [docs]
- **#2111** — gate `backend/deny.toml` with code-owner review [pm-security]
- **#2101** — triage MCP-push large-file issue [research]
- **#2100** — refactor: split rental repository into cohesive sub-modules [pm-backend]
- **#2099** — SlovakAccountingExport honesty invariant by construction [pm-backend]
- **#2098** — enum sync guard for currency/country lists [pm-qa]
- **#2097** — un-quarantine #1771 messaging soft-delete unread invariant [pm-qa]
- **#2096** — enforce quick-xml XXE pin via cargo-deny hard ban [pm-security]
- **#2095** — extract assign_fault recipient guards [pm-backend]
- **#2094** — reuse shared seed_org fixture in favorite_alert_read [pm-qa]
- **#1979** — fix outages happy-path auth (BIT-414) [pm-qa]
- **#2020**, **#2014** — dependabot bumps (rust_xlsxwriter, axum-test)

## What's next (top 5 actions)

1. **[high]** Migrate all `tenant.role.is_manager()` mutation/override gates to DB-validated role (mirror PR #2120) across documents/announcements/templates/granular_notifications — owner: pm-security. Same JWT-role bug class as #2120; already-shipped stories likely fail closed for real managers in production.
2. **[high]** Escalate stalled draft PR **#1797** (OCR endpoints auth + rental-guest PII) — owner: pm-security. 13d stale live PII exposure.
3. **[high]** Reconcile `sprint-status.yaml` against `coverage.json` (flip epic-10a, epic-10b, 7a-3, 7a-4 to done with evidence) — owner: pm-scrum-master. Unblocks the misreport and re-evaluates dependent test-hardening gates.
4. **[high]** Reconcile permission-based document access (`7a-3`) + document sharing (`7a-5`) — owner: pm-backend. Authz-sensitive; both code-complete but sprint-status stale and frontend parity unconfirmed (#485).
5. **[high]** Add OAuth 10a security test suite (introspection / refresh-rotation / PKCE) — owner: pm-security. Revoked-token bypass (#481) keeps 10a-1/10a-3 flagged.

## Blockers

- **JWT-role-vs-DB-role mutation-gate class widespread (post-#2120).** Owner: pm-security + pm-backend. ~9 handler files across documents/announcements/templates/granular_notifications.
- **PR #1797 stalled 13d.** OCR auth + rental-guest PII manager-gate. Owner: pm-security.
- **PR #1812 stalled 12d.** reality_portal repository split, `needs-human-review`. Owner: pm-tech-lead.
- **7a-2 CI red on `document_folder_tests` (PR #1316).** Owner: pm-backend.
- **Sprint-status.yaml drift vs. coverage.** Owner: pm-scrum-master.

## Role focus today: **pm-security** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, last 2026-05-27 → 40 days stale): 6 new next_actions appended to `action-list.json`; 4 new risks appended to `risks.json`; 2 new decisions in the role write-up. Full role JSON in `.research/management/roles/pm-security.md`. **Headline:** JWT-role-vs-DB-role authz bug class is systemic — #2120 was the tip; documents/announcements/templates/granular_notifications still ship the same anti-pattern in production. Plus #1797 PII draft has now sat 13d.
- **pm-scrum-master** (always-on): produced delivery synthesis above; 6 next_actions + 5 risks + 3 blockers-of-record captured. Full write-up in `.research/management/roles/pm-scrum-master.md`. **Headline:** zero sprint-story velocity this window + spine materially stale vs. coverage.

## Coverage (upkeep — 2026-07-06)

- **Cursor rotation:** coverage_cursor idx 12 → 0. Epic-9 refreshed (`9-1-totp-2fa-setup` last_checked stamped; status remains `done`). Next upkeep target: `epic-10a` (idx 0).
- **No PR-driven status flips this window.** Merged PRs did not map to a coverage story state change (mobile RN wiring in #2118 clears a backlog signal, not a story).
- **Dominant systemic drift stays:** 0/120 screen-maps populate `frontmatter.epics:` → epic→screen linkage impossible; manufactures the 29-screen orphan cluster. Backfilling `epics:` remains the single highest-leverage fix.
