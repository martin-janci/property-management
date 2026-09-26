# pm-security — 2026-09-26

_Rotating role this run (pm_cursor idx 5 → advances to 6, pm-data next). Static read; no compile/run._

## Summary

Sprint-story code is clean/reconciled, but two cross-tenant IDOR fixes (**#2944** violation
comments/evidence/payments read without org scoping; **#2945** portfolio_properties read+write
missing org verification) remain unmerged despite their tracking issues showing **closed** on
GitHub. Their retry-1 fixes (draft PRs **#2977** and **#2976**) are still open with
`mergeable_state: unstable`, stuck behind cloud-runner CI. The blocker is cited as
**#2652**, but that issue's actual body describes a mobile-native/KMP Gradle
`dl.google.com` egress 403 — not the swagger-ui/api-server egress that supposedly blocks
#2944/#2945. This mismatch means the wrong root cause is being tracked.

A new front-end auth-bypass (**#2978**, ppt-web AI-chat + OCR raw `fetch()` bypassing the
authenticated API client) already has a draft fix (**#2979**, `mergeable_state: dirty`); the
issue fails-closed today (401 behind `<ProtectedRoute>`) but is the same anti-pattern as the
already-fixed #486. Two legacy security items remain open: the `security-llm-doc-idor` plan
(Epic 64 `ai.rs` handlers drop `_principal` and run tenant-blind SQL on `self.pool`) and
`#480` (WS auth token in query string, no re-validation after JWT expiry).

## next_actions

- **[high]** Unblock and merge retry PRs #2977 (IDOR #2944) and #2976 (IDOR #2945). DoD:
  both PRs pass CI clean (not `unstable`) and merge to `dev` with regression tests.
  dependency: rust-backend.
- **[high]** Resolve the api-server cloud-build blocker cited as #2652. DoD: `cargo
  build/test` for api-server runs green in the cloud runner without egress 403.
  dependency: none.
- **[high]** Reconcile #2652's actual body (mobile-native/KMP `dl.google.com` egress)
  against the claimed api-server/swagger-ui blocker. DoD: correct root-cause issue is
  filed/linked for the #2944/#2945 retry-loop blocker. dependency: none.
- **[high]** Merge #2979 to route ppt-web AI-chat + OCR hooks through the authenticated
  API client. DoD: `useAiChat.ts` and OCR hooks use `getApiClient()` with Bearer
  injection; PR out of dirty/draft state. dependency: react-web.
- **[medium]** Re-open or correct the closed state of #2944/#2945 given their fixes are
  unmerged. DoD: issue state matches actual remediation status on `dev`. dependency: none.
- **[medium]** Promote `security-llm-doc-idor.md` (ai.rs cross-tenant IDOR, Epic 64) from
  research plan to an owned backlog item. DoD: tracking issue opened; principal binding
  + org predicate added to `publish_description` / `list_listing_descriptions` /
  `get_photo_enhancement`. dependency: rust-backend.

## risks

- **[H/H]** Cross-tenant IDOR in violation records and portfolio_properties (#2944/#2945)
  remains live on `dev` while marked closed. Mitigation: do not treat as resolved until
  #2976/#2977 merge; block release if unmerged.
- **[M/H]** Wrong root-cause issue (#2652) tracked for the retry-loop blocker delays real
  fix. Mitigation: verify actual cloud-runner failure mode before further retries.
- **[M/M]** AI-chat/OCR unauthenticated fetch pattern (#2978) could silently regress if a
  future change removes the fail-closed 401 (e.g. an open proxy route). Mitigation: merge
  #2979 and add a lint/test guarding against raw `fetch()` bypassing the API client.
- **[M/H]** `ai.rs` Epic-64 LLM handlers run tenant-blind SQL on an unscoped pool
  connection (no RLS). Mitigation: implement `plans/security-llm-doc-idor.md` scoped
  queries.
- **[M/M]** WS auth token logged in query param, no re-validation after JWT expiry
  (#480, open since 2026-05-26). Mitigation: close #480 before next release cut.

## open_questions

- Is #2652 actually about swagger-ui/api-server egress, or about mobile-native/KMP AGP
  egress to `dl.google.com` as its current body states — which is the real blocker for
  the #2944/#2945 retries?
- Why do #2944 and #2945 show closed on GitHub while their retry-1 fix PRs (#2976/#2977)
  are still open drafts with `unstable` mergeable state?
- What is the current status/owner of `security-forgot-password-no-rate-limit.md` and
  `security-realtors-mark-inquiry-read-idor.md` (not reviewed this pass — file-read budget)?
- Does PR #2979's "OCR hooks" fix cover a distinct hook from `useAiChat.ts`, or the same
  file? Confirm full scope before merge.

## decisions_needed

- Whether to block the next `dev`→`main` release cut until #2944/#2945 IDOR fixes actually
  merge (not just draft) — owner: pm-security / release manager.
- Whether #2652 needs to be split/reclassified given the swagger-ui vs mobile-native
  egress mismatch — owner: devops / infra.
