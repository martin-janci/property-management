# pm-frontend — 2026-10-28

_Frontend/mobile delivery lens. Daily PM rotation (idx 2 → 3). **Rotation came up 83 days stale** (last run 2026-08-06). Static, read-only._

## Summary

Backlog has accumulated 12 open frontend/mobile code-review findings while the pm-frontend lens was dormant — three of them are **"wire-the-button-but-not-the-POST"** bugs (ReportFault setTimeout fake-submit; reality-web /sell wizard Publish never POSTs; VoteDetail ranked ballot submits bare options), suggesting a chronic class-of-defect that only an e2e-submit smoke gate will catch. The carried `pm-qa-mobile-rn-lint-gap-hooks-i18n-2026-08-25` risk has not moved in 2+ months and the exact defects it predicts keep landing — adopting eslint-plugin-react-hooks + i18next/no-literal-string in `frontend/apps/mobile` is now overdue.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Wire `ReportFaultScreen.tsx` handleSubmit to real `/api/v1/faults` POST (currently `setTimeout(1500)`) | none | Fault reports reach backend; MSW test asserts the POST payload |
| high | Wire reality-web `/sell` wizard Publish to reality-server `/api/listings` POST | pm-backend (if endpoint shape differs) | Seller submissions persist; wizard shows real success/error states |
| high | Audit ppt-web logout AUTHED_QUERY_KEY_ROOTS allow-list (session leak on shared workstation) | none | Every mounted feature-group root query key invalidated on logout; add unit test |
| medium | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in `frontend/apps/mobile` (closes 2026-08-25 lint-gap risk) | pm-qa | Both rules enabled; one-sprint warn-only grace; CI gate active next sprint |
| medium | Fix reality-web ComparisonUrlHandler 404 (every shared comparison URL broken) | none | Hits a real endpoint; smoke test asserts 200 for a known listing id |
| medium | Fix reality-web listing-detail SSR crash on partial 200 body (JSON-LD build deref) | none | Defensive guards in JSON-LD builder; unit test for missing-field body |

## Risks

| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| Mobile RN + reality-web ship forms with wired-but-not-POSTed Submit buttons — three independent findings in 7 days, high-confidence pattern | high | high | Per-screen e2e-smoke: for every form with Submit, assert MSW/Mirage sees the POST with the collected payload; block merge otherwise |
| pm-frontend rotation stale 83 days while 12 UI findings accumulated — rotation cadence skewing toward backend-heavy weeks | medium | medium | Add staleness-bias to pm_cursor rotation: if any role >60 days stale, prefer it over current index |
| i18n fallbacks dead code across mobile RN (`t('key') ?? 'English'`) — users see raw keys on a locale miss | medium | low | Codemod pass + i18next/no-literal-string; the lint-gap risk's exact prediction |

## Open questions

- Does the ppt-web TanStack query cache leak on logout extend to the SSR-rendered reality-web pages too, or is this a ppt-web-only concern?
- Are the vote-detail (ranked ballot + dead i18n fallback) items being deferred because they're `priority:low` on action-list, or because no one is picking them up? If the latter, they need re-prioritisation now that the pattern is clear.

## Decisions needed

- Adopt eslint-plugin-react-hooks + i18next/no-literal-string in `frontend/apps/mobile` — blocking from day one or warn-only for one sprint? (Carried from 2026-08-25 decision log; still unresolved.) Owner: pm-frontend + pm-qa.
- Does every form Submit-handler need an MSW e2e-smoke assertion before merge, or only ones in critical paths (fault reporting, selling, voting)? Owner: pm-qa + pm-tech-lead.
