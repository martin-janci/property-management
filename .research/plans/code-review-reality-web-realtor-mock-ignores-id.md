# code-review-reality-web-realtor-mock-ignores-id

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-08-22 (reality-web segment)
**Confidence:** high

## Hypothesis
The public `/[locale]/realtor/[id]` route on reality-web imports `MOCK_AGENT` from a `_mock.ts` sibling, never reads the `[id]` route param, and never issues a fetch. Every realtor URL renders the identical hard-coded profile — duplicate content indexed by crawlers plus wrong data shown to visitors. A `GET /api/v1/realtors/{user_id}/profile` endpoint already exists on reality-server, and the `useRealtor(agencyId, realtorId)` hook already exists in `@ppt/reality-api-client`. Smallest fix: read `id` from `useParams()`, fetch the realtor profile, and delegate to `notFound()` on a 404 (mirroring the SSR `listings/[slug]/page.tsx` pattern already documented in the repo).

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx:1-20` — `'use client'` page; imports `MOCK_AGENT`, `MOCK_AGENT_LISTINGS`, `MOCK_AGENT_REVIEWS` from `./_mock`; never calls `useParams`/`params`; assigns `const agent = MOCK_AGENT`.
- `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/_mock.ts` (147 lines) — the fixture that every realtor URL currently renders.
- `frontend/packages/reality-api-client/src/agency/hooks.ts:169-173` — `useRealtor(agencyId, realtorId)` returns real realtor data via `/api/v1/agencies/{agencyId}/realtors/{realtorId}` (needs agency scope; `/api/v1/realtors/{user_id}/profile` is the id-only alternative).
- `backend/servers/reality-server/src/routes/realtors.rs:86` — `GET /api/v1/realtors/{user_id}/profile` returns a public realtor profile keyed by user id, matching the `[id]` route shape exactly.
- Sibling `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/page.tsx` fetches server-side, calls `notFound()` on 404, and documents the anti-pattern this route currently exhibits.

## Files
- `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/_mock.ts`
- `frontend/packages/reality-api-client/src/agency/hooks.ts`
- `backend/servers/reality-server/src/routes/realtors.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Run `frontend/apps/reality-web` locally (or open production).
2. Visit `/sk/realtor/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa` and `/sk/realtor/bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb` in the same browser.
3. Expected: two distinct profiles (or a 404 for an unknown id). Actual: both URLs render the identical `MOCK_AGENT` profile with HTTP 200.

## Suggested approach
1. Convert `page.tsx` from a `'use client'` component to a server component (drop the `'use client'` pragma; keep the interactive `contactOpen`/`message`/`messageSent` state in a small client sub-component under the same folder).
2. Read `params.id` from the server component's props (`{ params }: { params: { locale: string; id: string } }`).
3. Call the reality-server endpoint (either via a server-side `fetch` to `/api/v1/realtors/{id}/profile` — mirroring `listings/[slug]/page.tsx` — or wire a `useRealtorProfile(id)` hook in `@ppt/reality-api-client` and add a matching endpoint entry if not already generated).
4. On `null` / 404 response, call `notFound()` (imported from `next/navigation`) to render the Next.js 404 page instead of a soft-200.
5. Replace `MOCK_AGENT` / `MOCK_AGENT_LISTINGS` / `MOCK_AGENT_REVIEWS` reads with the fetched data; delete `_mock.ts` when nothing else imports it (grep to confirm before removal).
6. Split the existing contact-form state into a client `<AgentContactSection agent={agent} />` component so the top-level page can stay server-rendered.
7. Update the screen-map at `docs/screens/reality/agent-profile.md` (referenced in the file header) to reflect the new API dependency and the notFound behaviour.

## Alternatives considered
- **Keep the page client-side and use `useRealtor(agencyId, id)` from the URL** — rejected because the route path has no `agencyId` segment, and putting a client fetch behind the SEO path leaves the crawler seeing the initial-render skeleton or the mock again during SSR.
- **Redirect the mock page to the agency-scoped route (`/[locale]/agency/[slug]/realtor/[id]`)** — rejected because the id-only public URL is the one crawlers index today; changing the URL shape would break existing external links and require a redirect table on top of the fix.

## Root-cause trace
1. Symptom: every `/[locale]/realtor/[id]` URL renders the same profile; crawler indexes duplicate content under distinct URLs.
2. ← `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx:19` assigns `const agent = MOCK_AGENT;` unconditionally, without ever reading `params.id`.
3. ← The route was scaffolded as a mock while the reality-server `GET /api/v1/realtors/{user_id}/profile` endpoint was in-flight and never wired to real data after the endpoint landed.
4. Origin: initial reality-web `[id]` page scaffold — the `_mock.ts` fixture was added as a placeholder and never removed once `backend/servers/reality-server/src/routes/realtors.rs:86` shipped.

## Test plan
- [ ] `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.test.tsx` — mock the fetch client, assert two distinct ids render distinct names and that an unknown id triggers `notFound()`.
- [ ] Regression: existing contact-form state test still passes after the extraction.
- [ ] `cd frontend && pnpm -F reality-web test src/app/\[locale\]/realtor` and `pnpm -F reality-web typecheck`.

## Out of scope
- Full ISR/revalidation policy tuning for realtor profiles.
- Redesigning the profile visual (redesignStatus stays whatever the screen-map has).
- Wiring realtor reviews (`agent_reviews.rs`) — the current mock includes `MOCK_AGENT_REVIEWS`; if the fetched profile omits reviews, keep the section empty rather than expanding scope.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-realtor-mock-ignores-id.md`
- Mark the matching `backlog.json` row as `status: "done"`
