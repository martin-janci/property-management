# code-review-reality-web-i18n-systemic-workspaces

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-08-30 (buffer-low refill audit via Explore)
**Confidence:** high

## Hypothesis
The reality-web authenticated workspaces (`account/*`, `realtor/*`, plus `forbidden/`) contain a systemic i18n gap: 9 `page.tsx` files render headings, subtitles, form labels and confirm-dialog copy as literal English JSX instead of routing through `useTranslations()` / `getTranslations()` from `next-intl`. This is the same class of bug already fixed on `saved-searches` (PR #2894) and `inquiries` (PR #2895), but scoped to a wider directory tree. Users on `sk` / `cs` / `de` / `hu` / `pl` see English chrome in half the authenticated app. Smallest resolving change: add missing translation keys to the 6 message bundles under `frontend/apps/reality-web/messages/`, then convert the 9 pages to `next-intl` hooks/functions.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/account/password/page.tsx:68-70` — `<h1>Change password</h1>`, `<p>Update the password used to sign in to your account.</p>`; plus labels `Current password` / `New password` / `Confirm new password` (verified inline)
- `frontend/apps/reality-web/src/app/[locale]/account/profile/page.tsx:48-77` — `<h1>Edit profile</h1>` / `<p>Update your display name and contact preferences.</p>` / labels `Display name` / `Email` / hint `Contact support to change the email on this account.` (verified inline)
- `frontend/apps/reality-web/src/app/[locale]/realtor/profile/page.tsx:93-171` — `<h1>Realtor profile</h1>` plus labels `Name` / `Email` / `Phone` / `Title` / `Bio` / `License number` / `Specializations` (Explore audit)
- `frontend/apps/reality-web/src/app/[locale]/realtor/analytics/page.tsx:68-69` — `<h1>Analytics</h1>` / `<p>Performance of your listings over time.</p>` (Explore audit)
- Six additional page files with the same pattern — enumerated under *Files* below; 46 total `page.tsx` under `[locale]`, 19 don't import `next-intl`, and this plan covers the 9 most-visible authenticated pages; the remaining ~10 are either legally-static content or lower-traffic and can be follow-up work

## Files
- `frontend/apps/reality-web/src/app/[locale]/account/password/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/account/profile/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/account/listings/[id]/edit/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/account/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/realtor/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/realtor/profile/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/realtor/analytics/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/realtor/listings/new/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/forbidden/page.tsx`
- `frontend/apps/reality-web/messages/en.json`
- `frontend/apps/reality-web/messages/sk.json`
- `frontend/apps/reality-web/messages/cs.json`
- `frontend/apps/reality-web/messages/de.json`

## Dependencies
<!-- The two already-in-review reality-web i18n PRs use `useTranslations` from next-intl the same way this plan will; they don't gate this work but landing them first eliminates any merge overlap in the message bundles. If they're still open when this plan is claimed, take origin/dev at claim time and follow whatever key shape they added. -->

## Required capabilities
- [x] C1 — Systematic debugging (bug vector; grep-driven inventory across 9 pages)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `cd frontend && pnpm dev:reality` (or `stack up pm-local` if the full stack is up).
2. Open the site with URL locale `/sk/account/password` (any of the 9 pages listed under *Files*).
3. Observe: the page heading, subtitle, form labels and any inline error/success chrome render in English (`Change password`, `Update the password used to sign in to your account.`, `Current password`, etc.) even though the URL declares locale `sk`. Expected: the same visible strings render in Slovak per the message bundle.
4. Confirm on `/cs/account/profile`, `/de/realtor/profile`, `/hu/realtor/analytics` etc. — same English chrome across all non-en locales.

## Suggested approach
1. Grep the 9 files for literal JSX text content: `grep -nE '>[A-Z][a-zA-Z ,.?!:]+<' frontend/apps/reality-web/src/app/\[locale\]/{account,realtor,forbidden}/**/*.tsx`. Enumerate every visible string per file into a checklist.
2. Choose a translation-key namespace per page group: `account.password.*`, `account.profile.*`, `realtor.profile.*`, `realtor.analytics.*`, `realtor.newListing.*`, `forbidden.*` (mirror the existing conventions used by `saved-searches` and `inquiries` bundles landed in #2894/#2895 — take the shape from `origin/dev` after those merge).
3. Add the new keys to `messages/en.json` first (source of truth), then propagate stubs to `sk.json`, `cs.json`, `de.json`, `hu.json`, `pl.json`. English text stays as the fallback if a translator hasn't filled a locale; the goal here is unblocking the `t()` call, not a full translator pass.
4. For each page, add `const t = useTranslations('account.password')` (client components) or `const t = await getTranslations('account.password')` (SSR components — pattern already in `saved-searches/page.tsx`). Replace every literal string with `{t('changePasswordTitle')}` etc.
5. Confirm `next-intl`'s `unstable_setRequestLocale` (or the app's canonical equivalent in `layout.tsx`) already runs for these routes; if not, add it — otherwise SSR pages will fall back to the default locale.
6. Run `pnpm -F reality-web check && pnpm -F reality-web typecheck && pnpm -F reality-web test` — biome + tsc + vitest, one-package scope.
7. Playwright smoke: `pnpm -F reality-web e2e -- --grep 'i18n|locale'` if any exists; otherwise cover the change with a Vitest render assertion that a mock `<NextIntlClientProvider>` with `sk` messages renders the Slovak string instead of the English one (IG3 test — must fail on `origin/dev` before the change).

## Alternatives considered
- **Per-page individual plans (9 separate plans)** — rejected because it fragments the same class of work across 9 dispatcher rounds, blows the 2-plan-per-run cap for weeks, and each individual page is too small (1–2 hours) to justify a separate PR review round. Bundling is cheaper end-to-end.
- **Skip the message-bundle keys and rely on `next-intl`'s default-locale fallback with English literals in-place** — rejected because it perpetuates the anti-pattern (future refactors would still greet non-en users with English) and produces no visible improvement for `sk` / `cs` / `de` users until a follow-up bundle-fill lands. The fix is only meaningful if the keys exist in every locale bundle.

## Root-cause trace
1. Symptom: `sk` / `cs` / `de` / `hu` / `pl` users see literal English chrome on `account/*` and `realtor/*` pages after signing in.
2. ← Each of the 9 `page.tsx` files renders JSX strings directly (`<h1>Change password</h1>` at `account/password/page.tsx:68`) instead of `{t('changePasswordTitle')}`.
3. ← These pages never call `useTranslations()` or `getTranslations()` — verified by the Explore audit: 19 of 46 `page.tsx` files under `[locale]` import neither hook.
4. Origin: pages were authored before the i18n conventions were settled — the reality-web scaffold uses next-intl (layout.tsx:3-4 imports `NextIntlClientProvider`), but the authenticated workspace pages predate the pattern and were never retrofitted. PRs #2894 and #2895 fixed the pattern on `saved-searches` and `inquiries` only; the remaining pages were never audited.

## Test plan
- [ ] Add a Vitest render test per major page (`account/password`, `account/profile`, `realtor/profile`, `realtor/analytics`) that renders inside a `NextIntlClientProvider` with a mocked Slovak messages object and asserts the Slovak string appears — asserts the English literal is NOT in the rendered output. This test must FAIL on `origin/dev` before the change (IG3).
- [ ] Regression: run the existing reality-web test suite (`pnpm -F reality-web test`) to confirm no other page's rendering broke due to shared component changes.
- [ ] Command to run locally: `pnpm -F reality-web test && pnpm -F reality-web typecheck && pnpm -F reality-web check`.

## Out of scope
- The ~10 other reality-web `page.tsx` files that don't import `next-intl` but are either mostly-static legal content (`terms`, `privacy`, `cookies`) or low-traffic (`sell`, `help`, `careers`). These are follow-up items; scoping this plan wider risks a mega-PR that never lands.
- Refactoring shared components (buttons, dialogs, form fields) to accept translated labels rather than raw strings — that's a bigger UI-library change and should be its own plan.
- Actual translations by a human translator into `sk` / `cs` / `de` / `hu` / `pl`. This plan lands the infrastructure (keys in every bundle, hooks wired in every page); real translation copy can follow.
- The two pages already covered by PRs #2894 (`saved-searches/page.tsx`) and #2895 (`inquiries/page.tsx`) — those land through the dispatcher's existing review flow.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-i18n-systemic-workspaces.md`
- Mark the matching `backlog.json` row as `status: "done"`
