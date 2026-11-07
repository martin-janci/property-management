# code-review-ppt-web-ui-registry-rules-form-no-i18n

**Vector:** bug
**Score:** 2
**Source:** rotating-expert-review (ppt-web-ui segment, 2026-11-07) · precedent: PR #3351 + PR #3355 (same pattern merged 24 h ago)
**Confidence:** high

## Hypothesis
`RegistryRulesForm.tsx` has zero `useTranslation` imports and hardcodes 8 user-facing English strings (section headings, placeholders, labels). The enclosing `RegistryRulesPage.tsx` was just i18n-wired in PR #3351/#3355, but the child form it renders was missed — so building managers on sk/cs/de/pl/hu locales see an English form inside an otherwise-translated page. Wire the component through `useTranslation` and add keys to all six locale bundles, following the exact pattern the sibling `PetRegistrationForm.tsx` / `VehicleRegistrationForm.tsx` just landed with — mechanical refactor, no behavioural change.

## Evidence
- `frontend/apps/ppt-web/src/features/registry/components/RegistryRulesForm.tsx` — `grep -c "useTranslation\|react-i18next"` returns 0; 8 hardcoded strings at L78 ("Pet Rules"), L134/L228/L278 ("No limit"), L163 ("Banned breeds"), L176 ("Enter breed name and press Enter"), L238 ("Vehicle Rules"), L295 ("Any additional registry rules or notes...").
- Parent page is i18n-wired: `frontend/apps/ppt-web/src/features/registry/pages/RegistryRulesPage.tsx` uses `useTranslation` + `t()` for every chrome string after PR #3351 landed.
- Pattern precedent PR #3351 (merged 2026-11-07) added `registry.pet.*` / `registry.vehicle.*` keys to all six locale bundles (`en, sk, cs, de, pl, hu`) for the sibling form components — same test harness, same key-naming convention.
- Existing invariant `frontend/apps/ppt-web/src/i18n/localeNamespaceParity.test.ts` ensures every new namespace exists in all six bundles.

## Files
- `frontend/apps/ppt-web/src/features/registry/components/RegistryRulesForm.tsx`
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
2. Navigate to a building's Registry > Rules screen (`/buildings/:id/registry/rules`).
3. Observe: section headings render as "Pet Rules" / "Vehicle Rules", placeholder reads "No limit" / "Enter breed name and press Enter", form label reads "Banned breeds" — all English on a Slovak/Czech/German/Polish/Hungarian build. Expected: localised strings for every surface. Current: `grep -c useTranslation frontend/apps/ppt-web/src/features/registry/components/RegistryRulesForm.tsx` is 0.

## Suggested approach
1. Add `import { useTranslation } from 'react-i18next';` and `const { t } = useTranslation();` to `RegistryRulesForm.tsx` (match the PR #3351 pattern in `PetRegistrationForm.tsx`).
2. Replace each hardcoded string with `t('registry.rules.form.<key>')`:
   - L78 "Pet Rules" → `t('registry.rules.form.petRulesHeading')`
   - L134/L228/L278 "No limit" → `t('registry.rules.form.noLimitPlaceholder')`
   - L163 "Banned breeds" → `t('registry.rules.form.bannedBreedsLabel')`
   - L176 "Enter breed name and press Enter" → `t('registry.rules.form.bannedBreedsPlaceholder')`
   - L238 "Vehicle Rules" → `t('registry.rules.form.vehicleRulesHeading')`
   - L295 "Any additional registry rules or notes..." → `t('registry.rules.form.notesPlaceholder')`
3. Add the `registry.rules.form.*` subtree to all six locale bundles (`en, sk, cs, de, pl, hu`). English and Slovak hand-translated; the other four match English for first pass (follow the #3351 convention — reviewer can translate later).
4. Add a regression test `frontend/apps/ppt-web/src/i18n/registryRulesFormI18n.test.ts` modelled on the existing `registryPageI18n.test.ts` (#3355) — asserts the keys exist in every locale AND that the rendered component no longer contains the English literals. Fails on `main`; passes with the fix.
5. Run the impact-scoped gate: `cd frontend && pnpm exec biome check …` + `pnpm --filter @ppt/web typecheck` + `pnpm --filter @ppt/web test` — assert `localeNamespaceParity` still green.

## Alternatives considered
- **Leave as-is (defer until a translator sweeps the UI)** — rejected because (a) building managers are already using the page in production on non-English locales per PR #3351's rationale, and (b) the `localeNamespaceParity` harness will catch any future drift, so adding keys now costs nothing.
- **Centralise registry rules copy in a shared hook/helper** — rejected because PR #3351 established the convention of `t('registry.<area>.*')` inline at the usage site; introducing a wrapper diverges from the pattern and slows the reviewer.

## Root-cause trace
1. Symptom: Slovak/Czech/German/Polish/Hungarian building managers see English section headings and placeholders inside a locale-switched Registry Rules page.
2. ← Immediate cause: `RegistryRulesForm.tsx:78,134,163,176,228,238,278,295` — string literals are hardcoded English, no `t()` wrapper.
3. ← Upstream cause: Epic 57 (building registry) scaffolded the form component before the ppt-web i18n sweep reached `features/registry/`; subsequent PRs (#3351/#3355) covered the page chrome + sibling forms but missed the rules form.
4. Origin: `frontend/apps/ppt-web/src/features/registry/components/RegistryRulesForm.tsx` initial commit under Epic 57 (building registries rollout).

## Test plan
- [ ] `frontend/apps/ppt-web/src/i18n/registryRulesFormI18n.test.ts` — new file, asserts all `registry.rules.form.*` keys exist in every locale and that rendering `<RegistryRulesForm />` in a locale-switched provider does not surface English literals (fails on `main`).
- [ ] `localeNamespaceParity.test.ts` — existing invariant must stay green (new keys exist in all six bundles).
- [ ] Local command: `cd frontend && pnpm --filter @ppt/web test -- --run src/i18n/registryRulesFormI18n.test.ts src/i18n/localeNamespaceParity.test.ts`

## Out of scope
- Translating the four non-en/sk locales to native speakers' standard (PR #3351 shipped placeholder English for the same four; the dedicated translator sweep is tracked separately).
- Rewriting `RegistryRulesForm.tsx` state model, validation, or submit flow — only the string literals change.
- Rules-form a11y (role, aria-labelledby) — outside the i18n gap scope.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-ui-registry-rules-form-no-i18n.md`
- Mark the matching `backlog.json` row as `status: "done"`
