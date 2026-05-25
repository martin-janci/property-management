# PPT Project State

_Generated: 2026-05-24 — daily PM rotation (Scrum Master + pm-backend). Coverage map last rebuilt by `/ppt-project-management scan` on 2026-05-23._

## Executive summary

- Four PRs merged since the last run (2026-05-23T17:47Z): **#437** deleted the dead AuthHandler/BuildingHandler modules (2495 lines; resolves the duplicate-handler divergence risk), **#436** formally closed Epic 8A (3/3 stories), **#440** completed the story 6.1 review pass, and **#446** shipped gap-7a-4 PDF.js client-side document preview.
- **PR #435** (the P0/P1 security batch) merged 2026-05-23T22:26Z but left deferred findings, now tracked in issues **#438** and **#439**. The most concrete is **P1-05 SSRF** — verified still open at `signatures.rs:628` and a second sink at `integrations.rs:2743`; promoted to plan `security-ssrf-outbound-url-validation`.
- Six PRs (#441–#447) opened 2026-05-24 are in flight (4 draft) — the mobile slice of Epic 7A, login-flow wiring, MFA frontend, document-access API, and the DEC-001 build-order decision. Review queue depth is the near-term delivery risk.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

| Epic | Tracked status | Real status (from last scan) |
|---|---|---|
| 6 — Announcements & Communication | in-progress (1/6 done) | 6 partial — backend done, frontend apiStatus stub |
| 7A — Basic Document Management | in-progress (0/5 done) | 5 partial — 7a-4 preview now shipped (#446); mobile (7a-1/7a-5) + API integration (7a-3) in flight |
| 8A — Basic Notification Preferences | done (3/3) | 2 done, 8a-3 awaits Epic 2B WebSocket infra |
| 10A — OAuth Provider Foundation | in-progress (0/3 done) | 3 done backend; no admin UI / tests |
| 10B — Platform Administration | in-progress (3/7 done) | 3 done, 4 partial (handler stubs 10b-4/5/6/7) |

## What's next (top 5)

1. **[high · pm-security]** Resolve PR #435 post-merge findings (#438/#439): P1-05 SSRF, P1-04 Debug-format audit hash, P0-12 cookie scope, P1-01 ordering, IG3 test gap. _Why:_ auth-layer holes left open after a security PR compound fast.
2. **[high · pm-backend]** Fix the P1-05 SSRF — extract `validate_external_url` to a shared module and apply at the two unguarded outbound sinks. _Why:_ authenticated SSRF to cloud-metadata / internal services; plan ready (`security-ssrf-outbound-url-validation`).
3. **[high · pm-scrum-master]** Triage the six open PRs (#441–#447); promote #447/#445 (mobile 7A, non-draft) to review first. _Why:_ clears the 7A mobile slice and prevents review-queue stall.
4. **[high · pm-backend]** Implement the missing Epic 81 report-schedule endpoints (pause/resume/executions). _Why:_ frontend already calls them — 404 in production.
5. ~~**[medium · pm-tech-lead]** Land DEC-001 (PR #442) to formally sequence Epic 2B before the dependent Epic 6/8A slices.~~ **DONE — PR #442 merged; sprint-status.yaml sequenced by pm-scrum-master 2026-05-24.**

See `roadmap.md` for the full ranked plan and `action-list.json`/`action-list.md` for the tracker view.

## Blockers

- **Epic 2B WebSocket/notification infrastructure** — gates 8a-3 (preference sync) and 6-5 (DM realtime). Not started; DEC-001 (PR #442) merged 2026-05-24 — sequencing is now formal. Stories 2b-1 through 2b-c1 are `ready-for-dev` and must be picked up next.
- **Epic 81 backend endpoints missing** — frontend calls `/schedules/{id}/pause|resume` and `/executions`, which don't exist (404).
- **PR #435 deferred security findings (#438/#439)** — unowned; P0/P1 items should land before further auth-surface feature work.

## Role focus today

- **pm-scrum-master** (always-on): synthesized the 4-PR delivery picture; flagged the #435 deferred-security backlog and the in-flight PR queue.
- **pm-backend** (rotation index 1): traced and confirmed the P1-05 SSRF at `integrations.rs:2743` and `signatures.rs:628`; flagged 10B silent-stub handlers and the missing Epic 81 endpoints.
