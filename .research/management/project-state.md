# PPT Project State

_Generated: 2026-08-31 — routine Phase 1.6 lightweight upkeep (pm-devops rotation slot; 76-day stale slot refreshed) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; pm_cursor idx 4 → 5 (pm-devops → pm-security next), coverage_cursor idx 6 → 7 (epic-81 re-checked, no material change; advances to epic-82). Sprint window 2026-08-26..08-31 shipped 10 PRs — all code-review batch (#2889-#2899, dispatcher merged in-window)._

## Executive summary

- **Delivery still at 47/49 stories done, 2 partial** (84-1 direct-to-S3 upload wiring and 84-2 sign page). No status flips this window. The last window's 10 PRs were exclusively `code-review-*` fixes surfaced by the dev-team static passes:
  - Backend security/reliability — PR #2893 (SSO introspect negative-cache poisoning fix), PR #2898 (validate reporter_email/reporter_phone on report submit), PR #2899 (compliance guard widening for PlatformAdmin).
  - Backend correctness — PR #2892 (measure text length by char count not bytes; 8 files), PR #2896 (localize saved-search alert copy by recipient locale).
  - Frontend correctness — PR #2889 (wire realtime ws events to query invalidation — end-to-end dead sync fix), PR #2890 (surface swallowed mutation errors), PR #2891 (wire onUnauthorized so 401 triggers session recovery).
  - Frontend i18n — PR #2894 (Saved Searches localization), PR #2895 (Inquiries page localization).
- **Auto-review loop still working:** all 10 merged PRs were originated by the ppt-dev-review generator over the prior 2 days (dispatcher spawned static passes → landed as `code-review-*` PRs → all merged in the same window). Zero regressions detected on the batch.
- **Buffer starved (cloud-only):** 7/8 open backlog items are mobile-native/KMP — structurally unclaimable in the cloud dispatcher (AGP/Gradle egress gate, issue #2652). Every dispatcher commit since 2026-08-30 records `GC3-buffer-bounds=FAIL (record-only)`. Tier-1d generator kicks per run just to keep the buffer above floor. **Infra decision surfaced this run (pm-devops).**
- **One open PR touched this run:** #2897 (`code-review-reality-server-sso-per-call-client-no-timeout`) — approved but had a real merge conflict against #2893 (both touched `introspect_pm_token`); dispatcher Phase 5.6 threaded the pooled `reqwest::Client` through the extracted `introspect_pm_token_inner`, pushed the merge (bca285d..aa5c544). CI red on the resolution attempt (fmt + introspect test-shard). Standing manual-reconciliation request on the PR.
- **No new blockers.** Standing gh-issue-2797 (cargo-deny RUSTSEC-2026-0258 h2 DoS) still holds; owner pm-security.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run. Extended-scope epics (10B, 80, 81, 82, 83, 84, 85, 79, 8A, 9) folded into `coverage.json` and largely done.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage |
| 81 — Reports | (extended) | 2/2 stories done in coverage; **re-checked this run (idx 6), no material change, last_checked=2026-08-31** |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1, 84-2) — unchanged |
| 82 / 83 / 85 / 79 / 7a / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (10 PRs merged 2026-08-30 within the run window)

- **#2889** — code-review ppt-web-core ws-event-name-mismatch: wire realtime WS events to query invalidation (100% dead sync fix, real event names)
- **#2890** — code-review reality-web mutation-no-onerror: surface swallowed mutation errors (favorites/saved-searches/inquiries)
- **#2891** — code-review ppt-web-core api-onunauthorized-unwired: wire onUnauthorized so 401 triggers session recovery
- **#2892** — code-review reality-server bytelen-charcount-validation: measure text length by character count not bytes (8 files, accented Slovak/Czech/German)
- **#2893** — code-review reality-server sso-introspect-negcache-poison: don't cache active=false on PM introspection outage
- **#2894** — code-review reality-web saved-searches-i18n: localize Saved Searches page + card (6 locales)
- **#2895** — code-review reality-web inquiries-i18n: i18n the Inquiries page (6 locales)
- **#2896** — code-review reality-server alert-drainer-i18n-english-only: localize saved-search alert copy by recipient locale (sk/cs/de/en with plural buckets)
- **#2898** — code-review reality-server report-contact-unvalidated: validate reporter_email/reporter_phone on report submit
- **#2899** — code-review api-handlers compliance-superadmin-exact-match: let PlatformAdmin reach compliance reports

## What's next (top 5 actions from ranked backlog)

1. **[high] Unblock mobile-native/KMP builds in the cloud runner (issue #2652)** — 7/8 open backlog items structurally unclaimable — **owner: pm-devops**. New this run — cloud-runner starvation is now chronic; pushing it into the top actions.
2. **[high] Wire ppt-web direct-to-S3 upload (84-1 partial)** — POST /api/v1/documents/upload-url consumer + regression test — **owner: pm-frontend**. Highest-leverage single move (drops partial count 2 → 1).
3. **[high] Build signer-facing document-sign page (84-2 partial)** — flip screen-map ppt/document-sign buildStatus planned → shipped — **owner: pm-frontend**. Closes 49/49 MVP when paired with #2.
4. **[high] Resolve gh-issue-2797** — cargo-deny RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) blocks every backend PR — **owner: pm-security**. Standing since 2026-08-18.
5. **[medium] Resolve #2897 CI red** — PR approved and merge-conflict-resolved (bca285d..aa5c544), but CI shard is red (fmt + introspect test); needs manual reconciliation in a buildable env — **owner: pm-backend**.

## Blockers

- **None new this run.** Sprint continues with no red flags; all 10 code-review PRs landed in-window.
- **Standing:** gh-issue-2797 (cargo-deny RUSTSEC-2026-0258 h2 DoS) blocks every backend PR until landed. Owner: pm-security.
- **Standing infra:** issue #2652 — mobile-native/KMP builds unlandable in cloud runner. Owner: pm-devops (surfaced as this run's top action).
- **Aging:** 84-1 + 84-2 partial stories unchanged for 4 upkeep windows. Owner: pm-frontend.

## Role focus today: **pm-devops** (rotation idx 4; last 2026-06-16, 76d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): the code-review auto-generator + dispatcher merge loop delivered 10 PRs in one window with no regressions detected — the loop is a working delivery mechanism. Delivery blocker is now infrastructural, not code: mobile-native/KMP items can't be picked up in cloud, and the backlog rank is exhausted of landable items (`Buffer starved (claimable=0/floor=36)` on the 2026-08-31T00:35Z commit).
- **pm-devops** (rotation): flagged the mobile-native/KMP cloud-runner gap (issue #2652) as the top standing infra risk. Also proposed a nightly `verify-all.sh --quick` sweep on `dev` HEAD to surface silent infra regressions, and a dependabot-queue triage (7 open dep PRs, 3d idle). See `roles/pm-devops.md` for the full 4 next-actions and 2 risks.

## Coverage (upkeep this run — 2026-08-31)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped to 2026-08-31T03:10:00Z, no re-scan.
- **Epic re-check: epic-81** — cursor idx 6. Both stories (81-1 report-schedule-editing, 81-2 report-execution-history) still `done`. No PR in the 2026-08-26..08-31 window touched reports schedule routes/screens; evidence entry appended to 81-1 noting the negative check. `last_checked = 2026-08-31` stamped on both stories.
- **Merged-PR evidence:** none of the 10 merged PRs (all code-review batch) match a coverage story by keyword — all were correctness hardening of already-shipped surfaces. No status flips.
- **`coverage_cursor` advances 6 → 7** (epic-81 → epic-82 next run).
- **`pm_cursor` advances 4 → 5** (pm-devops → pm-security next run). role_last_run["pm-devops"] = 2026-08-31.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. **Missing UC links: 3** (UC-33.1/33.2/33.3 already queued). Zero orphan screens, zero validation errors.
