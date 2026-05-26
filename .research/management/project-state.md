# PPT Project State

_Generated: 2026-05-26 — daily PM rotation (Scrum Master + pm-devops). Coverage map last rebuilt by `/ppt-project-management scan` on 2026-05-23; upkeep-refreshed 2026-05-26._

## Executive summary

- **21 PRs merged since the last run (#513–#562)** — a big delivery + cleanup window. Feature work: e-signature UI (#513) + fail-closed security (#532), mediation API (#514), push-fanout worker (#515), Booking.com channel sync (#534) + Booking OTA (#544), Airbnb integration backend (#538), contextual help UI (#541), report execution-history UI (#547), document download/preview API (#550), document folder API (#551). Hardening: backend cleanup of 12 follow-ups incl IDOR/TOCTOU/auth (#548), ppt-web/admin-web 7 follow-ups (#549). Tooling: dispatcher auto-promote/harden/self-test/spec-gaps (#559–#562), infra/CI cleanup (#552), mobile cleanup (#553), post-merge review (#529).
- **The entire #516–#528 follow-up issue cluster is CLOSED** — resolved by the cleanup PRs #548/#549. No untriaged issues remain. The security-fix-without-tests debt from the prior run has been swept.
- **OTA channel integrations went live (backend).** Booking.com (#534/#544) and Airbnb (#538) backends merged. These add third-party credentials and always-on sync/fanout workers — new operational surface (secrets, observability, rollback) that DevOps now flags as the top non-feature gap.
- **80-3 mediation still partial.** Mediation API merged (#514) but the App.tsx route wiring is in OPEN PR #555 — DisputeDetailPage/MediationPage stay unreachable until it lands.
- **Documents advancing.** Folder API (#551) and download/preview API (#550) merged; dedicated folder-tree UI (#556) and doc-preview polish (#557) remain in draft.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

| Epic | Tracked status | Real status (from coverage upkeep) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6-1/6-5/6-6 done; 6-2/6-3 web UI wired, still partial pending gates |
| 7A — Basic Document Management | in-progress | 7a-3/7a-5 done; 7a-2 folder API merged (#551), folder-tree UI in draft #556; 7a-4 download/preview API merged (#550), web polish in draft #557 |
| 8A — Basic Notification Preferences | done | 8a-1/8a-2 done; 8a-3 push-fanout worker merged (#515) — observability gap remains |
| 80 — Disputes & Mediation | in-progress | 80-1 done; 80-3 mediation API merged (#514) but UI wiring open in #555 |
| 81 — Reports | in-progress | execution-history UI merged (#547); backend executions/download endpoint verify still pending |
| 83 — Channel Integrations | in-progress | 83-3 done; 83-1 Airbnb (#538) + 83-2 Booking (#534/#544) backends merged — connect/disconnect UI + reconciliation tests pending |
| 84 — Advanced Features | done | 84-2 e-signature UI (#513) + fail-closed (#532) merged — story done |

## What's next (top 5)

1. **[high · pm-frontend]** Land mediation-ui PR #555 — wire App.tsx dispute routes so DisputeDetailPage/MediationPage render (80-3 unreachable until merged).
2. **[high · pm-devops]** Add CI secret-scanning + env-scoped secrets for new OTA credentials (Booking.com/Airbnb in `integrations/install.rs`).
3. **[high · pm-devops]** Add observability (metrics/dead-letter/backoff alerting) to the push-fanout worker (`push_fanout.rs`) before mobile push GA.
4. **[medium · pm-backend]** Add channel-sync reconciliation + OTA round-trip tests for 83-1/83-2; build connect/disconnect UI surface.
5. **[medium · pm-frontend]** Merge folder-tree UI (#556) + doc-preview polish (#557) to close out 7a-2/7a-4 web slices.

See `roadmap.md` for the full ranked plan and `action-list.json`/`action-list.md` for the tracker view.

## Blockers

- **80-3-mediation-resolution** — mediation API merged (#514) but App.tsx route wiring in OPEN PR #555; pages unreachable until it merges. Owner: pm-frontend.
- **OTA channel ops gaps** — Booking/Airbnb backends live without secret-scanning gate, worker observability, or rollback runbook. Owner: pm-devops.

## Role focus today

Role focus today: pm-scrum-master, pm-devops.

- **pm-scrum-master:** 21 PRs shipped; the #516–#528 follow-up cluster is fully closed; OTA channel backends + push-fanout went live. Main open structural item is 80-3 UI wiring (PR #555). Buffer healthy (58/36 open).
- **pm-devops:** New always-on workers + third-party OTA credentials grew the deploy surface ahead of operational tooling. Top three: CI secret-scanning for channel creds, push-fanout observability, and per-channel rollback/feature-flag.
