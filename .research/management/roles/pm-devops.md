# pm-devops — 2026-08-29

_Rotation slot: idx 4 → 5 · last run 2026-06-16 (74 days stale, high-priority refresh)._

## Summary

CI unblocked today by PR #2874 (chacha20 lockfile bump — RUSTSEC advisory yank on `chacha20 0.9.1`), but the second-standing RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS, gh-issue-2797) has been open >11 days blocking every backend PR at the cargo-deny gate — this is now the critical infra hotspot. Dispatcher stack is drained (0 new open PRs today), but 3 dependabot batches #2865/#2866/#2867 have sat 2 days idle and the 84-x partials show a dispatcher-blindness pattern worth an audit.

## Next actions (6 max)

- **[high] Close RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS, gh-issue-2797)** — bump h2 workspace-wide to the fixed release; every backend PR is auth-gated on cargo-deny passing. — dependency: none — DoD: cargo-deny green on dev; issue #2797 closed.
- **[medium] Land or bulk-close dependabot batches #2865/#2866/#2867** — 2 days idle; CI is unblocked now that #2874 landed the chacha20 lockfile bump; the auto-approve gate needs a manual trigger past its 2-min buffer. — dependency: none — DoD: 3 PRs merged or closed with rationale.
- **[medium] Add scheduled cargo-deny --lockfile-only on dev** — chacha20 yank surfaced via a red PR check, one dispatcher-run late; a 6-hourly scheduled workflow that opens a GitHub issue on failure would surface yanks/advisories immediately without needing a PR to fail first. — dependency: none — DoD: new workflow lives in `.github/workflows/`; test-fires on a synthetic yank.
- **[medium] Audit dispatcher blindness on 84-1 + 84-2** — 4 upkeep runs have ranked them highest without spawning an implementer; check `.research/plans/` for the gap plans and verify the plan-matcher / claimable() predicate is not silently rejecting them. — dependency: pm-scrum-master shepherding action — DoD: either the plans are present and dispatcher picks one up, or a `top-ranked-stale >7d` alert is added.
- **[medium] AML/moderation hotspot E2E CI job** — 5 of today's 6 PRs touched aml_dsa moderation surfaces (backend + frontend); schedule a scoped E2E CI job exercising the report → moderate → appeal → decide loop so per-PR regressions surface pre-merge instead of via post-merge code-review. — dependency: none — DoD: new job in `.github/workflows/`; covers PR #2869/#2870/#2871 regressions.
- **[medium] Emit quiet_hours_drain Prometheus counters** — atomic-claim path from PR #2834 has no observability; without counters the >1-replica at-most-once invariant is invisible in prod. Pairs with the pm-qa concurrency test action. — dependency: pm-qa `pm-qa-atomic-claim-concurrency-test-2026-08-25` — DoD: counters visible in Grafana; alerting rule on skip-vs-deliver ratio.

## Risks

- **[high/high] RUSTSEC-2026-0258 h2 DoS >11 days blocking backend CI** — chacha20 yank this week only closed one advisory; the second is still open and cannot be worked around. Mitigation: bump h2 workspace-wide; add temporary cargo-deny allow with hard expiry + tracking issue if no upstream fix.
- **[medium/medium] cargo-deny advisory detection is PR-triggered, not scheduled** — chacha20 was detected only when a PR ran backend.yml, one dispatcher-run late; response was same-day but a slower one would have blocked every dev PR for hours.
- **[high/medium] 84-1 + 84-2 partials ranked-but-never-spawned for 4 windows** — likely a plan-file gap or claimable() predicate defect in the dispatcher; the ranker's work is being silently dropped.

## Open questions

- Are gap plans `gap-84-1-*` and `gap-84-2-*` present in `.research/plans/`? If yes, why is claimable() rejecting them?
- Should the h2 bump be a single workspace-wide patch PR or split per crate? (Impacts blast radius vs CI feedback speed.)
- Is the AML moderation E2E job worth the CI-minute cost, or should it be nightly-only?

## Decisions needed

- Blocking-CI-on-scheduled-cargo-deny yes/no (fail-fast dev vs warn-only) — owner: pm-tech-lead + pm-devops.
- Ownership handoff for RUSTSEC advisories that lack a workspace-wide fix (temporary allow with expiry vs vendor-and-patch) — owner: pm-security + pm-tech-lead.
