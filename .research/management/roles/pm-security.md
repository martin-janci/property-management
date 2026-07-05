# pm-security — 2026-07-05

_Rotating role this run (pm_cursor idx 5). Re-fired by buffer-low signal after 39d gap. Static read; no compile/run._

## Summary

OAuth epic 10a carries two open HIGH-severity gating issues (#481 refresh-token reuse, #487 MFA rate-limit gap); a frontend missing-`Authorization`-header pattern is broader than the two already-promoted bugs, recurring across the news feature; and one high-confidence cross-tenant IDOR (`security-llm-doc-idor`) remains unclaimed in the backlog.

## Files reviewed

- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `.research/backlog.json`
- `frontend/apps/ppt-web/src/features/ai-chat/hooks/useAiChat.ts`
- `frontend/apps/ppt-web/src/routes/groups/person-months.tsx`
- `frontend/apps/ppt-web/src/features/news/pages/NewsListPage.tsx`
- `frontend/apps/ppt-web/src/features/documents/hooks/useDocumentDownload.ts`

## Notable findings

- **The `Authorization`-header-drop pattern is not isolated to the two already-promoted bugs.** `useAiChat.ts`'s `apiFetch()` helper never adds an `Authorization` header, and `person-months.tsx`'s `useBuildingUnits` fetch only sets `Content-Type`. The same class of bug recurs at ≥9 more call sites in `frontend/apps/ppt-web/src/features/news/pages/{CreateArticlePage,EditArticlePage,NewsListPage,ArticleDetailPage}.tsx` — including manager-gated mutating actions (`publish`, `archive`, `pin`, comment create/delete, reactions). `useDocumentDownload.ts`'s bare `fetch(url)` is fine (presigned S3 URL by design). `useNotificationAnalytics.ts`, `useMessaging.ts`, `paymentMatching.ts`, and `App.tsx` correctly attach the `Authorization` header. Emitted as backlog vector `code-review-ppt-web-core-news-feature-fetch-unauthed` (score 3).
- **Test-hardening batch governance gap:** `sprint-status.yaml`'s own rule says stories can't be promoted to `done` while their gating security issue is open, but `epic-8a` is marked `done` (8a-2, 8a-3 both `done`) while batch items #480 (JWT in WS query-param access logs, high) and #484 (serial dispatch / FCM failures silently swallowed) are still `status: open` and explicitly gate those two stories.
- **Epic 10a (OAuth Provider Foundation, in-progress this sprint, 0/3 stories complete)** still has two open HIGH-severity gating items: #481 (refresh-token revocation bypassed — revoked tokens reusable, breaks RFC 9700) and #487 (MFA e2e tests missing rate-limit coverage). These must close before any 10a story review.
- **`security-llm-doc-idor`** (backlog.json) remains `status: ready`, high confidence, score 3 — cross-tenant IDOR letting any authenticated user publish/read another tenant's AI-generated listing descriptions and photo enhancements by UUID (`ai.rs:2620/2599/2847`, discarded `_principal`). Plan already drafted at `plans/security-llm-doc-idor.md` but unclaimed.
- **Recent merged batch (#2094–#2099, #1979)** looks like clean debt-paydown with low residual surface: quick-xml XXE now hard-banned via cargo-deny (#2096, supply-chain guard, not a runtime code fix — worth one confirmatory test that no alternate XML parser bypasses it); SlovakAccountingExport honesty-by-construction (#2099) removes a mis-reporting vector by compile-time enforcement; messaging soft-delete/unread invariant test un-quarantined (#2097) closes a test-debt gap adjacent to the existing messaging RLS cross-tenant tests. No new residual attack surface identified from these three.

## next_actions

- **[high]** Extend the useAiChat.ts/person-months.tsx auth-header fix into a repo-wide sweep of raw `fetch()` calls in ppt-web, starting with `features/news/pages` (`publish`/`archive`/`pin`/`comments`/`reactions` all missing `Authorization`) — owner: pm-frontend
- **[high]** Reconcile test-hardening batch rule violation: #480 and #484 are open but their gated stories (8a-2, 8a-3) are already marked done — owner: pm-backend
- **[high]** Fix #481 OAuth refresh-token revocation bypass before any 10a story leaves `ready-for-dev` — owner: pm-backend
- **[medium]** Close #487 MFA rate-limit/brute-force test coverage gap (gates 10a-1) — owner: pm-backend
- **[high]** Pick up `security-llm-doc-idor` (cross-tenant IDOR on AI listing descriptions/photo enhancements) — owner: pm-backend
- **[low]** Add a runtime XXE regression test alongside the #2096 cargo-deny quick-xml ban to verify no alternate XML ingestion path bypasses it — owner: pm-backend

## risks

- **[high/high]** Raw `fetch()` calls without `Authorization` headers across ai-chat, person-months, and news feature.
- **[high/medium]** Test-hardening batch gate rule violated in practice (8a-2/8a-3 done while #480/#484 open).
- **[high/high]** Epic 10a (OAuth) ships with revoked-token reuse (#481) and no MFA brute-force coverage (#487) still open.
- **[medium/high]** `security-llm-doc-idor` sits unclaimed despite high-confidence, score-3 rating.
- **[low/medium]** quick-xml XXE mitigation is a dependency pin, not a code-level test.

## open_questions

- Does api-server reject `/api/v1/news/*` and `/api/v1/ai/chat/*` mutation requests lacking an `Authorization` header, or do they fail open — what is the actual blast radius of the frontend gap?
- Should #480/#484 be closed or explicitly deferred given 8a-2/8a-3 are already merged/done — and who is authorized to make that call given the batch's own gating rule?
- Is `security-llm-doc-idor` scheduled for any upcoming sprint, or purely sitting in backlog with no assigned specialist?
- Was the fix scope for useAiChat.ts/person-months.tsx (already promoted to plans) limited to those two files, or does it already include the news-feature call sites found here?

## decisions_needed

- Formally close or defer test-hardening batch items #480/#484 now that their gated stories are done — owner: rust-backend / release manager
- Prioritize `security-llm-doc-idor` fix against current sprint epics (10a OAuth, 6/7a) — owner: PM/tech lead
