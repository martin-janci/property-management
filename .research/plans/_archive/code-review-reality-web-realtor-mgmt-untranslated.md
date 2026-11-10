# code-review-reality-web-realtor-mgmt-untranslated

**Vector:** bug
**Score:** 3
**Source:** Issue None (Phase 1.5 rotating expert review of `reality-web` on 2026-06-14)
**Confidence:** high

## Hypothesis
Reality Portal ships in sk/cs/de/hu/pl/en, but `RealtorManagement.tsx` hardcodes English chrome for the entire agency invite/manage flow (modal title, form labels, button labels, placeholder strings, loading state). Non-EN visitors see English mid-form on a screen the rest of which is correctly translated. The component already imports `useTranslations("agency")` and uses `t("inviteError")` — the i18n pattern is in place; the literals were missed during the agency-flow build. Wire each visible string through `t()` and add the missing keys to all 6 message bundles.

## Evidence
- `frontend/apps/reality-web/src/components/agency/RealtorManagement.tsx:569` — `<h2 id="invite-modal-title">Invite Realtor</h2>` (hardcoded title)
- `RealtorManagement.tsx:593,605,617,628` — form labels `Email *`, `Full Name *`, `Job Title`, `Personal Message` hardcoded
- `RealtorManagement.tsx:640-643` — button literals `Cancel`, `Sending...`, `Send Invitation` hardcoded
- `RealtorManagement.tsx:599-601, 611, 622, 633` — placeholder strings `realtor@example.com`, `John Doe`, `Senior Real Estate Agent`, `Add a personal message to the invitation...` hardcoded
- `frontend/apps/reality-web/messages/en.json` (`agency.inviteRealtor` + `agency.inviteError` already present) — the namespace is set up; new keys just need adding to en/sk/cs/de/hu/pl

## Files
- `frontend/apps/reality-web/src/components/agency/RealtorManagement.tsx:569`
- `frontend/apps/reality-web/messages/en.json`
- `frontend/apps/reality-web/messages/sk.json`
- `frontend/apps/reality-web/messages/cs.json`
- `frontend/apps/reality-web/messages/de.json`
- `frontend/apps/reality-web/messages/hu.json`
- `frontend/apps/reality-web/messages/pl.json`

## Dependencies

(none)

## Required capabilities
- [ ] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `cd frontend && APP_LANG=sk pnpm -F reality-web dev` (or set a `sk` accept-language in the browser).
2. Sign in as an agency owner, open the realtor-management screen, click **Invite Realtor**.
3. Expected: modal title, all 4 form labels, all 4 placeholders, the cancel button, and the submit button (incl. its loading state) render in Slovak.
4. Actual (today): every one of those strings renders in English even though the rest of the page is in Slovak.

## Suggested approach
1. In `RealtorManagement.tsx` keep the existing `const t = useTranslations("agency")` reference (it already covers `inviteRealtor`/`inviteError`). Replace each hardcoded string with a `t(...)` call: `t("inviteRealtor")` for the modal title, then add 11 new keys — `formLabelEmail`, `formLabelName`, `formLabelTitle`, `formLabelMessage`, `formPlaceholderEmail`, `formPlaceholderName`, `formPlaceholderTitle`, `formPlaceholderMessage`, `buttonCancel`, `buttonSending`, `buttonSendInvitation`.
2. Add the 11 keys to `messages/en.json` under `"agency"` with the current English copy as the source of truth.
3. Replicate the same 11 keys into `sk.json`, `cs.json`, `de.json`, `hu.json`, `pl.json` with locale-appropriate translations. The existing `agency.inviteRealtor` translations in those files are a good style reference for tone (formal/informal).
4. Re-check the screen via `pnpm -F reality-web check` (Biome) and `pnpm -F reality-web typecheck`.
5. Skim the rest of `RealtorManagement.tsx` (the list / pending-invite / role-edit modals at the top of the file) for any other hardcoded strings the same review missed — if any are obvious, fix them in the same PR; otherwise leave them for a follow-up (note in PR body).
6. Verify the JSON files stay valid and sorted in the same order as the rest of the bundle (this repo's convention is alphabetical inside each namespace).

## Alternatives considered
- **Move the modal into its own component and wrap with `<IntlProvider>`** — rejected because the parent already provides the locale context via `next-intl`; a separate provider would double-wrap and break the existing `t("inviteError")` call.
- **Defer until a full agency-flow audit** — rejected because the gap is one component, narrowly scoped (5 lines worth of evidence), and shipping non-EN users a half-English form is a visible regression today; an audit can still cover the rest of the surface later.

## Root-cause trace
1. Symptom: Slovak/Czech/German/Hungarian/Polish users see English chrome inside the **Invite Realtor** modal while the surrounding page renders in their locale.
2. ← Immediate cause at `RealtorManagement.tsx:569,593,605,617,628,640-643` — JSX nodes contain English string literals instead of `t(...)` calls.
3. ← Upstream cause: when the modal was added to the component, the author wired the *error* string through `t("inviteError")` (line ~588) but missed the modal title, labels, placeholders, and button copy. The i18n pattern was set up correctly; the literals were a copy-paste oversight.
4. Origin: agency invite-modal addition (PR pending — `git log -L 569,650:frontend/apps/reality-web/src/components/agency/RealtorManagement.tsx` will name the commit; the rotating reviewer's 2026-06-14 reality-web pass surfaced it).

## Test plan
- [ ] Integration test: render `<RealtorManagement>` with each locale (en/sk/cs/de/hu/pl) through next-intl's test provider and assert the modal title, every form label, and both action buttons match the locale's `agency.formLabel*` / `agency.button*` keys, NOT raw English strings (the failing-on-main case).
- [ ] Snapshot the rendered modal for `sk` to ensure no stray English literal slipped through.
- [ ] Command: `cd frontend && pnpm -F reality-web test -- RealtorManagement` (the existing testing harness — `next-intl/test` provider patterns are already used by the `Header.test.tsx` suite in the same app).

## Out of scope
- Translating other components in the agency flow (`RealtorList`, `BrandingEditor`, etc.) — separate review, separate PR.
- Adding new locales (hu/pl already exist).
- Backend wording / email-template translations (server-rendered notification copy lives in a different repo path).
- Refactoring the modal into a reusable `Modal` shell.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-realtor-mgmt-untranslated.md`.
- Mark the matching `backlog.json` row as `status: "done"`.
