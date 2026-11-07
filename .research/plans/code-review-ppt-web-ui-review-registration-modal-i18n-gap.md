# code-review-ppt-web-ui-review-registration-modal-i18n-gap

**Vector:** bug
**Score:** 2
**Source:** rotating-expert-review (ppt-web-ui segment, 2026-11-07) · precedent: PR #3355 (same pattern merged 24 h ago)
**Confidence:** high

## Hypothesis
`ReviewRegistrationModal.tsx` imports `useTranslation` (so the author intended to i18n it) but 6 user-facing strings are still hardcoded English in the modal shown to building managers approving/rejecting pet+vehicle registrations. Close the gap by routing those 6 strings through `t()` and adding the keys to all six locale bundles — exactly the shape of PR #3355 for `RegistryPage.tsx`.

## Evidence
- `frontend/apps/ppt-web/src/features/registry/components/ReviewRegistrationModal.tsx:77` — `<span className="...">Choose action:</span>` (section header, hardcoded).
- `ReviewRegistrationModal.tsx:90,111,136` — three SVG `<title>` elements hardcode `Approve`, `Reject`, `Info` (accessible labels for the action icons).
- `ReviewRegistrationModal.tsx:193` — `placeholder="Please provide a reason for rejecting this registration..."` on the rejection-reason textarea.
- `ReviewRegistrationModal.tsx:195` — `<p>This will be sent to the owner.</p>` (helper text under the textarea).
- `useTranslation` already imported at the top of the file — partial i18n coverage; same mixed-state bug that PR #3355 fixed on `RegistryPage.tsx`.

## Files
- `frontend/apps/ppt-web/src/features/registry/components/ReviewRegistrationModal.tsx`
- `frontend/apps/ppt-web/messages/en.json`
- `frontend/apps/ppt-web/messages/sk.json`
- `frontend/apps/ppt-web/messages/cs.json`
- `frontend/apps/ppt-web/messages/de.json`
- `frontend/apps/ppt-web/messages/pl.json`
- `frontend/apps/ppt-web/messages/hu.json`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok

## Repro steps
1. `cd frontend && pnpm --filter @ppt/web dev` — load the Property Management UI with locale switched to `sk` (or any of cs/de/pl/hu).
2. Navigate to a building's Registry > Pets list and click **Review** on a pending registration — the `ReviewRegistrationModal` opens.
3. Observe: `Choose action:` label (English), three icon tooltips (`Approve` / `Reject` / `Info`), textarea placeholder (`Please provide a reason for rejecting this registration...`), and helper (`This will be sent to the owner.`) all render in English on a Slovak/Czech/German/Polish/Hungarian build.
4. Expected: every string localised. Current: `grep -nE '>[A-Z][a-z][^<]*<|placeholder="[A-Z]' ReviewRegistrationModal.tsx` returns six matches for English literals.

## Suggested approach
1. Add `registry.review.modal.*` keys to all six locale bundles (`en, sk, cs, de, pl, hu`): `chooseAction`, `approve`, `reject`, `info`, `rejectReasonPlaceholder`, `sentToOwnerHelper`. English and Slovak hand-translated; the other four match English for the first pass (PR #3355 convention — reviewer can translate later).
2. Replace each hardcoded string with `t('registry.review.modal.<key>')`:
   - L77 `Choose action:` → `t('registry.review.modal.chooseAction')`
   - L90 `<title>Approve</title>` → `<title>{t('registry.review.modal.approve')}</title>`
   - L111 `<title>Reject</title>` → `<title>{t('registry.review.modal.reject')}</title>`
   - L136 `<title>Info</title>` → `<title>{t('registry.review.modal.info')}</title>`
   - L193 placeholder → `placeholder={t('registry.review.modal.rejectReasonPlaceholder')}`
   - L195 helper paragraph → `{t('registry.review.modal.sentToOwnerHelper')}`
3. Add a regression test `frontend/apps/ppt-web/src/i18n/reviewRegistrationModalI18n.test.ts` modelled on `registryPageI18n.test.ts` (PR #3355) — asserts the keys exist in every locale AND that the rendered modal no longer contains the English literals. Fails on `main`; passes with the fix.
4. Run the impact-scoped gate: `cd frontend && pnpm exec biome check …` + `pnpm --filter @ppt/web typecheck` + `pnpm --filter @ppt/web test` — assert `localeNamespaceParity` still green.

## Alternatives considered
- **Reuse `common.approve` / `common.reject` / `common.info` keys if they exist** — rejected unless a `common.approve` key is already in all six locale bundles (grep first); otherwise introducing top-level `common` fallbacks changes the i18n taxonomy. Prefer the `registry.review.modal.*` namespace for scope-locality consistent with PR #3355's `registry.page.*` grouping.
- **Move strings to a shared helper that returns i18n'd labels** — rejected because PR #3355 established the convention of inlining `t()` at the usage site; a helper diverges from the pattern and slows the reviewer.

## Root-cause trace
1. Symptom: A Slovak building manager opens the review modal; label, icon tooltips, textarea placeholder, and helper text are all English despite the rest of the UI being localised.
2. ← Immediate cause: `ReviewRegistrationModal.tsx:77,90,111,136,193,195` — 6 hardcoded English string literals, no `t()` wrapper despite `useTranslation` being imported at the top of the file.
3. ← Upstream cause: component was authored with `useTranslation` in scope but the author only translated a subset of surfaces before shipping — the same mixed-state bug the ppt-web i18n sweep landed-on-RegistryPage.tsx via PR #3355.
4. Origin: `frontend/apps/ppt-web/src/features/registry/components/ReviewRegistrationModal.tsx` initial commit under Epic 57 (building registries); the partial i18n import is the fossil.

## Test plan
- [ ] `frontend/apps/ppt-web/src/i18n/reviewRegistrationModalI18n.test.ts` — new file, asserts all `registry.review.modal.*` keys exist in every locale and rendering the modal in a locale-switched provider does not surface the six English literals (fails on `main`).
- [ ] `localeNamespaceParity.test.ts` — existing invariant must stay green.
- [ ] Local command: `cd frontend && pnpm --filter @ppt/web test -- --run src/i18n/reviewRegistrationModalI18n.test.ts src/i18n/localeNamespaceParity.test.ts`

## Out of scope
- Translating the four non-en/sk locales natively (PR #3355 shipped placeholder English for the same four; dedicated translator sweep tracked elsewhere).
- Rewriting the modal's layout, animation, or submit logic — only the six string literals change.
- A11y role/aria-* enhancements on the modal — outside the i18n gap scope.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-ui-review-registration-modal-i18n-gap.md`
- Mark the matching `backlog.json` row as `status: "done"`
