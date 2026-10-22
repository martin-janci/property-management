# code-review-reality-web-agency-href-scheme-xss

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review 2026-10-22 reality-web (dispatcher Tier-1d); diff verified in routine 2026-10-22
**Confidence:** high

## Hypothesis
Reality-web's public agency profile and the shared listing Resources section render raw agency/listing URLs as `<a href={...}>` with no scheme validation. If an agency or listing owner stores `javascript:alert(document.cookie)` (or any `javascript:` / `data:text/html` URL) in `website`, `virtualTourUrl`, or `floorPlanUrl`, clicking the link executes attacker JS in the reality-web origin — classic persisted-stored href-scheme XSS. `rel="noreferrer"` and `target="_blank"` do not block `javascript:` URLs. The smallest change is to route every tenant/user-authored href through an allowlist that keeps only `http:` / `https:` (and optionally `mailto:` / `tel:` where used), drop the link otherwise. The project already has `isSafeImageUrl` in `frontend/packages/shared/src/validation.ts:188-200` with the exact same allowlist semantics — generalise it to `isSafeHref` and call it at each of the three unsafe call sites.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/agency/[slug]/page.tsx:297` renders `<a href={agency.website} target="_blank" rel="noreferrer">🌐 Navštíviť web</a>` — the agency-authored `website` string is pasted into `href=` with no `new URL(...)` parse, no scheme check, no sanitiser.
- `frontend/apps/reality-web/src/components/listings/sections/Resources.tsx:17` renders `<a href={listing.virtualTourUrl} target="_blank" rel="noopener noreferrer">` — same raw-href class on the public listing detail page.
- `frontend/apps/reality-web/src/components/listings/sections/Resources.tsx:39` renders `<a href={listing.floorPlanUrl} target="_blank" rel="noopener noreferrer">` — identical pattern.
- `frontend/packages/shared/src/validation.ts:188-200` already defines `isSafeImageUrl` with the http/https allowlist and a comment explicitly flagging `javascript:` / `data:` / `blob:` / `file:` / `vbscript:` as the attack surface — the fix is to lift its logic to a sibling `isSafeHref` and call it from the agency page + Resources.
- The project's CSS-injection sanitiser (`frontend/apps/reality-web/src/lib/tenant-config.ts:243-279` `sanitizeCssValue`) proves the pattern is already understood in reality-web — the agency template just missed the href path.

## Files
- `frontend/apps/reality-web/src/app/[locale]/agency/[slug]/page.tsx`
- `frontend/apps/reality-web/src/components/listings/sections/Resources.tsx`
- `frontend/packages/shared/src/validation.ts`

## Dependencies
<!-- No stem collisions with current plans/ or active assignments (verified 2026-10-22 against assignments.json + action-list.json). -->

## Required capabilities
- [x] C1 — Systematic debugging (security vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security change — expect scrutiny)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Seed an agency row with `website = "javascript:alert(document.cookie)"` directly in the DB (bypasses any form-level validation — the fix must defend at the render boundary too, because the field accepts anything up to the DB column cap).
2. Visit `/sk/agency/<slug>` in reality-web.
3. Click the "🌐 Navštíviť web" link.
4. Expected: the browser treats the link as a dead/unsafe URL (preferred: the anchor is suppressed or routed through a wrapper that renders plain text when the href fails the scheme allowlist); no script execution.
5. Actual: an `alert()` fires with the current document cookies — code execution in the reality-web origin via a user-authored string. The same repro applies for `listing.virtualTourUrl` and `listing.floorPlanUrl` on a listing detail page.

## Suggested approach
1. Add `isSafeHref(url: string | null | undefined): boolean` to `frontend/packages/shared/src/validation.ts`, next to `isSafeImageUrl`. Same allowlist (`http:` / `https:`), same reject on everything else, same root-relative carve-out. Export from `frontend/packages/shared/src/index.ts` alongside `validateUrl` / `isSafeImageUrl`.
2. In `frontend/apps/reality-web/src/app/[locale]/agency/[slug]/page.tsx:295-304`, replace the raw-href anchor with a wrapper that renders the link only when `isSafeHref(agency.website)` is true; otherwise render the text as a non-interactive span (or drop it entirely). Keep the `target="_blank" rel="noreferrer noopener"` on the safe branch.
3. In `frontend/apps/reality-web/src/components/listings/sections/Resources.tsx:14-45`, apply the same guard to both `listing.virtualTourUrl` and `listing.floorPlanUrl`. If neither URL passes the check, suppress the entire resources section header too (keeps the UI consistent with "nothing to show").
4. Add `rel="noopener noreferrer"` on the agency page anchor to match Resources — the current `rel="noreferrer"` is weaker than the sibling section.
5. Add unit tests under `frontend/packages/shared/src/__tests__/` covering the new `isSafeHref` with the classic attack vectors (`javascript:`, `JavaScript:`, `\tjavascript:...`, `data:text/html,...`, `vbscript:`, `blob:...`, `file:...`, `//evil.example`) plus the positive cases.
6. Add a reality-web component test (Vitest + Testing Library) that renders the agency page with `agency.website="javascript:alert(1)"` and asserts the anchor is not rendered.
7. If any other reality-web template still renders user URLs without the guard (search for `href={` on tenant-authored strings), bring it into the same helper — do not expand scope beyond the three cited call sites unless the grep hits an obvious sibling pattern.

## Alternatives considered
- **DOMPurify or a sanitize-url npm dep** — rejected because the attack surface is a single allowlist decision (`http`/`https` yes, everything else no); the project already has the exact helper shape inside `frontend/packages/shared/src/validation.ts`, so adding a transitive dep for one three-line function trades a maintained local check for a supply-chain footprint reality-web does not otherwise carry.
- **Validate on form submit instead of at render** — rejected because the DB column accepts anything up to its varchar cap; existing agency rows may already contain unsafe URLs, and admin tools or imports bypass the form. Rendering is the one choke point every path hits, so the guard belongs there.

## Root-cause trace
1. Symptom: `javascript:` URL stored in `agency.website` executes JS when a visitor clicks the link, in the reality-web origin → stored XSS → session cookie theft, CSRF-token exfiltration, phishing redirect.
2. ← Immediate cause at `frontend/apps/reality-web/src/app/[locale]/agency/[slug]/page.tsx:297`: `href={agency.website}` with no `new URL(...)` scheme check and no `isSafeHref` wrapper.
3. ← Upstream cause at `frontend/apps/reality-web/src/components/listings/sections/Resources.tsx:17,39`: the same pattern was already in place when the agency page was added — the agency template copied the Resources shape without the guard that `isSafeImageUrl` already supplies for `<img src>` elsewhere in the same package.
4. Origin: the agency profile page landed as "Mirrors the realtor/[id] public-profile pattern" (header comment at `page.tsx:7-9`) — the realtor profile does not expose a tenant-authored URL field, so the author missed that the agency version introduces one; and reality-web's Resources section pre-dates `isSafeImageUrl` being written, so the href guard never got applied retroactively.

## Test plan
- [ ] New unit tests `frontend/packages/shared/src/__tests__/validation.test.ts` asserting `isSafeHref` rejects every classic unsafe scheme (`javascript:`, mixed-case, leading whitespace, `data:text/html`, `vbscript:`, `blob:`, `file:`, protocol-relative `//evil`) and accepts `http://`, `https://`, and root-relative `/path`.
- [ ] New component test `frontend/apps/reality-web/src/app/[locale]/agency/[slug]/__tests__/page.test.tsx` (or an existing sibling test file) that renders `<AgencyPage agency={{ website: "javascript:alert(1)", ... }} />` and asserts `screen.queryByRole('link', { name: /Navštíviť web/i })` returns null; a parallel test with `website: "https://example.com"` asserts the link IS rendered with the correct href.
- [ ] Run: `cd frontend && pnpm --filter @ppt/shared test` (new helper), `pnpm --filter @ppt/reality-web test` (component), and `pnpm --filter @ppt/reality-web typecheck`.
- [ ] Spot-check `pnpm biome check frontend/apps/reality-web/src/app/[locale]/agency` to make sure the diff is lint-clean.

## Out of scope
- Form-level URL validation on the agency/listing edit flows — a defence-in-depth improvement but a separate concern; this plan fixes the render boundary only.
- A broader audit of every `<a href={userValue}>` in reality-web outside the three cited call sites — do a focused `grep -rn "href={" frontend/apps/reality-web/src --include=*.tsx` as a sanity check during review and surface a follow-up issue if other sites exist; do not fold them into this PR.
- The sibling CSS-injection path on `agency.primaryColor` — tracked separately as `code-review-reality-web-agency-gradient-color-unsanitized`.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-agency-href-scheme-xss.md`
- Mark the matching `backlog.json` row as `status: "done"`
