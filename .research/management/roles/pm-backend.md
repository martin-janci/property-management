# pm-backend — 2026-10-26

_Rotating backend lens; last ran 2026-07-30 (87 days stale — long overdue). Static read; no compile/run._

## Summary

Backend queue has one brand-new critical blocker (**#3168 cargo-deny — 3 new RUSTSEC advisories gating every backend merge** — second cargo-deny gate in a month after #3124 cleared 2026-10-19) and one tier-1 authz hole surfaced via issue triage (**#3164 violations add_evidence ungated** — same shape as the historical dispute add_evidence gap #2483, suggesting the `add_evidence` sub-route pattern is systemically under-tested). Churn signals are healthy — the two cross-tenant IDOR test suites (`violations_role_authz_tests.rs`, `community_cross_tenant_idor_tests.rs`) are being hardened — but the actual IDOR fix for #3164 has not been drafted yet.

## next_actions

1. **[high]** Gate violations add_evidence sub-resource with a tenant-scoped authz check; add cross-tenant integration test on the `community_cross_tenant_idor_tests.rs` pattern (DoD: #3164 closed with a merged PR that includes a failing-on-main regression test). dependency: none.
2. **[high]** Resolve #3168 cargo-deny advisories (bump the offending transitive crate(s) or add bounded cargo-deny exceptions with a 30-day deadline) — gates every backend merge today. DoD: cargo-deny CI green on dev. dependency: pm-security owns the triage; pm-backend owns the Cargo.toml bump.
3. **[medium]** Diagnose #3131 merge-pipeline deadlock — long-stale; likely sqlx transaction scope or advisory-lock contention in the merge path (needs a traced repro, not more guessing). DoD: root cause documented + narrowing-test added. dependency: none.
4. **[medium]** Sweep remaining `add_*` sub-routes (announcements/attachments, documents/signatures, mediation/evidence) for the same ungated pattern that bit #3164 — turn it into a one-shot audit before shipping the fix. DoD: audit report in `.research/management/` with pass/fail per sub-route. dependency: none.
5. **[low]** Push_fanout FCM eviction (PR #3161 draft) — review-ready; carried `repeated-churn-push_fanout` item is addressed by this PR landing. DoD: PR #3161 merged. dependency: none.
6. **[low]** Un-quarantine the dispute KPIs test (carried risk `pm-backend-disputes-kpis-window-validation-gap-2026-07-30`) — still open from the 2026-07-30 pm-backend run. DoD: test re-enabled with window_start<=window_end validation. dependency: none.

## risks

1. **[prob=high, impact=high]** violations add_evidence authz gap (#3164) — same shape as #2483; systemic add_evidence pattern blind spot. **Mitigation:** fix + audit (see next_actions #1 and #4).
2. **[prob=high, impact=high]** RUSTSEC advisories gating backend merges (#3168) — second cargo-deny gate in a month. **Mitigation:** resolve #3168; schedule quarterly cargo-tree hygiene review.
3. **[prob=medium, impact=high]** Merge pipeline deadlock (#3131) is now months old — any backend contributor hitting it loses hours. **Mitigation:** force a traced repro this sprint.

## open_questions

1. Is the merge-pipeline deadlock (#3131) correlated with specific migration patterns or arbitrary PRs? No one has posted a repro recipe in the issue.
2. Does the `add_evidence` ungated gap affect only the public `violations` sub-route or also platform-admin paths?
3. Which transitive dep introduced the #3168 RUSTSEC advisories — is it another `rust_xlsxwriter → zip` chain, or something new?

## decisions_needed

1. Policy: should newly-introduced sub-routes require a cross-tenant IDOR test by DoD? — owner: pm-tech-lead.
2. Should #3168 trigger a bounded cargo-deny exception or a straight-up version bump — standing default? — owner: pm-security.
