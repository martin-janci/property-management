# PPT Project State

_Generated: 2026-06-27 — daily PM rotation (Scrum Master + pm-security; routine catch-up after 11d lag). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 12 → 0 (epic-9 → epic-10a)._

## Executive summary

- Sprint 'Epic 6, 7A, 8A & 10A' is in late execution: Epic 8A is done, Epic 10B is fully reconciled done (all 7 stories), and all 6 Epic 6 stories have shipped code — 5 of 6 now formally marked done in sprint-status after the 2026-06-25 reconciliation wave. The sprint is functionally complete except for Epic 7A (1/5 done, story 7a-2 stuck in CI-red review) and Epic 10A (0/3 done, all blocked by open test-hardening gates #481/#482/#487), with a secondary duty to clear 37 open follow-up issues and replay two stranded research-land sessions onto dev.
- **Security read (pm-security rotation):** Sprint has active auth/authz debt across four open THB issues (#480, #481, #482, #487) that gate Epic 10A OAuth stories; messaging attachment IDOR (#1791) has test coverage landed but the GitHub issue remains flagged, while SSO CSRF-skip (#1826) is documented as intentional PKCE-based protection in sso.rs:42-49 but requires independent verification that state param is always validated on callback.
- **Routine lag note:** This is a catch-up brief covering 11 days. Two prior runs (2026-06-22 and 2026-06-25) wrote briefs on session branches that never replayed onto dev via `research-land.yml` — suspect CI-replay regression. Today's run lands directly to dev on the next push.
- **Code review HIGH this run:** `search_alert_drainer.rs:317-325` — duplicate-delivery bug. When the alert successfully sends on at least one channel but the subsequent `mark_search_alert_notified` UPDATE fails, the worker logs and continues without bumping `notify_attempts`; the row keeps `notified_at IS NULL` and re-enters the next 60s poll → users receive duplicate emails/pushes until the mark succeeds. Promoted to plan.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth"** · epics_done=2/6

| Epic | Status |
|------|--------|
| 6 — Announcements & Communication | 5/6 stories reconciled to done in 2026-06-25 wave (sprint-status); coverage.json still shows partial (stale) |
| 7A — Document Management | 1/5 done; 7a-2-folder-organization stuck in CI-red review (PR #1316); 7a-3/7a-4/7a-5 sequentially blocked |
| 8A — Notifications | done (3/3) |
| 10A — OAuth | 0/3 done; gated on THB #480/#481/#482/#487 (pm-security review pending) |
| 10B — Platform Administration | 7/7 done (reconciled) |
| 16 — Saved-search Alerts (Reality) | Story 16.3 + transport drainer shipped (#1847/#1849); HIGH bug just found in drainer |

## Shipped since last successful routine commit (cursor #1439 → 96 PRs in 11d)

- PR #1829 — 10b-5-support-data-access: finish Support Data Access to done
- PR #1830 — 79-1-api-client-integration: reconcile story status to done
- PR #1831 — 10b-7-contextual-help-documentation: reconcile sprint-status to done
- PR #1832 — 6-1-announcement-creation-targeting: verify & reconcile story to done
- PR #1834 — 6-2-announcement-viewing-acknowledgment: finish viewing/ack to done
- PR #1835 — 6-3-announcement-comments-discussion: verify to done
- PR #1843 — 6-4-pinned-announcements: verify and reconcile to done
- PR #1844 — 6-5-direct-messaging: reconcile status to done
- PR #1847 — feat(reality-server): story 16.3 — saved-search alert_frequency cadence (BIT-140)
- PR #1849 — feat(epic-16): email/push transport drainer for saved-search alerts (BIT-139)
- PR #1848/#1853 — feat(messaging): expose+render full participant list for group threads (BIT-206/244)
- PR #1850 — [codex] Add org-scoped favorite alert worker
- PR #1751 — fix(dispatcher): archive-terminal reconciler + unique branch names
- PR #1753 — feat(api-server): fail-fast preflight for required production env vars
- PR #1757 — fix(db): resolve duplicate migration 00192 collision
- PR #1756/#1768 — fix(messaging): camelCase wire contract for attachment/thread/message APIs
- PR #1822 — 79-2-authentication-flow: reality-web SSO callback e2e coverage
- PR #1856 — docs(endpoint-checklist): test-verified endpoint audit (BIT-247)

## What's next (top 5 actions, role focus this run)

1. **[high]** Fix HIGH duplicate-delivery bug in search_alert_drainer.rs:317-325 — mark_search_alert_notified must run in same txn as send, or add idempotency before drainer re-queues on 60s tick — owner: pm-backend
2. **[high]** Fix MED silent DB-error swallow in saved_search_alerts.rs:176-180/237-240 — watermark update failures must surface to preserve documented daily/weekly cadence — owner: pm-backend
3. **[high]** Resolve 11-day research-land replay lag — 2026-06-22 and 2026-06-25 briefs on session branches were never replayed onto dev via research-land.yml; replay or re-generate now — owner: orchestrator
4. **[high]** Unblock Epic 10A by closing/deferring test-hardening gates #481 (OAuth refresh-token revocation), #482 (ProtectedRoute multi-tenant role fallback), #487 (MFA rate-limit test gap) so 10a-1/10a-2/10a-3 can promote — owner: pm-backend
5. **[medium]** Green-CI 7a-2-folder-organization (PR #1316) — FK/isolation fix must pass document_folder_tests before promotion from review to done, unblocking 7a-3/7a-4/7a-5 — owner: pm-backend

## Blockers

- **Epic 10A (10a-1/10a-2/10a-3)** (pm-security): All three gated by open test-hardening issues #481 (HIGH security), #482, #487; cannot promote to done until gates close or defer
- **7a-2-folder-organization** (pm-backend): CI test job red on PR #1316; story reverted from done to review; 7a-3/7a-4/7a-5 sequentially blocked
- **Research pipeline / coverage.json** (pm-devops): 11d routine lag + two unmerged session branches → coverage, sprint-status, backlog rankings on stale state

## Role focus today

- **pm-scrum-master** (always-on): delivery synthesis above
- **pm-security** (rotation #6): see `roles/pm-security.md` for the 6 next-actions + 5 risks added to the registers this run.

