# code-review-ppt-web-ui-marketplace-website-href-xss

**Vector:** security
**Score:** 2
**Source:** Phase 1.5 code-review-finding 2026-07-18 (ppt-web-ui segment), file `frontend/apps/ppt-web/src/features/marketplace/pages/ProviderDetailPage.tsx:241`
**Confidence:** high

## Hypothesis

`ProviderDetailPage.tsx:241` renders `<a href={provider.website} target="_blank" rel="noopener noreferrer">` on an untrusted, provider-supplied URL with no scheme allow-list. React does not block `javascript:` URLs in anchor `href` attributes (it emits a dev-only warning) and `rel="noopener noreferrer"` does not defend against script-scheme URIs. A hostile marketplace provider that stores `javascript:alert(document.cookie)` (or any script payload) as their `website` gets stored-XSS on every manager who opens their public profile — an authenticated context that carries the manager's session cookies and CSRF token. The smallest safe change is a single `safeExternalHref()` helper that returns the raw URL only when it parses to `http:` / `https:` / `mailto:` / `tel:`, and returns `undefined` (or the empty href sentinel) otherwise. Applying it at the two other `href={<untrusted>}` sites the marketplace exposes (portfolio images already flow through `<img src>`, so only the `website` field is affected today, but tighten the write-side too) closes the class of bug.

## Evidence

- `frontend/apps/ppt-web/src/features/marketplace/pages/ProviderDetailPage.tsx:236-247` — `{provider.website && ( <a href={provider.website} …`; interface at line 17 declares `website?: string` with no runtime guard.
- Grep across `frontend/packages/` and `frontend/apps/ppt-web/src/` for `safeUrl|sanitizeUrl|isSafeHref|allowedProtocols|isHttpsUrl` returns 0 hits — no existing helper.
- Grep across `frontend/apps/ppt-web/src/features/marketplace/` for `useTranslation` also returns 0 — the entire slice was landed without going through the review flow (Phase 1.5 also raised the i18n regression as a separate finding).
- React docs (developer error #815) — `javascript:` scheme in `href` fires only a dev-mode console warning; production builds pass it through to the DOM.
- No hostile-provider test coverage in the marketplace slice: `find features/marketplace -name '*.test.*'` returns 0.

## Files

- `frontend/apps/ppt-web/src/features/marketplace/pages/ProviderDetailPage.tsx`
- `frontend/apps/ppt-web/src/features/marketplace/pages/MarketplacePage.tsx`
- `frontend/apps/ppt-web/src/features/marketplace/pages/ProviderProfilePage.tsx`

## Dependencies

<!-- no upstream backlog dependencies — this fix is a self-contained frontend patch -->

## Required capabilities

- [x] C1 — Systematic debugging (bug / security fix)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion (must exercise the failing case)
- [ ] C7 — Code-review reception

**Execution mode:**

Mode: cloud-ok

(C4/C5 not required — the failing case is exercised via Vitest + JSDOM against the rendered anchor, no live browser needed.)

## Repro steps

1. Check out `dev` at HEAD, install: `pnpm install`.
2. Write a failing Vitest case in a new sibling `ProviderDetailPage.test.tsx`: render `<ProviderDetailPage provider={{ ..., website: 'javascript:alert(1)' }} />` (fill the other required props with test fixtures) and assert `screen.getByRole('link', { name: /website/i }).getAttribute('href')` equals `'#'` (or is absent), NOT the `javascript:` string.
3. Expected on `dev` today: the assertion fails because the anchor's `href` is `"javascript:alert(1)"` verbatim (React passed it through).
4. Run the test again after the fix — it passes. Add a parallel case for a normal `https://` URL to confirm the helper doesn't strip legitimate values.

## Suggested approach

1. Add `frontend/packages/shared/src/url/safeExternalHref.ts` exporting `safeExternalHref(raw: string | undefined): string | undefined`. Parse via `new URL(raw)` inside a try/catch (returns `undefined` on invalid input); accept protocols `http:`, `https:`, `mailto:`, `tel:` only; return `undefined` for anything else. Export it from `@ppt/shared` (`packages/shared/src/index.ts`).
2. Edit `ProviderDetailPage.tsx:241` — replace `href={provider.website}` with `href={safeExternalHref(provider.website)}`; wrap the whole `{provider.website && (...)}` in `{safeExternalHref(provider.website) && (...)}` so the block also disappears when the URL is disallowed. Update the label at line 246 to render the raw text if you still want to show what they typed, or omit the block entirely.
3. Repeat the pattern at any other `<a href={untrusted}>` site in `features/marketplace/`. If none exist today (confirm via `grep -rn 'href={' features/marketplace/`), still tighten `ProviderProfileForm.tsx`'s onSubmit to reject `javascript:` and other non-allowed schemes before persisting the write.
4. Write two Vitest cases in the new `ProviderDetailPage.test.tsx`: (a) hostile `javascript:` value → link is absent OR `href` normalised, and (b) valid `https://…` value → link renders unchanged.
5. Add one Vitest case in a new `safeExternalHref.test.ts` covering: `undefined`, empty string, `http://`, `https://`, `mailto:`, `tel:`, `javascript:`, `data:`, `vbscript:`, `//no-protocol`, and a malformed URL.
6. Run `pnpm --filter @ppt/web check && pnpm --filter @ppt/web typecheck && pnpm --filter @ppt/web test` locally.
7. Commit as `fix(marketplace): safe-scheme filter on provider website hrefs (closes findings from 2026-07-18 code review)`.

## Alternatives considered

- **Render `provider.website` as plain text with a "Copy link" button, never as an anchor** — rejected because it degrades UX for the 99 % legitimate providers and doesn't help future `<a href={x}>` sites elsewhere in the marketplace; the helper is the reusable fix.
- **Server-side scheme filter in `api-server` on the marketplace write path** — rejected as the *only* line of defence: the existing backend surface is not the only source of `provider.website` data (portfolio/image URLs already flow into the client from other endpoints), and defense-in-depth wants both. Server-side hardening is a good follow-up but is out of scope for this plan (call it out in `## Out of scope`).

## Root-cause trace

1. Symptom: `<a href="javascript:alert(1)">Provider website</a>` renders in the DOM when a hostile provider stores that string as their `website`; a manager clicking it executes attacker JS with the manager's session.
2. ← `ProviderDetailPage.tsx:241` passes `provider.website` unfiltered into the JSX `href` attribute.
3. ← The `website?: string` field on `ProviderDetailPageProps.provider` (line 17) is untyped — no branded `SafeUrl` type, no runtime guard.
4. ← React does not sanitize `javascript:` URLs; the DOM accepts them as valid `href` values.
5. Origin: the marketplace slice was landed as a large brand-new feature (Epic 68) without going through the Phase 1.5 code-review pass or the ppt-web review flow (the same subagent finding also raises the i18n regression and zero-test regression on the same files) — no gate caught the missing scheme filter at merge time.

## Test plan

- [ ] `frontend/apps/ppt-web/src/features/marketplace/pages/ProviderDetailPage.test.tsx` — hostile-scheme case (`javascript:` → link absent) and legit-scheme case (`https://…` → link renders unchanged).
- [ ] `frontend/packages/shared/src/url/safeExternalHref.test.ts` — unit table for `http:`, `https:`, `mailto:`, `tel:`, `javascript:`, `data:`, `vbscript:`, empty, malformed, `undefined`.
- [ ] Local command: `pnpm --filter @ppt/web test -- ProviderDetailPage` and `pnpm --filter @ppt/shared test -- safeExternalHref` both green.
- [ ] Optional Playwright / Chrome MCP check: load the provider-detail route, verify the DOM's `href` on the website anchor equals the fed value only when it starts with `http`/`https`; skip if the local dev stack is not available (cloud-ok mode).

## Out of scope

- Server-side scheme validation on the `api-server` marketplace write path (call out as follow-up).
- Localizing the marketplace slice — separate backlog item `code-review-ppt-web-ui-marketplace-i18n-regression`.
- Adding sibling `*.test.*` files across the whole marketplace + advanced-notifications slices — separate backlog item `code-review-ppt-web-ui-marketplace-notifications-untested`.
- Redesigning the marketplace layout / adding portfolio-image URL sanitisation.

## After-merge

- Move this file to `plans/_archive/code-review-ppt-web-ui-marketplace-website-href-xss.md`.
- Mark the matching `backlog.json` row (`code-review-ppt-web-ui-marketplace-website-href-xss`) as `status: "done"`.
- Open a follow-up issue against `api-server` marketplace endpoints to enforce server-side scheme allow-list on the provider-write path.
