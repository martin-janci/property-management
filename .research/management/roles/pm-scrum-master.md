# Role: pm-scrum-master — 2026-06-29

> Delivery lead / coordinator. Always runs. Static read-only.

## Summary

Sprint "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"
saw a **major verification + test-quality remediation wave** this fortnight.
All six Epic 6 stories flipped to done (verified 2026-06-25), and BIT-345/351/340/348/357/359
unblocked the backend CI gate after **718 quarantined tests** were diagnosed and
triaged. Epic 7A is the active delivery frontier with story 7a-2 stuck in
review on a CI-red folder-tests gate, and three stories still ready-for-dev.
Epic 10A remains fully blocked by four open test-hardening gate issues
(#481 / #482 / #487 plus #480 / #484 for 8a-3).

## Sprint progress

- **Sprint:** Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth
- **Epics done:** 3 / 6 (epic-6, epic-8a, epic-10b; epic-6 verified 6 / 6 this fortnight)
- **Endpoint checklist:** 811 / ~2006 done (40.4 %); 926 projected after pending merges

## Shipped since last run (2026-06-16 → 2026-06-29)

- **Epic 6 fully complete:** stories 6-1 through 6-5 verified done
  (PRs #1832 / #1834 / #1835 / #1844, commits 2026-06-25).
- **#1948 (BIT-351)** — quarantine 718 blind-CI failures; unblock backend dev test gate.
- **#1945 (BIT-345)** — file-level `dead_code` sweep across 132 test files; restore
  `-D warnings` build.
- **#1932 (BIT-340)** — governance / voting happy-path 2xx coverage (+69 endpoints, Wave 5).
- **#1944 (BIT-348)** — compliance / AML / EDD / DSA / moderation happy-path coverage
  (+26 endpoints, Wave 1B re-cut after #1865 lost).
- **#1953 (BIT-357)** — admin / platform-admin happy-path 2xx backfill (+26 endpoints).
- **#1951 (BIT-359)** — `document_share_type` enum-cast runtime panic fix
  (`get_shares_rls` cast to text).
- **#1928** — CI toolchain fix: restore `dtolnay/rust-toolchain @1.94.1` (revert of bad
  dependabot #1837 that broke every backend job).
- **Screen-map reconciliations:** #1909 / #1914 / #1916 / #1918.

## Next up

1. **HIGH — pm-backend:** resolve 7a-2 CI red on `document_folder_tests`
   (FK/isolation). PR #1316 round-2 needed. DoD: green CI + merge.
2. **HIGH — pm-backend:** deliver 7a-3 (permission-based access) — ready-for-dev,
   on the critical path to Epic 7A done.
3. **HIGH — pm-backend:** close or formally defer test-hardening gate issues
   #481 (OAuth revocation) and #487 (MFA rate-limit) to unblock Epic 10A.
4. **HIGH — pm-backend:** merge the open BIT-258 test-backfill wave PRs
   (#1923 reality + ai-auto, #1934 financial, #1938 reality-server Wave 6,
   #1921 integrations / docs / notifications) now that dev CI gate is green;
   endpoint checklist crosses 926 done (46 %).
5. **MEDIUM — pm-frontend:** wire dispute party-submissions endpoints for 80-3 (PR #1846
   open) and complete 5-step wizard redesign for 80-2 to close both partial MVP stories.
6. **MEDIUM:** flip sprint-status `epic-6.stories_completed` to 6 and `status` to `done`
   to reflect verified ground truth; update `coverage.json` partial entries for 6-1
   through 6-5.

## Blockers

- **7a-2 folder organization** — CI red on `document_folder_tests` (FK / isolation fix in
  PR #1316 round 1); story reverted from done → review pending green CI. Owner: pm-backend.
- **10a-1 / 10a-3 OAuth** — gated by #481 (refresh revocation tests) and #487 (MFA rate-limit
  coverage). Owner: pm-backend.
- **10a-2 OAuth client reg** — gated by #482 (ProtectedRoute tenants[0] fallback). Owner: pm-frontend.
- **7a-5 document sharing** — gated by #485 (share-panel UUID validation) + new cross-org
  fan-out risk surfaced by pm-security this run. Owner: pm-security.
- **Backend CI repair backlog (BIT-354)** — 60 RLS / IDOR / authz tests quarantined by BIT-351
  with disposition = FIX (clusters A / B / C); no sprint slot assigned. Owner: pm-backend.

## Risks

- 718 quarantined tests (BIT-351) need cluster-by-cluster repair; without sprint slot,
  coverage erosion compounds. **High / high.**
- Epic 10A stories at ready-for-dev with 4 open gates; if gates persist at sprint end,
  OAuth ships zero stories. **Medium / high.**
- Epic 7A: 7a-2 blocking; if CI-red persists, 7a-3 / 4 / 5 sit idle. **Medium / medium.**
- Open PR queue of 28 including 6 critical fix PRs (messaging IDOR #1799, payment-reminder
  dedup #1804, N-party delivery #1802, OCR auth #1797, Stripe hardening #1824, rentals PII
  #1823); merge-conflict + security exposure grows daily. **Medium / medium.**
- sprint-status epic-6 header contradicts development_status (stale metadata = false
  planning signals). **High / low.**

## Open questions

- Is `document_folder_tests` CI-red a schema migration gap or a logic regression?
  Does repair require a live DB run?
- Formal disposition of test-hardening gate issues #480 and #484 (notification pipeline
  serial dispatch + FCM stub silent swallows) — deferred post-8a or actively scheduled?
- Do the 60 quarantined RLS / IDOR / authz binaries (BIT-354 cluster A) require a
  `NOSUPERUSER` role test-helper that doesn't yet exist in the db crate? Who builds it?
- Is Epic 80 in-scope for the current sprint or scheduled as a separate epic sprint?
- Endpoint checklist at 811 / ~2006 (40.4 %): committed target completion date for
  BIT-258 wave coverage? Does it gate the next release cut?

## Decisions needed

- Gate issues #481 / #487 (OAuth security/quality) — close with fix this sprint or
  formally defer to a dedicated security hardening sprint. Owner: pm-backend + tech lead.
- Epic 7A sequencing — start 7a-3 / 4 / 5 in parallel with 7a-2 CI repair, or wait for 7a-2 merge.
  Owner: pm-backend.
- BIT-354 repair timeline — sprint slot + owner for each of clusters A / B / C.
  Owner: pm-tech-lead.
- Draft fix PRs #1797 / #1802 (OCR auth + N-party messaging IDOR, both high-severity) open
  for 5+ days without review — approve merge or assign explicit reviewer.
  Owner: pm-backend.
