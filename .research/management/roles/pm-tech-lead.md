# pm-tech-lead — 2026-10-21

**Routine Phase 1.6 rotating slot. Prior run: 2026-07-27 (86 days ago — fresh architecture lens overdue).**

## Summary

The backend merge gate is a three-layer stack (**cargo-deny**, **dispatcher self-approval**, **cloud egress**) with no owner and no SLA, and it has been blocked ~16 days. `backend/deny.toml` on the checked-out tree **already ignores RUSTSEC-2026-0367/0368/0369 for `zip 3.0.0`** (the utoipa-swagger-ui build-dep path) and has a `[bans]` guard on `zip 4.x–8.x`. So **#3134 may be a different / new advisory, or a stale / mis-reported run**. I could not confirm which — the role is read-only and I have no CI log.

## Next actions

### 1. Triage #3134 cargo-deny failure precisely — [high · pm-security]

Pull the actual failing `cargo-deny` output on `dev` HEAD and name the advisory ID + crate. Three branches:

- **If it is only zip 3.0.0,** find why the existing ignore + ban isn't applying — check whether the PR carrying that `deny.toml` actually merged to `dev`, and the CI's `deny.toml` path.
- **If it is a new ID,** add a dated, scoped ignore with a deadline.
- **Adopt a written policy:** a transitive advisory with no upstream fix and a non-attacker-reachable path (build-script-only) gets an ID ignore + a patched-floor `[bans]` guard + a tracking issue + a 30-day expiry. Reachable runtime paths must be fixed by bumping / replacing the dependency.

DoD: cargo-deny advisories green on dev; policy written into `deny.toml` header or docs; stuck backend PRs can merge; every ignore has an issue link + expiry.

### 2. Resolve the #3131 merge-deadlock — [high · pm-devops]

Recommend **option C**: scoped GitHub ruleset that lets PRs labeled `auto-impl` merge once required CI is green **and** the dispatcher's internal `reviewer_verdict=approve` is recorded. Use **option B** (second bot identity) only if the ruleset cannot enforce it. Keep human review via **CODEOWNERS** for security / migration / auth paths. Reject **option D** (direct commits) — removes the audit trail. In the meantime, a human drains the 16-PR queue oldest-first starting with #3024. Also raise `DISPATCHER_PR_MAX` or add a stuck-PR age alert so `WIP=cap` can't silently halt the planner.

DoD: a bot-authored PR with green CI merges with no human click; the queue drains below the WIP cap; the planner claims work again; the policy is documented.

### 3. Decide the #3132 PRICE_MAP_CACHE design — [medium · pm-backend]

Put a per-IP + global rate limit (tower-governor or the existing Redis limiter) **in front of** the price-map endpoint; make the cache a fixed-size, non-amplifying structure (moka/TinyLFU with TTL, or a plain TTL cache that rejects inserts at the cap instead of running an eviction loop). Normalise or whitelist cache keys so request-controlled keys can't flood it. Add a regression test that floods distinct keys and asserts bounded per-request work.

DoD: ADR / issue comment records the chosen strategy; follow-up PR bounds per-request eviction work; a flood test passes.

## Risks

### R1 — Merge-gate stack single point of failure — probability: high, impact: high

cargo-deny, bot self-approval and cloud egress (#2966, #2652) each independently block all backend merges, with no owner, SLA or escape hatch. Queue blocked ~16 days, 16 approved PRs stuck, tier-1 security fixes (#2944/#2945/#2946 IDOR) not landing. Fixes that cannot merge accumulate merge-conflict / staleness rework.

Mitigation: name one owner for merge-gate health; add a `gate-red > 24h` alert; document break-glass criteria for human merges of security PRs; fix / allowlist swagger-ui egress (#2966).

### R2 — Advisory-ignore debt compounds — probability: medium, impact: medium

`deny.toml` carries many global ID ignores (h2, rustls, rustls-webpki, zip), each pinned to un-upgradable AWS-SDK or utoipa-swagger-ui lines. Version-scoped guards are only possible through `[bans]` (a workaround easy to forget to lift). The long-term fix needs a utoipa 6 or AWS SDK migration that nobody has scheduled.

Mitigation: give every ignore an expiry + issue link; review the ignore list monthly; schedule a spike for the utoipa 6 / swagger-ui upgrade.

### R3 — Unbounded / amplifying cache on a public endpoint — probability: medium, impact: medium

`PRICE_MAP_CACHE` (#3132) still leaves a DoS exposure on main. Related: #3127 shipped a silent data-loss bug (dead-store of uploaded bytes) which suggests weak review / lint coverage for unused-variable + underscore-prefix patterns.

Mitigation: rate limit *before* the cache; deny `clippy::let_underscore_must_use` and `clippy::used_underscore_binding` on handler code; require a persistence assertion in upload-handler tests.

## Open questions

1. What does the #3134 cargo-deny output name as the failing advisory + crate? `deny.toml` already ignores RUSTSEC-2026-0367/0368/0369 for zip 3.0.0, so is this a new advisory, a stale run, or a CI config that doesn't read this deny.toml?
2. Can GitHub rulesets scope a bypass or auto-merge rule to the `auto-impl` label or the dispatcher app identity? Does the dispatcher have a second GitHub App or PAT identity available?
3. What is the cargo-deny policy for ignore expiry, and who owns triage (pm-security or pm-devops)? The risk log lists both as owners of overlapping items.
4. Current traffic profile + key cardinality of the price_map endpoint? Is there an existing rate-limit layer in reality-server?
5. Are the 2026-10-01 risks still open (auth-header triplication #3006, voice_webhooks churn, accounting trio's reviewer starvation)? Not re-verified this run.

## Decisions needed

- Merge-gate policy for bot-authored PRs: scoped ruleset bypass vs second bot identity vs human merge queue — owner: pm-devops with human maintainer sign-off.
- cargo-deny exception policy: scoped ID ignore + `[bans]` floor guard + expiry for non-reachable transitives vs chasing every pull path — owner: pm-security.
- Price-map protection strategy: rate limiter in front of the cache vs smarter eviction vs dropping the cache — owner: pm-tech-lead.
- Fund the utoipa 6 / swagger-ui upgrade spike (would retire zip 3.0.0 ignore and reduce #2966 dependency) — owner: pm-tech-lead.
