# code-review-reality-web-jsonld-xss

**Vector:** security
**Score:** 3
**Source:** Rotating expert review 2026-07-12 (reality-web segment) — signal id `code-review-reality-web-jsonld-xss`
**Confidence:** high

## Hypothesis

`ListingDetailContent.tsx:104` injects the RealEstateListing JSON-LD payload with `dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}` and NO `<`→`<` escape. `jsonLd.name` and `jsonLd.description` are copied verbatim from realtor-supplied `listing.title` / `listing.description` in `buildListingJsonLd()` (jsonLd.ts:45,58). A listing whose title or description contains `</script><img src=x onerror=...>` breaks out of the SEO `<script type="application/ld+json">` tag and executes for every public visitor of the SSR detail page — stored XSS reachable to any authenticated realtor who can create/edit listings. The sibling `app/env.js/route.ts:55` already applies `JSON.stringify(env).replace(/</g, '\\u003c')` for the exact same inline-JSON pattern, confirming the intended mitigation was simply omitted at the JSON-LD injection site.

## Evidence

- `frontend/apps/reality-web/src/components/listings/ListingDetailContent.tsx:104` — raw `JSON.stringify(jsonLd)` interpolated into `__html` with no escape.
- `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/jsonLd.ts:45,58` — `name: l.title` and `jsonLd.description = l.description` copied verbatim.
- `frontend/apps/reality-web/src/app/env.js/route.ts:55` — reference mitigation `JSON.stringify(env).replace(/</g, '\\u003c')` for a structurally identical inline-JSON case.
- SSR path: server rendered → executes in every public visitor's browser.

## Files

- `frontend/apps/reality-web/src/components/listings/ListingDetailContent.tsx:104`
- `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/jsonLd.ts`
- `frontend/apps/reality-web/src/app/env.js/route.ts:55`

## Dependencies

<none>

## Required capabilities

- [x] C1 — Systematic debugging (security fix; must not regress SEO markup)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps

1. As an authenticated realtor, create or edit a listing with title `Foo</script><img src=x onerror="window.__XSS__=1">`.
2. Publish the listing.
3. GET the public detail URL `/{locale}/listings/{slug}` (SSR).
4. Expected: JSON-LD renders literally; no script executes. Actual (today): the `</script>` byte-sequence closes the `<script type="application/ld+json">` tag; the injected `<img>` executes the `onerror` handler for every visitor of the page.

## Suggested approach

1. Add a shared helper `safeJsonLd(value: unknown): string` (co-locate with `jsonLd.ts` under `app/[locale]/listings/[slug]/`, or lift into `src/lib/`), that returns `JSON.stringify(value).replace(/</g, '\\u003c').replace(/>/g, '\\u003e').replace(/\//g, '\\u002f').replace(/ /g, '\\u2028').replace(/ /g, '\\u2029')`. This mirrors `env.js/route.ts:55` but is stricter (also handles `>`, `/`, and U+2028/U+2029 line separators that break some parsers).
2. Update `ListingDetailContent.tsx:104` to call the helper: `dangerouslySetInnerHTML={{ __html: safeJsonLd(jsonLd) }}`.
3. Audit for other `dangerouslySetInnerHTML` call sites in `frontend/apps/reality-web/src/` (`grep -rn dangerouslySetInnerHTML frontend/apps/reality-web/src`) — the follow-up finding `code-review-reality-web-tenant-bootstrap-xss` (layout.tsx:188 tenant bootstrap) is the other known site; it can be routed through the same helper in a follow-up plan (out of scope for this PR).
4. Refactor `env.js/route.ts:55` to also call the helper so all three inline-JSON call sites share one implementation.

## Alternatives considered

- **Client-side sanitisation with DOMPurify** — rejected because JSON-LD SSR renders before client hydration; DOMPurify runs in the browser and cannot prevent the initial parser attack.
- **Stripping `<`/`>` from listing title/description in the write path** — rejected because it discards legitimate text (French quotation marks, HTML entity references in descriptions) and does not defend against ` `/` ` parser attacks; escape-on-render is the correct layer.

## Root-cause trace

1. Symptom: stored XSS executes on every public visitor of `/{locale}/listings/{slug}` when a realtor's title/description contains `</script>`.
2. ← `ListingDetailContent.tsx:104`: raw `JSON.stringify(jsonLd)` sent to `__html`.
3. ← `jsonLd.ts:45,58`: `l.title` and `l.description` copied verbatim into `jsonLd.name` / `jsonLd.description` — the JSON-LD builder is not the right layer to escape (JSON-LD content is legitimate; the escape must happen at the HTML-boundary injection site).
4. Origin: the reality-web JSON-LD SEO feature (`ListingDetailContent.tsx`) was authored without cross-referencing the existing `env.js/route.ts:55` escape — the pattern was known and applied in one place but not systemised.

## Test plan

- [ ] `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/jsonLd.test.ts` (new) — unit test on the `safeJsonLd` helper: given `{ name: 'Foo</script><img src=x>' }`, asserts the returned string contains `</script>` and does NOT contain `</script>`.
- [ ] `frontend/apps/reality-web/src/components/listings/ListingDetailContent.test.tsx` (extend if present, otherwise new) — assert the rendered `<script type="application/ld+json">` for a listing with `title: 'X</script>Y'` contains `</script>` and does not close the tag prematurely.
- [ ] Regression command: `pnpm -F @ppt/reality-web test -- jsonLd` (Vitest under `frontend/`; the `-F` flag picks the reality-web workspace).

## Out of scope

- The `tenant-bootstrap-xss` finding (`layout.tsx:188`) is a sibling XSS with a different payload surface (tenant admin, not public realtor). Track it in a separate plan.
- The `killswitch-i18n` finding (`layout.tsx:138,151,152`) is a translation gap, unrelated.
- Audit of `dangerouslySetInnerHTML` beyond the three known sites (`ListingDetailContent`, `layout.tsx`, `env.js/route.ts`) is a broader hardening task.

## After-merge

- Move this file to `plans/_archive/code-review-reality-web-jsonld-xss.md`
- Mark the matching `backlog.json` row as `status: "done"`
