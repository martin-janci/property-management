# PPT roadmap

<sub>Generated: 2026-11-08T18:35:00Z UTC by the research routine (Phase 1.6, upkeep mode).</sub>

> **Canonical source:** `.research/management/coverage.json` (49 stories, 13 epics, all status=done at scan_kind=upkeep). This file is regenerated after every Phase 1.6 run.

## State of the project

- Stories: **49 done / 0 partial / 0 not-started** (all tracked stories are complete).
- Screen coverage: 0 stories without screen-map, 0 orphan epics, 0 orphan screens, 0 missing UC links.
- Top 3 risks right now: (a) dispatcher pipeline stall (#3357) — buffer at claimable=6/72; (b) non-mechanical conflict re-land drift on the orphaned PRs; (c) mobile-rn auth flow (979 LoC) has zero dedicated tests.

## Ranked plan

### Current phase (post-sprint stall remediation)

- [high] Resolve #3357 (re-land #3301 / #3333 / #3356 on current `dev`) — owner: pm-scrum-master / pm-tech-lead — why: the dispatcher produces `0 claimed` while those PRs sit orphaned.
- [high] Verify `code-review-ppt-web-core-refresh-token-localstorage` (ppt-web refresh token in localStorage, XSS-reachable) and promote if confirmed high-confidence — owner: pm-security — why: only non-mobile-native-gated security item in backlog.
- [high] Write test suites for the four mobile-rn auth screens (LoginScreen / RegisterScreen / ForgotPasswordScreen / TwoFactorScreen) — owner: pm-qa + mobile-rn specialist — why: 979 LoC of security-sensitive production code with no dedicated tests.
- [high] Define + test ACs for the Tier-1d signal generator dead-end fix (#3357 item 2) — owner: pm-qa — why: silent stall reproduction without observability.
- [medium] Claim the two READY KMP bugs (inquiries-response-contract, create-listing-not-wired) — owner: mobile-native engineer — why: only score-3 ready items; KMP but acceptable once AGP egress is unblocked locally.
- [medium] Add pipeline-health regression guard (approved-PR-stuck-N-hours + dev-history-rewrite alert) — owner: devops / pm-qa — why: prevent recurrence of the #3357 pattern.

### Screen-map drift

- (none this run — all 49 tracked stories have shipped screen-maps; `/screens update` clean.)

Buffer: 19/36 open · dispatcher claimable 6/72 · research backlog avail=0. ⚠ Buffer below half — this run minted 9 non-gated planned actions to recover.
