# PPT Project State

_Generated: 2026-06-26 — daily PM rotation (Scrum Master + pm-security; pm_cursor 5→6, coverage_cursor 12→0 epic-9→epic-10a). Coverage `scan_kind=upkeep`. Routine lag: 10 days since last run._

## Executive summary

- **292 PRs merged in 10-day window; promotion-lag flood largely cleared.** 9 stories reconciled to done this cycle (6-1/6-2/6-3/6-4/6-5, 79-1, 10b-5, 10b-7, 80-2). Coverage now sits at **46 done · 3 partial · 0 not-started** of 49 stories.
- **Dispatcher buffer collapse is the new headline blocker.** action-list at 5/36 open (14%), claimable=1/72; Tier-1 coverage exhausted. This rotation refills the buffer to 30/36 from ranked coverage + security candidates.
- **pm-security (30d stale) flags 2 high-severity findings + 3 high-priority next actions:** (1) #481 OAuth revocation appears fixed in code (oauth.rs:413 `revoked_at IS NULL` + services/oauth.rs:511 `is_revoked()` guard) but still open in sprint-status — blocks 10a-1/10a-3 promotion; (2) #480 WS JWT in URL infra-log exposure — needs WS ticket endpoint; (3) accounting-server (PAP-312, +2084 LOC PRD merged #1817; +2254 LOC web scaffold #1808) has zero threat model.
- **6 churn-hotspot module splits landed 2026-06-24:** sensor (#1810), document (#1683), subscription (#1796), vendors (#1813), iot (#1815), reserve_funds (#1816), enhanced_tenant_screening (#1800). emergency.rs (1681 LOC) and subscription.rs (1240 LOC) remain unsplit — queued.
- **3 drafts gated on human review:** #1812 (reality-portal-rs split), #1814 (form-rs split), #1846 (80-3 mediation, scope_drift). All blocking downstream work.
- **Cross-org IDOR cluster on OTA path closed:** #1467, #1601, #1635, #1639, #1741 merged. reality-server portal-imports IDOR (#1561) also closed. 3 migration-version repairs landed + CI guard (#1755/#1757/#1724).
- **Saved-search/group-thread workers shipped:** #1849 (BIT-139 email/push drainer), #1847 (BIT-140 alert_frequency), #1848/#1853 (BIT-206/244 group-thread participants), #1850 (org-scoped favorite alert worker).

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** — Epic 6 effectively closed via reconciliation flood (6-1 through 6-5 all done); Epic 10B closed (10b-5/10b-7 reconciled). 10A still blocked on #481/#487 security gates.

## Shipped since last run (top 10 of 292 PRs since cursor #1439)

- **#1832/#1834/#1835/#1844** — Epic 6 stories 6-1/6-2/6-3/6-5 reconciled to done
- **#1831** — 10b-7 contextual help reconciled to done (full 10B closure)
- **#1829** — 10b-5 support data access reconciled (refresh_tokens column bug fix)
- **#1830** — 79-1 API client reconciled to done
- **#1845** — 80-2 dispute filing reconcile (UC-38 i18n fast-follow closed)
- **#1822** — 79-2 reality-web SSO callback e2e shipped (gap-79-2-auth-callback-e2e)
- **#1810/#1683/#1796/#1813/#1815/#1816/#1800** — 6 churn-hotspot module splits
- **#1849/#1847** — saved-search workers (BIT-139/BIT-140)
- **#1848/#1853/#1850** — group-thread participants + favorite alert worker (BIT-206/BIT-244)
- **#1817/#1808** — accounting-server PRD+epics + accounting-web Next.js scaffold (PAP-312)

## What's next (top 5 actions)

1. **[high] Refill dispatcher buffer to >=18 open items + verify Tier-2 endpoint config** (sm-refill-dispatcher-buffer-claimable) — owner: pm-tech-lead. This rotation took it from 5→30; verify dispatcher consumes the queue.
2. **[high] Formally close issue #481 (OAuth revocation bypass)** — owner: pm-security. Code fix verified; needs sprint-status update + final test pass.
3. **[high] Implement WS ticket endpoint (close #480 permanently)** — owner: pm-security/rust-backend. JWT-in-URL hits nginx/CDN logs.
4. **[high] Produce accounting-server security ADR before any backend code lands** — owner: pm-security. PAP-312 surface is +2084 LOC + +2254 LOC with no threat model.
5. **[high] Human reviewers needed on draft PRs #1812 / #1814 / #1846** — owner: pm-tech-lead + pm-frontend.

## Blockers

- **Dispatcher buffer collapse** — Tier-1 coverage exhausted; claimable=1/72. Refilled this run to 30; planner refill cadence needs to keep pace.
- **PR #1846 (80-3 mediation)** — draft, scope_drift gated; 80-3 cannot promote to done.
- **PRs #1812, #1814** — module-split refactors, draft + needs-human-review.
- **Issues #480, #481, #487** — gate 10a-1/10a-3 promotion. #481 already fixed in code; needs formal close.

## Role focus today: **pm-security** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, last 2026-05-27, 30d stale): 7 new next_actions appended to `action-list.json`; 3 new risks appended to `risks.json`; 3 new decisions noted. Full role JSON in `.research/management/roles/pm-security.md`. Headline: #481 fixed in code but still open in sprint-status; #480 WS-token-in-URL needs ticket endpoint; accounting-server has zero threat model.
- **pm-scrum-master** (always-on): produced the delivery synthesis above. Headline = buffer collapsed at 14%; Epic 6 closure flood; 10A blocked on #480/#481/#487; accounting-server new surface needs sequencing decision.

## Coverage upkeep (epic-9, idx 12)

epic-9 has 1 story (9-1 TOTP 2FA Setup) — already `done` with high confidence; `last_checked` advanced to 2026-06-26. No status changes. Coverage cursor advances to 13 mod 13 = 0 → next run upkeeps **epic-10a**.
