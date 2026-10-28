# pm-frontend — 2026-10-28

_Frontend/mobile delivery lens. Daily PM rotation (rotation index 2 → 3). Static, read-only analysis. Last run: **2026-08-06** (83-day stale slot — a quiet frontend window, but the rotating-expert pass has been dripping steady UI defects into the backlog in the meantime)._

## Summary

Reality-web locale/i18n fix (#3179) and the sprint-status rollups fix (#3177) merged this window — both low-risk, test-included. On the mobile RN side, the rotating-expert review surfaced two new VoteDetailScreen defects (ranked-question UI collapsed into unordered multi-select; the `t('key') ?? 'English'` fallback idiom is dead code because i18next never returns nullish). Neither moves a coverage story — all 49/49 remain `done` — but they confirm the frontend quality debt is accumulating faster in the code-review stream than in the coverage schema.

## Delta since last pm-frontend run (2026-08-06)

- **#3177** — ppt-web sprint-status epic rollups fix (small; display-layer).
- **#3179** — reality-web price locale fix (closes a reality-web i18n defect; test included).
- **Mobile RN:** two fresh findings from the 2026-10-27 rotating-expert pass:
  - `code-review-mobile-rn-vote-detail-ranked-no-order` — ranked ballots can't express order (silent data-loss shape, bug).
  - `code-review-mobile-rn-vote-detail-dead-i18n-fb` — `?? 'English'` fallbacks are dead code; raw key leaks on miss.
- **Reality-web churn watch:** `frontend/apps/reality-web/src/app/[locale]/*` continues to show in the top-churn list this run (sell/report/home/components/import). After #3179 that cluster deserves a batched sweep + screen smokes.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| medium | Ship a VoteDetailScreen fix pass — rank-position UI for `ranked` questions + remove the dead `?? 'English'` i18n fallbacks (L180-354) | none | Both items `done` in action-list; e2e covers rank submission; `t()` call sites no longer gate on nullish |
| medium | Adopt `eslint-plugin-react-hooks` + `i18next-hardcoded-strings` in `frontend/apps/mobile` and gate CI on it | none | CI lints block any new occurrence; both VoteDetailScreen findings would have been caught statically |
| low | Reality-web `[locale]` sweep after #3179 — batched locale/pricing/currency audit + Playwright smokes for top 5 locale permutations | pm-qa (fixture capacity) | Smokes green on sk/cs/de/en + fallback; locale/price regressions caught before prod |
| low | 7a-4 mobile DocumentPreview wire-up (carried from 2026-05-25 lens — still unverified) | none | DocumentPreviewScreen calls get_preview_url; app.config.ts + iOS Info.plist keys confirmed |
| low | Screen-map update for ppt-web route refactor PR #2994 (carried) | pm-qa | docs/screens/ppt entries reflect the new route tree |

## Risks

| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| Mobile RN quality debt accumulating as code-review-only findings (ranked-UI gap + dead i18n fallback both caught post-merge on VoteDetailScreen) that coverage.json cannot see | high | medium | Land the two eslint plugins + one-shot sweep for `?? English` and non-multi survey rendering sites |
| Reality-web `[locale]/*` is a persistent churn hotspot (sell/report/home/import) without screen-map coverage proofs — locale/currency regressions can slip past tests | medium | medium | Add Playwright screen smokes for top 5 locale permutations; batch the locale sweep |
| pm-frontend rotation is cold (83-day gap). Frontend-owned backlog has grown to 5 open items while the slot was quiet | medium | low | Shorten the rotation gap once pm-qa/pm-devops hot items are resolved; or raise frontend action density per run |

## Open questions

- Is the mobile RN eslint config (`frontend/apps/mobile/.eslintrc*`) already shipping either plugin? If yes, the lint gap is config/rule-level, not plugin-level.
- Do the `[locale]/sell` and `[locale]/report` churn PRs share a form-infra refactor, or are they independent? (Determines whether the sweep is one PR or several.)
- Is there a Playwright-smoke harness already live in `frontend/apps/reality-web` we can wire into, or does it still need bootstrapping?

## Decisions needed

- Lint-gate scope: `frontend/apps/mobile` only, or extend hardcoded-strings rule to `ppt-web` / `reality-web` as well — owner: **pm-frontend** / **pm-tech-lead**.
- Playwright screen-smoke ownership for reality-web locale sweep — owner: **pm-frontend** / **pm-qa**.
