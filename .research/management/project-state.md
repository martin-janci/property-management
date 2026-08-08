# PPT Project State

_Generated: 2026-08-08 — routine Phase 1.6 lightweight upkeep (pm-qa rotation slot, 54 days overdue). Coverage `scan_kind=upkeep`; pm_cursor idx 3 → 4 (pm-qa → pm-devops next), coverage_cursor idx 5 → 6 (epic-80 re-checked, PR #2712 evidence added to 80-1; advances to epic-81). 7 PRs merged since #2702; 4 open PRs touched; #2704 auto-closed by #2707._

## Executive summary

- **Delivery still at 47/49 stories done, 2 partial** (the 84-1 direct-to-S3 upload wiring and 84-2 sign page). Coverage-view epics done: **12/13** (only epic-84 has partials). No status flips this window.
- **The auto-review loop shipped 7 PRs since 2026-08-07 and closed one of its own tickets inside 24h** (#2704 memory-DoS opened 06:30, PR #2707 fix merged 16:18 same day). #2703 (SSRF DNS-rebinding TOCTOU) is still open with draft PR #2710 in flight.
- **Two hotfix-no-test slips this window**: PR #2707 (memory cap, closes #2704) and PR #2712 (dispute add_evidence audit event) both shipped without a named regression test — this is exactly the recurring pattern pm-backend flagged on 2026-07-30 and pm-qa is now proposing as a merge-gate this run.
- **PR #2696 is a live sequencing hazard**: the inquiry-email notifier seam is ready-to-merge, but the follow-up (`code-review-reality-server-inquiry-notify-route-wiring`) noted that the live public endpoint bypasses the seam. Merging #2696 alone ships a "success message, no notification" regression. Sequence-lock queued as pm-qa's #1 action this run.
- **Sandbox verification gap**: both this window's api-server refactors (#2711 layout/tenant.rs, #2713 layout/admin.rs) marked cargo test / clippy DEFERRED-TO-CI because the utoipa-swagger-ui build script needs github.com egress. The biggest crate now has no local pre-flight — pm-qa proposes vendoring the swagger-ui zip.
- **sprint-status.yaml drift**: epics 6/7a/10b/80 still listed as in-progress despite coverage.json showing all their stories done. A housekeeping reconcile is queued.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · yaml `epics_done = 2/5` (8a, 10a); yaml claims 3 more in-progress (6, 7a, 10b) but coverage view has all their stories done. Extended-scope coverage: **12/13 epics done, 1 partial (epic-84)**.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial (yaml stale) | **3/3 stories done in coverage (re-checked 2026-08-08, PR #2712 add_evidence audit added to 80-1)** |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1 direct-S3 wiring, 84-2 sign page) |
| 82 / 83 / 85 / 79 / 81 / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (7 PRs > #2702)

- **#2713** — refactor(api-server): dedupe layout admin handler boilerplate (churn-hotspot mirror of #2711)
- **#2712** — data(dispute): emit add_evidence access-audit event on audit_logs (AuditRead-gated)
- **#2711** — refactor(api-server): dedupe layout tenant-override handlers (churn-hotspot)
- **#2709** — i18n(reality-web): ListingForm via next-intl catalogs (6 locales) — closes `code-review-reality-web-listingform-no-i18n`
- **#2708** — fix(workflow_executor): compare_numeric rejects non-finite numeric strings (NaN/Inf uncomparable, not < 100)
- **#2707** — fix(workflow api_call): cap unbounded response-body read at 8 MiB, truncate-with-marker — closes #2704
- **#2706** — fix(ai/llm): fail-closed on partial RAG embedding batch (502 EMBEDDING_FAILED on count mismatch)

## What's next (top 5 actions from ranked backlog)

1. **[high] Merge PR #2710** — closes #2703 SSRF DNS-rebinding TOCTOU (live vuln, draft PR open >24h). Requires resolver-spoof regression test before merge. **owner: pm-tech-lead**.
2. **[high] Sequence-lock #2696 (inquiry-email seam) with `code-review-reality-server-inquiry-notify-route-wiring`** — merge as a pair; solo #2696 ships silent-success. **owner: pm-backend / pm-qa**.
3. **[high] Backfill regression tests** for #2707 (body-cap) and #2712 (add_evidence audit) — recurring hotfix-no-test pattern. **owner: pm-qa**.
4. **[high] Ship 84-1** direct-to-S3 wiring in ppt-web (POST /documents/upload-url consumer). **owner: pm-frontend** (blocked-on #2573).
5. **[high] Ship 84-2** signer-facing document-sign page in ppt-web. **owner: pm-frontend**.

## Blockers

- **PR #2696 in isolation** — would merge as functional dead code; live send_contact_message bypasses the new notifier seam. Sequence-lock with route-wiring follow-up before merge. Owner: pm-backend.
- **Story 84-1 (ppt-web direct-to-S3 wiring)** — blocked-on #2573 same-org reference-check gap (carried from 2026-07-30). Owner: pm-backend.
- **SSRF #2703 (workflow api_call.rs DNS-rebinding TOCTOU)** — live vulnerability with no merged fix yet (draft #2710). Owner: pm-tech-lead.

## Role focus today: **pm-qa** (rotation idx 3 — last run 2026-06-15, 54 days stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): 7 PRs merged, 4 open PRs touched, 1 issue auto-closed inside 24h. The auto-review loop is genuinely closing findings. New sequencing hazard emerged (#2696 seam-without-wire) and two hotfix-no-test slips need pm-qa backfill. Sprint-yaml drift against coverage is queued for a housekeeping PR.
- **pm-qa** (rotation): top 3 findings — (1) hotfix-no-test recurring on #2707 and #2712; (2) #2696 ready-to-merge is a silent-success trap; (3) sandbox verification gap on api-server (utoipa-swagger-ui zip needs vendoring). Six pm-qa next_actions queued into action-list; five pm-qa risks added to risks.json.

## Coverage (upkeep this run — 2026-08-08)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped.
- **Epic re-check: epic-80** — cursor idx 5. All 3 stories still `done`; evidence entry added to 80-1 for PR #2712 (dispute add_evidence audit event); re-check notes added to 80-2 / 80-3. `last_checked = 2026-08-08` stamped on all 3 stories.
- **Merged-PR evidence added:** none of the 7 merged PRs mapped to coverage stories other than 80-1's audit trail (PR #2712). #2707/#2708/#2706 are code-review hardening; #2709 is reality-web i18n; #2711/#2713 are churn-hotspot refactors.
- **`coverage_cursor` advances 5 → 6** (epic-80 → epic-81 next run).
- **`pm_cursor` advances 3 → 4** (pm-qa → pm-devops next run). role_last_run["pm-qa"] = 2026-08-08.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. Same 3 missing UC links (UC-33.x — 2 queued into action-list, 1 remaining). Zero orphan screens, zero validation errors.

## Skipped this run

- **Full 36-item action-list buffer refill.** Only the pm-qa items (6 new) plus this run's PR-status updates were merged into `action-list.json`. Fresh gap-ranked candidates were not re-derived because the task spec limits this run to lightweight upkeep (the deep re-ranking / re-scan is a LOCAL-only `scan` mode operation). Current buffer: **~14 open**. Below-half warning fires; recommend a human-triggered `/ppt-project-management scan` run.
- **Broad `gh` PR-status checks** for carried follow-ups (#2555 accounting trio, #2573 ref-check status, #2528 booking webhook) — kept as open questions in the Scrum Master synthesis rather than tool-verified.
