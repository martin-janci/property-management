# code-review-reality-web-share-comparison-404

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review reality-web 2026-06-14
**Confidence:** high

## Hypothesis

`ComparisonUrlHandler` (the entry point for "Share Comparison" links, Epic 51 Story 51.3) fetches each shared listing from `/api/listings/${id}`. No such Next.js API route exists in reality-web — `src/app/api/` only ships a `health/route.ts`. Every shared comparison URL therefore 404s, the catch swallows it as `t('loadError')`, and the user sees an empty comparison page instead of the listings the sharer intended. The fix is one line: route the fetch through the canonical reality-server path (`${getApiBase()}/api/v1/listings/${id}`) just like every other reality-web caller.

## Evidence

- `frontend/apps/reality-web/src/components/comparison/ComparisonUrlHandler.tsx:38` — `const response = await fetch(\`/api/listings/${id}\`);`
- `frontend/apps/reality-web/src/app/api/` — directory contains only `health/route.ts`; no `listings/` subroute exists
- `frontend/apps/reality-web/src/lib/env.ts:60-67` — canonical pattern documented: callers do `${getApiBase()}/api/v1/...`
- `frontend/apps/reality-web/src/lib/auth-api.ts:35` + many siblings — every other lib/* api caller uses `${getApiBase()}${path}` with `/api/v1/...` prefix
- `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/page.tsx:71` — reality-server listing endpoint is `/api/v1/listings/${slug}` (SSR side)

## Files

- `frontend/apps/reality-web/src/components/comparison/ComparisonUrlHandler.tsx`
- `frontend/apps/reality-web/src/lib/env.ts`

## Dependencies

## Required capabilities

- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps

1. Open the dev/staging Reality Portal in a browser.
2. Navigate to a `/<locale>/compare?ids=<id1>,<id2>` URL (the shape `ComparisonUrlHandler` parses `sharedIds` from).
3. Open devtools Network panel.
4. Expected: each of the up-to-4 listings resolves via a 200 to `${apiBase}/api/v1/listings/<id>` and the comparison page renders the listings.
5. Actual: each request 404s against the Next.js app (no `/api/listings/<id>` route exists), the page swallows the error and renders `t('loadError')`.

## Suggested approach

1. Read `frontend/apps/reality-web/src/lib/env.ts:68-` to confirm `getApiBase()` import path; it is the canonical pattern used by every other client-side fetcher in this app.
2. In `ComparisonUrlHandler.tsx`, import `getApiBase` from `'../../lib/env'` (sibling of `lib/auth-api.ts`).
3. Replace the bare `fetch(\`/api/listings/${id}\`)` at line 38 with `fetch(\`${getApiBase()}/api/v1/listings/${id}\`)`.
4. Tighten the response handling: keep the existing `if (!response.ok)` 404 branch but log the response status on the catch so future shape regressions surface in observability rather than silently rendering `t('loadError')`.
5. Add a Vitest unit test `ComparisonUrlHandler.test.tsx` that mocks `fetch` and asserts the URL pattern includes `/api/v1/listings/` (the regression bar — the wrong URL silently 404'd).

## Alternatives considered

- **Create a Next.js `/api/listings/[id]/route.ts` that proxies to reality-server** — rejected because reality-server is already directly reachable from the client (or via the Next.js rewrite when running on a worktree host); adding a proxy route doubles the network hops and creates a new layer to keep in sync with the upstream schema. The canonical pattern is `getApiBase()` for this exact reason.
- **Switch to the generated `@ppt/reality-api-client` `getListingById` SDK call** — would be cleaner long-term but the SDK only exposes `ListingDetail` and `ComparisonUrlHandler` deserializes into `ListingSummary`; using the wrong shape would require either a new SDK method or a manual conversion. Out of scope for this bug fix.

## Root-cause trace

1. Symptom: Story 51.3 "Share Comparison" always lands on the error state (`t('loadError')`); shared listings never appear.
2. ← `ComparisonUrlHandler.tsx:53-55` — `try` block throws on the `!response.ok` 404 branch; `catch` sets `error` to the i18n key and `console.error`s.
3. ← `ComparisonUrlHandler.tsx:38` — `fetch(\`/api/listings/${id}\`)` requests a Next.js route that doesn't exist.
4. ← `src/app/api/` directory only contains `health/route.ts` (no `listings/` subroute, no `[id]/` dynamic).
5. Origin: the file was added under Epic 51 Story 51.3 ("Share Comparison"); the author likely assumed a `/api/listings/` proxy existed (it doesn't) and the missing route was never caught because the catch swallows the 404 as a translated error message.

## Test plan

- [ ] `frontend/apps/reality-web/src/components/comparison/ComparisonUrlHandler.test.tsx` — mock `global.fetch` to record the called URL; render with `sharedIds=['abc']`; assert `fetch.mock.calls[0][0]` matches `/\/api\/v1\/listings\/abc$/`. Would fail today (URL is `/api/listings/abc`).
- [ ] Mock 200 path returns a `ListingSummary` body; assert `addToComparison` is called with it.
- [ ] Mock 404 path; assert `error` state is set to `t('loadError')` (regression of the existing behavior on a real upstream miss).
- [ ] Local: `pnpm --filter @ppt/reality-web test -- ComparisonUrlHandler`.

## Out of scope

- Migrating the call to the generated `@ppt/reality-api-client` SDK (covered by alternative #2).
- Touching the i18n `comparison.loadError` copy.
- Visual / spacing changes to the comparison loading / error UI.

## After-merge

- Move this file to `plans/_archive/code-review-reality-web-share-comparison-404.md`
- Mark the matching `backlog.json` row as `status: "done"`
