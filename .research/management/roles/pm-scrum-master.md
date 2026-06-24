# pm-scrum-master — 2026-06-24

## Summary

Sprint (Epic 6, 7A, 8A, 10A) is significantly behind its YAML label — `coverage.json` (2026-06-23) shows most Epic 6 and 7A stories as **partial** or **done** in code while `sprint-status.yaml` still reads `ready-for-dev` or `review`. The status file has not been reconciled with merged work for multiple cycles. Outside the active sprint, **95 PRs merged in 8 days** delivered Epic 11/13/14/18 plus a new ACC Epic scaffold (PR #1821) and a large "split route monolith" wave. Cloud cron ran **8 days late** — last_run_iso was 2026-06-16.

## Sprint progress

- Sprint: Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth
- Epics done: 1 / 6 (per sprint-status; coverage.json reconciliation likely shifts this).

## Shipped since last run (16 highlights)

PR #1822 (story 79-2 auth /auth/callback e2e); PR #1821 (Epic ACC scaffold + :8082 server); PR #1816/#1815/#1813/#1810/#1800/#1798/#1796 (churn-hotspot route/repo splits); Story 11.6/11.7 (financial scheduler + reports); Epic 14 IoT BIT-145 WebSocket + alerts; Epic 13 Sentiment/Predictive dashboards; Epic 18 Story 18.2 Guest ID-doc OCR; messaging camelCase + N-party (#1768/#1756/#1802/#1689/#1696/#1702); BIT-185 fault notifications (#1705); revert #1713 (BIT-213 delegations retirement).

## Next actions

1. **HIGH** — Reconcile sprint-status.yaml with coverage.json (Epic 7A done, half of Epic 6 partial/done).
2. **HIGH** — Unblock PR #1812 (exec bit on `check-rls-enforcement.sh`). *owner: pm-tech-lead*.
3. **HIGH** — Rebase + merge PR #1814 (form.rs split conflict). *owner: pm-tech-lead*.
4. **HIGH** — Close / defer test-hardening-batch #480/#481/#484 gating 8A-3/10A-1/10A-3. *owner: pm-security*.
5. **HIGH** — Fix backlog `security-llm-doc-idor` (residual cross-tenant read IDOR on `list_listing_descriptions`).
6. **MEDIUM** — Fix mobile KMP DeepLinkRouter URL-decoding divergence (backlog `code-review-mobile-native-kmp-deeplink-token-not-url-decoded`).

## Risks (added today)

- **HIGH × MED** — Cloud cron ran 8d late; 31 untriaged follow-up issues piled up; routine missed daily surfacing.
- **HIGH × HIGH** — Test-hardening #480/#481 open since 2026-05-25 (30 days) without closure or explicit defer.
- **MED × MED** — ACC Epic introduces a 4th server (:8082) with no sprint planning or owner assignment.
- **MED × HIGH** — Mobile KMP DeepLinkRouter URL-decoding bug — silent iOS SSO failures for tokens with percent-encoded chars.
- **MED × MED** — KMP SearchScreen stale-response race can clobber results.

## Open questions

- Which role owns ACC (PR #1821) and what is its target sprint?
- 31 open untriaged issues — this sprint or batch to next?
- Are #1801 (event-bus lag) / #1807 / #1804 (scheduler dedup) in scope this sprint?
- Owners + target merge dates for #1799 / #1819 / #1823 / #1824 / #1825?
- BIT-213 delegation retirement — follow-up doc cleanup needed?

## Decisions needed

- Close or defer #480 / #481 (security-high) before OAuth stories ship to staging.
- Assign sprint slot + owner role for Epic ACC.
- Triage decision for the 31 open follow-up issues.
- Confirm 79-2-authentication-flow done given PR #1822 + coverage.json now reflects.

## Blockers

- PR #1812 — CI failing (exec bit lost on check-rls-enforcement.sh). owner: pm-tech-lead.
- PR #1814 — form.rs modify/delete conflict vs dev post-#1781. owner: pm-tech-lead.
- Stories 10A-1 / 10A-3 / 8A-3 — gated by #480 / #481. owner: pm-security.
