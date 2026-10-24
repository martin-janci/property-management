# pm-backend — 2026-10-24

_Role rotation slot (daily upkeep). Last ran 2026-07-30 — ~3mo stale, so the lens here is cross-cutting backend state rather than a single sprint-story focus._

## Summary

Backend sprint-level work is parked: sprint-status.yaml's `development_status` block shows all 43 stories `done` and coverage.json agrees 49/49. The current backend queue is dispatcher-sourced code-review / IDOR follow-up PRs — 11 are in `review` with `verdict=approve` awaiting human merge, 2 landed this run (#3145 push-fanout, #3148 ammonia RUSTSEC). The planner is starved on cloud-claimable units because the remaining open rows have no plan files and the 2 `ready` plans are KMP-local-only (blocked by #2652).

## Next actions

| # | Action | Priority | Dependency | Definition of done |
|---|---|---|---|---|
| 1 | Author cloud-ok plan for `code-review-api-core-workflow-condition-err-aborts-batch` (score 2, confidence medium) — `?` on evaluate_conditions at `workflow_executor process_event:359` aborts the whole batch on one bad stored condition | high | none | `.research/plans/code-review-api-core-workflow-condition-err-aborts-batch.md` exists with all 15 required sections, `Mode: cloud-ok`, dispatcher-claimable; backlog row flips `open → ready` |
| 2 | Decay-review 6 backlog rows aged ≥14d (`code-review-api-handlers-violations-number-count-race` 2026-10-11, `code-review-api-handlers-violations-record-payment-non-atomic` 2026-10-11, `code-review-api-handlers-violations-missing-role-authz` 2026-10-17, `code-review-reality-server-register-account-enumeration` 2026-10-17, 2× `code-review-reality-web-*` 2026-10-22/23) | medium | none | Each row either closed as done (code gone / PR merged) or re-scored with fresh grep evidence; dropped if evidence is gone |
| 3 | Commission plans for the 5 other open score-≥2 backlog rows (`code-review-reality-server-add-to-compare-toctou-max-cap`, `code-review-api-core-workflow-condition-err-aborts-batch`, `code-review-ppt-web-core-iot-ws-reconnect-no-backoff`, 2× violations rows) — stem-collision check first (`code-review-api-handlers-violations-*` likely collide with the open PRs in `review` status) | medium | pm-tech-lead for stem/claim-check | 2+ new plans at `Mode: cloud-ok` land in `.research/plans/` this week |
| 4 | Address #3124 (zip-side) — RUSTSEC-2026-0368/0369 against `zip 8.6.0` via `rust_xlsxwriter`; ammonia side closed by #3148/#3143 but zip side is still open and still gates backend merges | high | pm-security for policy call (version bump vs bounded exception) | `cargo deny check advisories` passes on dev; cargo-deny CI gate green |

## Risks

| # | Risk | Probability | Impact | Mitigation |
|---|---|---|---|---|
| 1 | Dispatcher stays starved for another 1-2 runs if no cloud-ok plans are commissioned — Telegram digest surfaces "quiet day" when the real state is "unable to work" | high | medium | New pm-backend `next_action` #1 (api-core workflow plan) targets this; promote in next routine run |
| 2 | 11 open assignments all `verdict=approve` + `needs-human-review` label pile up without a human-merge actor; dispatcher cannot self-approve or auto-merge own PRs | high | medium | Already surfaced in `project-state.md`; needs the human operator to merge-reap, or a bot allowlist change (out of pm-backend scope) |
| 3 | Decay rule has not run against `.research/backlog.json` in a few runs — some stale rows may already be auto-dropped-eligible but still showing `open` | medium | low | Phase 2 of the main routine runs the decay pass; this run will apply it after Phase 1.6 returns |
| 4 | Mobile-native-kmp backlog grows every tier1d review (today added `code-review-mobile-native-kmp-nav-create-listing-stub`) but cannot land in cloud until #2652 unblocked | high | low | Not a backend fix; owner stays pm-devops; keep flagging the compounding cost |

## Open questions

- Is the `backlog+retry avail=0` reading from the planner excluding `-retry1` suffixed rows that are *claimable*, or treating them as retry-exhausted? If the latter, the policy may need relaxing (manual review ok before re-promotion).
- Should the dispatcher be allowed to merge its own PRs after N days of self-approval blocking when CI is green? (Policy question for pm-devops / pm-security.)

## Decisions needed

- Policy: at what buffer level does the routine *proactively* fire a run of `/ppt-project-management scan` locally (needs an operator) vs. keep running upkeep-only in cloud? — owner: pm-scrum-master.
- Dependency on #2652: is there an interim path (self-hosted mobile-native runner, cached Android SDK bundle, Nix shell with pinned SDK)? — owner: pm-devops.
