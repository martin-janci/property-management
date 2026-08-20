# code-review-ppt-web-ui-moderation-prompt-cast

**Vector:** bug
**Score:** 4
**Source:** rotating-expert-review (ppt-web-ui, 2026-08-19 + 2026-08-20) — signals `code-review-ppt-web-ui-moderation-prompt-cast-unvalidated`, `code-review-ppt-web-ui-moderation-prompt-cast`
**Confidence:** high

## Hypothesis
`ContentModerationPage` sends destructive moderation actions and DSA-appeal decisions using a raw `window.prompt(...)` string cast (`as 'remove'|'restrict'|'warn'|'approve'` / `as 'uphold'|'overturn'`) with no membership check before `.mutate(...)`. A user typo — or a UI text change on a shipped page — mutates the union into an invalid `action_type` that the API rejects with a generic 400 (bad UX) or, worse, that a laxer server-side accepts and forwards to the moderation queue. Add a client-side allowlist check that returns early on any non-member value, and back it with a component test that today fails when the user submits garbage.

## Evidence
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx:139` — `handleTakeAction` builds the mutate payload with `action_type: actionType as 'remove' | 'restrict' | 'warn' | 'approve'` after only `if (!actionType) return;`.
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx:175` — `handleDecideAppeal` builds `decision: decision as 'uphold' | 'overturn'` after only `if (!decision) return;`.
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx:127-128` and `:163-164` — both handlers carry a `TODO(Phase-2)` comment noting these are shipped stubs.
- Independently verified against the current tree on 2026-08-20 (the sibling `-unvalidated` signal from 2026-08-19 and the reworded 2026-08-20 signal both describe the same live pattern).
- The i18n half of the original tier1d claim is NOT valid — prompt/alert strings on this page already go through `t()` (lines 129, 132, 147, 165, 168, 182); only the missing-validation half is the real defect.

## Files
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Serve `ppt-web` locally, sign in as an org owner, open `/compliance/moderation`.
2. Click *Take Action* on any pending item. When the prompt appears, type `deletee` (typo) and confirm the second prompt.
3. Expected: the client refuses the submit with a translated inline error and does not fire the moderation API request. Actual today: the client fires `POST /api/v1/compliance/moderation/:id/take-action` with `action_type: "deletee"`, the API returns a generic 400 rejection with no field-level context, and the user is left staring at the alert-based generic failure banner.
4. Same repro on *Appeal → Decide*: entering anything other than `uphold` / `overturn` sends the raw string.

## Suggested approach
1. Inline the allowed value sets in `ContentModerationPage.tsx` as `const ACTION_TYPES = ['remove','restrict','warn','approve'] as const;` and `const APPEAL_DECISIONS = ['uphold','overturn'] as const;` typed via `type ActionType = typeof ACTION_TYPES[number];`.
2. In `handleTakeAction` (lines 125-153), after the truthiness check but before the `.mutate(...)` call, guard with `if (!ACTION_TYPES.includes(actionType as ActionType)) { alert(t('compliance.moderation.invalidActionType')); return; }`. Drop the raw `as` cast — the guard proves the type.
3. Do the same in `handleDecideAppeal` (lines 161-188) with `APPEAL_DECISIONS`.
4. Add the two new i18n keys (`compliance.moderation.invalidActionType`, `compliance.moderation.invalidAppealDecision`) to `frontend/apps/ppt-web/src/locales/en/translation.json` and mirror stubs in `sk/cs/de` (English fallback string is acceptable — the copy team owns real translations).
5. Delete the `TODO(Phase-2)` comments at lines 127-128 and 163-164 — they were tied to this exact gap.
6. Because both handlers only use `window.prompt`, do not attempt to refactor to a real form here — that is a separate UI plan.

## Alternatives considered
- **Server-side allowlist only** — rejected because the current API already rejects unknown values, but users still see a generic 400 and a bad UX; the point of the client guard is the deterministic in-page error and cost-free typo protection on a destructive action.
- **Replace `window.prompt` with a real form/dialog** — rejected because a proper modal is a much larger UX + i18n change that this bug-scope plan should not carry; the guard is a one-file surgical fix that unblocks the safety issue today.

## Root-cause trace
1. Symptom: user typing `deletee` at the moderation prompt sends `action_type: "deletee"` to the API and shows a generic failure alert.
2. ← Immediate cause at `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx:139` — the `as 'remove' | 'restrict' | 'warn' | 'approve'` cast is a compile-time-only assertion and does no runtime check.
3. ← Upstream cause at the same file lines 127-128 — the `TODO(Phase-2)` comment records that the "real form" was deferred and the prompt-based path was shipped as a stub that never got the promised validation.
4. Origin: the page was shipped as a stub during the compliance/DSA feature build-out; the exact PR is not needed to fix the surface bug, and the fix does not backfill missing history.

## Test plan
- [ ] `frontend/apps/ppt-web/src/features/compliance/pages/__tests__/ContentModerationPage.test.tsx` (new) — RTL: mock `window.prompt` to return `"deletee"`, click *Take Action*, assert the mutation mock was NOT called and the invalid-action i18n key is rendered.
- [ ] Same file — parallel test for `handleDecideAppeal` with a mocked prompt returning `"maybe"`; assert `decideAppeal.mutate` was not called.
- [ ] Regression assert that valid values (`remove` for takeAction, `uphold` for decideAppeal) still call `mutate` with the correct payload shape.
- [ ] Command: `pnpm -F @ppt/web test -- ContentModerationPage`.

## Out of scope
- Replacing `window.prompt` with a real modal/form (larger UX effort).
- Real translations for the two new i18n keys (English stubs only; localization team handles the copy).
- Server-side validation changes (already rejects unknown values).
- Any other stub-comment cleanup on the same page.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-ui-moderation-prompt-cast.md`
- Mark the matching `backlog.json` row as `status: "done"`
