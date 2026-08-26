# pm-devops — 2026-08-26

## Summary

Sprint CI is green for the merged backend/frontend PRs this cycle, but the delivery pipeline has a structural cloud-runner gap: all remaining open work is mobile-native/KMP and Android builds fail via AGP dependency egress-403 in the cloud runner (`mobile-native.yml`'s `build-android` job), so nothing in the current dispatcher queue can land without a human/local-runner path. The sole non-KMP PR in flight (#2744, dispatcher un-wedge fix) has sat human-gated for 12 days with only a passive, never-escalating stale-PR guard watching it.

## Next actions

- **[high]** Get a human reviewer to merge or explicitly reject PR #2744 (dispatcher un-wedge for oversize-archive push, approved, 12 days stale, needs-human-review) — dependency: none (human/repo-owner); DoD: PR #2744 merged or closed with reason recorded.
- **[high]** Stand up or designate a self-hosted/allow-listed runner (or proxy egress rule) for mobile-native Gradle/AGP dependency fetches so Android builds stop failing in cloud CI — dependency: none (infra/platform); DoD: `build-android` job in `mobile-native.yml` completes without egress-403 on a test PR.
- **[medium]** Escalate `stale-pr-guard` beyond the run-summary — add a notification/issue-comment step (or Slack/webhook) when a flagged PR crosses `stale_days`, since the current job is flag-only and never surfaces findings outside Actions logs — DoD: `stale-pr-guard.yml` posts a visible alert (issue/comment/webhook) for PRs open >3 days, not just a job summary.
- **[medium]** Audit and clear the dependabot PR backlog (12+ stalled) using the same review pattern as the closed #2473 batch (`docs/runbooks/dependabot-cargo-minor-batch-pr2473.md`) so lockfile drift risk doesn't compound — DoD: open dependabot PR count reduced and each either merged or deferred with a runbook note.
- **[medium]** Re-baseline the dispatcher action-list backlog composition: confirm all 6 open items are genuinely KMP/cloud-unlandable (not misclassified) and route any non-KMP items back into the landable queue — DoD: backlog triage note confirms KMP-only vs. mixed composition.
- **[low]** Watch repeat churn on `backend/servers/api-server/voice_webhooks.rs` (2nd hotspot appearance) for a stability/observability gap (e.g., missing test coverage or retry/logging issue) rather than pure feature churn — dependency: rust-backend; DoD: root cause of repeat voice_webhooks.rs churn identified or ruled out as noise.

## Risks

- **[high/high]** Cloud runner cannot resolve AGP/Google Maven dependencies (egress-403), so all mobile-native/KMP work is fully blocked in cloud CI/dispatch — an entire platform's delivery pipeline is stalled with no landing path. **Mitigation:** Provision a self-hosted runner or proxy allow-list for Google Maven/AGP artifact hosts; document the exception in the `mobile-native.yml` header.
- **[high/medium]** PR #2744 (dispatcher fix for oversize-archive push, #1162/#2743) is approved but stuck 12 days in human-gated review — this is the fix for a dispatcher wedge, so its own delay is itself blocking dispatcher throughput. **Mitigation:** Prioritize human merge/reject decision this week; treat as a devops-critical-path item.
- **[medium/medium]** Dependabot backlog (12+ PRs) stalled increases lockfile drift and delays security/soundness patches (precedent: the merged #2473 batch carried real soundness fixes in `futures-util`). **Mitigation:** Batch-review and merge dependabot PRs on a regular cadence per the existing runbook pattern.
- **[medium/medium]** pm-devops rotation slot was over 2 months stale (last run 2026-06-16) before this routine — infra/CI drift during that gap was unobserved by this role. **Mitigation:** Restore the devops rotation cadence so CI/deploy risk is reviewed at least monthly.
- **[high/medium]** `stale-pr-guard.yml` is explicitly flag-only (never merges, closes, rebases, or escalates outside the Actions run summary) — a human must actively check Actions logs to notice stalled PRs like #2744, which is exactly what happened here. **Mitigation:** Add an active notification channel to the guard so stale/blocked PRs surface without manual log-checking.

## Open questions

- Is there an approved plan/budget to add a self-hosted or egress-allow-listed runner for mobile-native builds, or is the AGP egress-403 constraint considered permanent/by-design for this cloud environment?
- Who owns the human-gated review queue for dispatcher-critical PRs like #2744, and what SLA (if any) applies to `needs-human-review`-labeled PRs?
- What is the current true count and age distribution of open dependabot PRs (gh access was unavailable in this session to verify directly)?
- Does the dispatcher have any fallback path (e.g., manual/local build attestation) for KMP work when cloud CI cannot land it, or does all such work simply queue indefinitely?

## Decisions needed

- Approve infra spend/config change for a self-hosted or egress-exempted mobile-native CI runner — owner: pm-devops / platform lead
- Set an SLA or escalation policy for human-gated dispatcher PRs (e.g., #2744) so approved fixes don't idle for 12+ days — owner: repo-owner / dispatcher maintainer
- Decide whether `stale-pr-guard` should gain active notification/escalation capability or remain flag-only by design — owner: pm-devops
