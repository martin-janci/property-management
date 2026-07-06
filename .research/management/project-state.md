# PPT Project State

_Generated: 2026-07-06 — daily PM rotation (Scrum Master + pm-security; routine refresh). Coverage `scan_kind` unchanged (last deep scan 2026-07-02); pm_cursor idx 5 → 6 (pm-data next), coverage_cursor idx 12 → 13._

## Executive summary

- **Clean-merge day: 18 PRs onto `dev`, zero CI casualties.** The 2026-06-16 `dev`-red incident (#1437) appears fully behind us — 18 back-to-back merges through the queue with no compile-red / no revert. Highlights: `#2120` (authorize outage mutations on DB role not JWT claim — the anti-pattern PR), `#2100` (rental repository split, 5866 LOC move), `#2118` (mobile RN Meters/Leases/Forms/Threads wired to api-server), `#2096` + `#2111` (quick-xml XXE cargo-deny hard ban, deny.toml gated on codeowners), `#2099`/`#2117` (SlovakAccountingExport enforce-by-construction).
- **pm-security headline (rotation focus today): the #2120 anti-pattern is still live at 34 call sites across 12 route files** — all in this sprint's active epics (6, 7A, 8A). Same JWT-role-vs-DB-role gate that silently 403's real managers on core mutations while integration tests (raw-token mint) pass. Not a privilege-escalation hole (fails closed to `Guest`), but a likely functional regression on Epic 6 announcement create/update, Epic 7A document folder/share ops, and Epic 8A granular-notification writes.
- **15 follow-up / from-merged-review issues closed** by the matching PR merges — the backlog burned down cleanly this window.
- **Stale draft cluster still stuck:** `#1797` (OCR auth + rental PII gate, 13 days) — pm-security notes its fix content is *already present* in the working tree, so it may need close/rebase not merge; `#1812` (`reality_portal.rs` split, `needs-human-review`); `#2089` (draft, DB-round-trip regression test for `lease.rs`). Dependabot `#2016` (rust-toolchain), `#2018` (aes-gcm — pm-security wants manual crypto-surface review before merge).
- **Sprint-status drift note:** `7a-3`/`7a-4` were flipped to `done` on 2026-07-04 (post-coverage-snapshot), `7a-5` still shows `ready-for-dev` despite shipped code + merged PR `#2003`; needs a reconciliation pass.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=1/4 (8A only in the named sprint) — same delta as last run in raw count, but the pm-security 34-site finding materially changes Epic 6/7A "done-ability" until swept.

| Epic | Tracked status | Real status (from coverage + this-run activity) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories `done` per coverage, but 4 `announcements/*.rs` files carry the JWT-role gate — sweep required before release |
| 7A — Basic Document Management | in-progress | 7a-1 done, 7a-3/7a-4 flipped done 2026-07-04; 7a-2 CI-red, 7a-5 stale-status (code shipped via #2003), 4 `documents/*.rs` files carry the JWT-role gate |
| 8A — Basic Notification Preferences | near-done | 8a-1/8a-2 done; 8a-3 WS half live; mobile-push (FCM/APNs) leg still open; `granular_notifications.rs` carries the JWT-role gate |
| 10A — OAuth Provider Foundation | in-progress | 10a-1/10a-2/10a-3 all `done` per coverage; sprint-status still `ready-for-dev`; #481/#487 security-test gate still open |

## Shipped since last run (cursor #2093, 18 PRs)

- `#2098`/`#2113` — currency/country enum-sync guard (data-integrity)
- `#2099`/`#2117` — SlovakAccountingExport enforce-by-construction + named-field input
- `#2097` — un-quarantine `#1771` unread-invariant test (test-hardening)
- `#2096` — quick-xml XXE cargo-deny hard ban
- `#2111` — gate `deny.toml` with codeowners
- `#2120` — **authorize outage mutations on DB role not JWT claim** (Phase-1.6 anti-pattern seed)
- `#2100` — rental repository split (5866 LOC move)
- `#2118` — mobile RN Meters/Leases/Forms/Threads wired to api-server
- `#2115` — ListingDetail auth reset regression fix
- 15 follow-up / from-merged-review issues closed via matching PR merges
- `7a-3`/`7a-4` promoted to done (2026-07-04) since the 2026-07-02 coverage snapshot

## What's next (top 5 actions)

1. **[high] Sweep 34 `TenantExtractor::role.is_manager()` call sites** across announcements (6 files) + documents (4 files) + `granular_notifications.rs` + `templates.rs`, replace with `RlsConnection::role()` / `ValidatedTenantExtractor`, and land a real-login-flow regression test — owner: **pm-backend** (pm-security-owned finding). This is the release-gating item for Epic 6/7A.
2. **[high] Confirm PR #1797 merge/close status** (OCR auth + rental PII gate) — fix content is already in the tree; may need to be closed as superseded to avoid duplicate rework. Owner: pm-scrum-master.
3. **[high] Fix CI-red `document_folder_tests` (FK/isolation) blocking 7a-2 promotion** — owner: pm-backend.
4. **[high] Close security follow-ups #480/#481/#487** gating 8a-3 mobile-push and epic-10a promotion — owner: pm-backend / pm-qa.
5. **[medium] Reconcile sprint-status.yaml for 7a-5** (code shipped via #2003, still `ready-for-dev`) and resolve `#485` (window.confirm + missing UUID validation on share panel) — owner: pm-frontend.

## Blockers

- **34-site JWT-role vs DB-role anti-pattern.** Owner: pm-backend (pm-security). Silently 403's real managers on Epic 6/7A/8A mutations in production; tests hide it via raw-token mint.
- **7a-2-folder-organization CI red.** Owner: pm-backend. `document_folder_tests` FK/isolation failing since PR #1316 round 1; reverted from done pending green.
- **Epic 10A promotion gated on #481 (revoked-token reuse) and #487 (MFA rate-limit).** Owner: pm-backend / pm-qa.
- **Stale drafts #1797 (13d) and #1812 (needs-human-review).** Owner: pm-tech-lead / pm-scrum-master (go/no-go this week before rebases become costly).

## Role focus today: **pm-security** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, last 2026-05-27, 40d stale — the longest gap in the rotation): 6 new next_actions, 5 new risks, 3 new decisions appended. Full role JSON in `.research/management/roles/pm-security.md`. Headline: `TenantExtractor::role` anti-pattern still live at 34 sites across 12 files; #2120 is one fix among many needed.
- **pm-scrum-master** (always-on): produced the delivery synthesis above; headline = clean 18-PR merge day, but Epic 6/7A "doneness" is now conditional on the 34-site sweep.

## Coverage (last deep scan 2026-07-02)

- `coverage.json` unchanged since 2026-07-02 deep scan — 49 stories: 40 done · 9 partial · 0 not-started. `7a-3`/`7a-4` shown as `partial` in the snapshot but were flipped `done` 2026-07-04 in sprint-status; a follow-up upkeep pass should reconcile these two.
- Coverage cursor idx advanced 12 → 13 (next rotating epic-check).
- No coverage story flip induced by this run's merges: #2120/#2118/#2100/#2115 are fixes/refactors, not story completions.
