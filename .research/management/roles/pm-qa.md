# pm-qa — 2026-08-09

**Role:** pm-qa (rotating; last run 2026-06-15, 55d stale)

## Summary

Test coverage on the 19-PR window is **materially stronger than the orchestrator's file-count heuristic reports**. All three fix PRs the orchestrator flagged (`#2707`, `#2710`, `#2714`) DO ship regression tests — they live as `#[cfg(test)] mod tests` at the bottom of the modified source files, not under `tests/`. The real gaps are (a) five pure-refactor churn dedupes on auth/layout/reports with no new behavioural test but no new behaviour either, and (b) `#2547` scheduler retention prune still flagged `hotfix-no-test` (carryover, not new).

## Coverage audit (PR-by-PR, this window)

| PR | Kind | Test file present | Coverage verdict |
|----|------|-------------------|------------------|
| #2707 memory DoS cap | fix | inline `read_capped_body_truncates_oversized_response` + `..verbatim_under_limit` in `api_call.rs` | OK — over/under boundary |
| #2710 SSRF DNS-rebind TOCTOU | fix | inline `execute_rejects_dns_rebinding_to_private_ip_at_connect` (`#[tokio::test]`) in `api_call.rs` — IG3-shaped (fails on dev pre-fix) | OK |
| #2712 dispute add_evidence audit | feat | `dispute_lifecycle_tests.rs` +80 LOC — `add_evidence_writes_access_audit_event` (IG3: fails on dev) | OK |
| #2714 scheduled notification decouple | fix | inline `#[sqlx::test]`: `test_announcement_notification_retries_after_transient_error`, `test_..stamps_empty_audience`, `test_vote_started_notification_retries_after_transient_error` | OK |
| #2716 platform-settings PATCH/GET | feat | dedicated `platform_settings_patch_tests.rs` (+210 LOC) | OK |
| #2717 mobile-config PATCH/GET | feat | dedicated `mobile_config_route_tests.rs` (+87 LOC) | OK |
| #2718 layout webhook HMAC parity | test | `layout-revalidate/route.test.ts` +23 LOC (parity assertion) | Thin — pins body-binding only; does not close #2485 replay-guard risk |
| #2719 inquiry notify route wiring | fix | none new — refactor onto existing `InquiriesHandler` seam covered by #2696 tests | Marginal — relies on prior test seam |
| #2722 community reads gate | security | dedicated `community_unauthenticated_reads_tests.rs` +302 LOC (auth-gate matrix) | OK |
| #2723 announcement fan-out metrics | feat | dedicated `announcement_fanout_metrics_tests.rs` +295 LOC — real SQL, not pure-Rust re-model | OK — partially closes `risk-announcement-fanout-test-fidelity` |
| #2709 ListingForm i18n | fix | updated `ListingForm.test.tsx` +17 LOC | OK |
| #2711 layout tenant dedupe | refactor | none new — behaviour identical | Acceptable |
| #2713 layout admin dedupe | refactor | none new | Acceptable |
| #2715 auth-handler dedupe | refactor | none new (2950 LOC file; behaviour identical) | Acceptable but see risk below |
| #2720 reports helpers extract | refactor | none new | Acceptable |
| #2721 acquire_public_conn extract | refactor | none new | Acceptable |
| #2696 inquiry notifier seam | refactor | inline DB-free unit tests (notifier invoked, failing transport swallowed) | OK |

Verdict: **10/11 code-changing PRs ship regression tests**; refactor-only PRs (5) legitimately reuse existing coverage.

## Systemic-gap assessment

The orchestrator's "three fix PRs had NO test files" claim is a **file-count heuristic false positive** — the dispatcher counts standalone files under `tests/` and misses `#[cfg(test)] mod tests { ... }` inline blocks, which is where the api-server crate co-locates unit-scale regressions. This is worth surfacing to `pm-tech-lead` so the heuristic can be widened (grep `#[cfg(test)]` in modified source files, or count `#[tokio::test]` / `#[sqlx::test]` occurrences in the diff).

## Release-readiness signal

**GREEN** on the current window. No high-severity gap introduced. Two carryover concerns:

1. **#2547 scheduler retention prune** still tagged `hotfix-no-test` — a regression test is queued in `bug-hotfix-no-test-pr-2547` (owner: pm-backend).
2. **`risk-announcement-fanout-test-fidelity-2026-07-23`** — the SQL-integration test recommendation is now **materially satisfied** by #2723's `announcement_fanout_metrics_tests.rs` (real SQL, not model). Recommend downgrading the risk from "medium × high" to "low × high" or closing it after a quick spot-check that the RLS predicate itself (not just the metric read) is exercised.

## `next_actions`

1. **Widen the routine's `hotfix-no-test` heuristic to count inline `#[cfg(test)]` blocks** — high-value token-level fix; today the daily digest reports false-positive test gaps that mislead planning. — priority: medium — dep: pm-tech-lead — DoD: heuristic PR; digest re-run reports the correct 0/19 count on this window.
2. **Add a nonce+timestamp replay-guard test to layout webhook** (#2485 still open) — PR #2718 only pinned body-binding parity; the replay window is still untested. — priority: medium — dep: pm-security — DoD: `layout_webhook_replay_tests.rs` fails on dev pre-fix.
3. **Close out `bug-hotfix-no-test-pr-2547`** — scheduler retention prune regression test. — priority: medium — dep: pm-backend — DoD: dedicated test file exercises prune boundary conditions.
4. **Spot-check announce fan-out RLS predicate coverage in #2723's suite** — verify SQL-level RLS is exercised, not only the metric-aggregation query. If it is, close `risk-announcement-fanout-test-fidelity-2026-07-23`. — priority: low — dep: pm-qa — DoD: risk row `status=closed` with a one-line evidence pointer.
5. **Auth.rs (2950 LOC) still monolithic after #2715 dedupe** — dedupe reduced boilerplate but the module split (`code-review-ppt-web-ui-...`, `repeated-churn-...auth-rs`) is still open. Push for a module-split plan before the next churn cycle. — priority: low — dep: pm-tech-lead — DoD: split plan in `.research/plans/`.

## `risks`

1. **Refactor-without-test on churn hotspots** — 5 pure-refactor PRs (`#2711/#2713/#2715/#2720/#2721`) landed on auth/layout/reports (all top-10 churn hotspots) with no new test asserting behavioural equivalence. If existing suites drift or have coverage holes, a silent regression could ship. — probability: low — impact: medium — mitigation: mandate a "refactor delta covered by ≥1 existing test" check in reviewer verdicts.
2. **Layout webhook replay window still unguarded** (#2485) — `risk-layout-webhook-replay-2026-07-23` unchanged this window; #2718 pinned body-binding but the replay path is untested and unimplemented. — probability: medium — impact: high — mitigation: (see next_action 2).
3. **Inline-test heuristic gap** biases planning — the daily digest and dispatcher may deprioritise pm-qa followups because it "thinks" tests are missing when they aren't, and conversely may miss real gaps that hide under a passing file-count. — probability: high — impact: low — mitigation: (see next_action 1).

## `open_questions`

1. Are the `#[cfg(test)]` tests in `api_call.rs` and `scheduler/mod.rs` actually run by `just verify` / CI, or gated behind a feature flag? (Suspected yes based on cargo defaults, but not verified in this window.)
2. Does the reviewer-verdict prompt distinguish "refactor — no new test acceptable" from "fix — new regression test required"? PR #2547 slipped without a test; #2568/#2571 also had follow-ups.

## `decisions_needed`

- Adopt "inline `#[cfg(test)]` counts as test coverage" for the routine's hotfix-no-test signal — owner: pm-tech-lead.
- Downgrade or close `risk-announcement-fanout-test-fidelity-2026-07-23` after spot-checking #2723's suite — owner: pm-qa (self).
