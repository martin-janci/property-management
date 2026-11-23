# pm-qa — 2026-11-23

## Summary

24 PRs merged in-window, but **12/24 (50%) shipped with zero test files in the diff** — a sharp uptick from the 7/7-tested baseline on 2026-11-09. Three of them are source-only fixes to the newly-split push stack: #3725 (FCM 2xx body-parse → delivered, 1 file), #3726 (worker Redis-requeue drop → bounded retry, 1 file), #3731 (push db-error leak → mapped type, 3 files). The push service now has three untested behavioral changes in flight. Some 0-test PRs are legitimately test-less (#3728 screen-map doc, #3714 30-file locale JSON, #3730 SSO capacity cap — one-line struct change), but the push cluster + #3735 (email action dispatch), #3737 (voice OAuth 503), #3722 ("test(api-server): cover voice oauth refresh rate limiter" but diff shows source-only) need per-PR classification before this becomes a trend. 2026-12-01 mobile i18n lint flip is 8 days out with 7 draft PRs still open.

## Next actions

1. **[medium]** Add regression tests covering #3725/#3726/#3731 — three single-file push-stack fixes shipped without test diffs. Dep: pm-backend.
2. **[low]** Audit this window's 12 no-test-diff PRs — classify each as docs-only, inline-#[cfg(test)], or real test-gap; publish the classification as a prerequisite for a per-crate test-ratio gate. Dep: pm-backend.
3. **[medium]** Carried from 2026-11-09 — adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile, now 90 days unresolved. The 2026-12-01 lint flip is 8 days out and 7 drafts are still in flight. Dep: pm-frontend.

## Risks

- **FCM/worker/push untested fixes cluster (medium/medium)** — three single-file source-only PRs changed push service behavior in one window; next regression here will likely be a real production incident before any test catches it. Mitigation: QA-owned test-shadow burn-down within one sprint-week, plus a #[cfg(test)] requirement on push/* PRs going forward.
- **i18n flip T-8d burn-down stalled (high/high)** — 7 of 8 open PRs are drafts targeting the 2026-12-01 lint flip, all updated in the last 4 hours. Dispatcher is pushing drafts faster than they land. Mitigation: hard promote-or-close by 2026-11-29 to leave a weekend of drainage.

## Open questions

- Is PR #3722's "test(api-server): cover voice oauth refresh rate limiter" misleadingly titled, or are tests embedded inline in `oauth.rs`? The commit message claims coverage but the file list is source-only.
- Does the push-service test-shadow debt justify a one-shot "push/* changes require matching *_tests.rs diff" lint, or is the inline #[cfg(test)] convention sufficient?

## Decisions needed

- Mandate a #[cfg(test)] block (or paired *_tests.rs diff) for every PR touching push/apns.rs, push/fcm.rs, or push/worker.rs — three single-file source-only fixes landed this window. Owner: pm-qa + pm-backend.
- Investigate the 50% no-test-diff rate on this window's merged PRs — classify each PR as legitimately test-less or test-gap and publish the taxonomy before a corrective gate. Owner: pm-qa.
