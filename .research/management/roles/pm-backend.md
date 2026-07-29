# pm-backend — 2026-07-29

**Summary:** Backend delivery is dominated this window by three unreviewed accounting-server PRs (#2555 lifecycle, #2558 PDF, #2559 PAY-by-square QR — all opened 2026-07-28 by the same author, no cross-review yet) plus the layout hardening + event-emission wave (#2478 + #2549). Layout risk cluster from #2485/#2486 is being worked but the hardening changes in `routes/layout/{tenant,admin,webhook}.rs` (1535 LOC total, touched twice this run) still lack integration tests for the TOCTOU/replay paths, and #2547 shipped without an api-server regression test — the "hotfix-no-test" signal fired again this run.

## Next actions

1. **[high]** Review + land PR #2555 (sent/cancelled invoice lifecycle) as prerequisite for #2558 + #2559. #2555 introduces the `InvoiceStatus::Sent`/`Cancelled` enum variants both dependent PRs consume; merging them out of order will cause rebase churn in accounting-server. _DoD:_ #2555 CI green + one cross-reviewer approval, then #2558/#2559 rebased and reviewed.
2. **[medium]** Supply-chain check on new deps `crc32fast` + `lzma-rs` introduced by #2559 (PAY-by-square encoder). Both pure-Rust so no C-toolchain surprise, but fold into the pending `sec-ammonia-supply-chain-audit-2026-07-23` cargo-audit sweep. _DoD:_ zero open advisories, licenses reviewed.
3. **[medium]** Add integration test for layout webhook TOCTOU + replay paths hardened by #2478 (`routes/layout/webhook.rs`, `routes/layout/tenant.rs`). Also unlocks the `sec-layout-webhook-integration-test-2026-07-23` follow-up. _DoD:_ failing-on-main test covers publish TOCTOU + replay window rejection.
4. **[medium]** Post-#2504: add e2e route test asserting `/documents/{id}/signature-requests` list/create is reachable — the mount regression (BIT-313) had zero test coverage before this fix. _DoD:_ regression test in `signatures_tests.rs`.
5. **[medium]** Land the `test(backend) dev-team follow-ups` from #2557 — backend hardening backlog spun out of the pre-merge dev-team review of the layout stack.
6. **[low]** Plan module-split for `backend/servers/api-server/src/routes/reports.rs` (3329 LOC, runs_seen=3). Recurring recommendation carrying forward from 2026-05-24 pm-backend — auth.rs (2950 LOC, runs_seen=4) is now the second candidate.

## Risks

- **Single-author accounting-server slice (#2555/#2558/#2559)** — three cohesive slices, one author, no cross-review yet. Piecemeal merges without lifecycle-first ordering will cause rebase churn. probability high · impact medium.
- **Layout webhook hardening without integration test** — #2478 hardened authz + publish TOCTOU + webhook replay + defensive rendering in the same window as #2549 event emission; 1535 LOC across 6 files touched twice this run with no failing-on-main test locking the hardening in. probability medium · impact high.
- **auth.rs size-review risk** — 2950 LOC, runs_seen=4, repeatedly touched by security/2FA work. Same file-size + RLS-omission pattern that produced 10B silent-stub findings. probability medium · impact high.
- **hotfix-no-test-pr-2547** — scheduler retention prune shipped last window without an api-server regression test; still on the open action-list (`bug-hotfix-no-test-pr-2547`). probability medium · impact medium.
- **New lzma-rs / crc32fast deps for bysquare QR** — spec-invalid payloads possible if `lzma-rs` encoder defaults ever bump; the reference-vector round-trip test guards this but the crate isn't pinned. probability low · impact medium.

## Open questions

- Is there a designated cross-reviewer for the accounting-server slice, or does it always land solo-author? (informs the #2555/#2558/#2559 merge sequencing decision)
- Does `sec-ammonia-supply-chain-audit-2026-07-23` intend to cover new-in-window crates, or only the crate that triggered it (ammonia)?
- Is the `pm-backend-reports-rs-split` recommendation blocking any active work, or is it purely tech-debt hygiene that can wait for a low-churn window?

## Decisions needed

- Merge sequencing for the three open accounting PRs: enforce #2555-first, or allow parallel review with rebase-on-demand? — owner: pm-tech-lead + pm-backend.
- Split threshold for oversized route files (auth.rs 2950, reports.rs 3329, and older watchlist: organizations.rs 4018, integrations.rs 4327) — is there a numerical trigger, or purely reviewer discretion? — owner: pm-tech-lead.
