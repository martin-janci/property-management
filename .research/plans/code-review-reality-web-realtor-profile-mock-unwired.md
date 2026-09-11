# code-review-reality-web-realtor-profile-mock-unwired

**Vector:** bug
**Score:** 3
**Source:** dispatcher Tier-1d rotating-expert-review 2026-09-03 (reality-web)
**Confidence:** high

## Hypothesis
The public agent-profile route `/[locale]/realtor/[id]` renders three
hardcoded fixtures (`MOCK_AGENT`, `MOCK_AGENT_LISTINGS`, `MOCK_AGENT_REVIEWS`)
imported from `./_mock`, never reads the `[id]` route param, never calls
the reality-server, and its contact-form `handleSendMessage` is a UI-only
no-op that never POSTs the inquiry. Every `/[locale]/realtor/<any-id>` URL
shows the same fake agent, and realtor contact requests are silently
dropped. On top of that the same file hardcodes Slovak-only chrome
("✓ Overený maklér", "Aktívne ponuky", "Hodnotenia") — cs/de/en visitors
see Slovak. The reality-server already exposes `GET /api/v1/realtors/{user_id}/profile`
and the inquiries endpoint the contact form should POST to, so this is a
frontend-wiring job on an app that builds cleanly in cloud (unlike
api-server per issue #2949 and mobile-native per #2652). Two tier-1d
signals converge on the same file — score stacks to 3, confidence high →
promotable.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx:11` — `import { MOCK_AGENT, MOCK_AGENT_LISTINGS, MOCK_AGENT_REVIEWS } from './_mock';`
- `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx:20` — `const agent = MOCK_AGENT;` (the `[id]` param is discarded)
- `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx:22-28` — `handleSendMessage` sets `messageSent=true` only; no network call
- `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx:114,227,330` — hardcoded Slovak: `"✓ Overený maklér"`, `"Aktívne ponuky (…)"`, `"Hodnotenia (…)"`
- Wiring targets already in place: `backend/servers/reality-server/src/routes/realtors.rs:86` (`GET /api/v1/realtors/{user_id}/profile`) and the existing `frontend/apps/reality-web/src/lib/realtor-api.ts` `request()` helper used by sibling routes

## Files
- `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/_mock.ts`
- `frontend/apps/reality-web/src/lib/realtor-api.ts`
- `frontend/apps/reality-web/messages/sk.json`
- `frontend/apps/reality-web/messages/en.json`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug fix, network wiring)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (reality-server + a seeded realtor profile to sanity-check the fetch)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived):** `cloud-ok` — reality-web + reality-server
both build in the cloud runner. `stack up pm-local` (or ppt-bridge) can
stand up a seeded dev instance for the manual smoke, but the automated
suite (`pnpm -F @ppt/reality-web test -- --run`) is sufficient for
verification.

Mode: cloud-ok

## Repro steps
1. Start the reality-web app pointed at reality-server (`stack up pm-local` or `pnpm -F @ppt/reality-web dev` with `NEXT_PUBLIC_REALITY_API_URL` set to a running reality-server).
2. Open `http://localhost:3000/en/realtor/anything-here` and `http://localhost:3000/cs/realtor/anything-here`.
3. Observed: both URLs render the same hardcoded Slovak mock agent regardless of `[id]` and regardless of `[locale]`. Fill the contact drawer and submit — no request is issued.
4. Expected after fix: each `[id]` resolves against `GET /api/v1/realtors/{id}/profile` (returns `notFound()` on 404), chrome renders in the active locale, and submitting the drawer POSTs the inquiry (visible in the network tab, 2xx response, `messageSent` set only on success).

## Suggested approach
1. Add typed fetchers to `frontend/apps/reality-web/src/lib/realtor-api.ts`: `getPublicRealtorProfile(userId, opts): Promise<RealtorProfile | null>` (returns `null` on 404 so the page can call `notFound()`), `getRealtorPublicListings(userId, { limit }): Promise<...>`, `getRealtorReviews(userId, { limit }): Promise<...>`, `submitRealtorInquiry(userId, payload): Promise<void>`. Route each through the existing `request()` helper.
2. Convert `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx` into a `Server Component` that reads `params.id` (Next 16 async params) and calls the three fetchers in parallel with `Promise.all`. Delete the `MOCK_AGENT*` imports; render `notFound()` when the profile fetcher returns null. Keep the drawer as a Client Component boundary.
3. Convert `handleSendMessage` into an async handler that calls `submitRealtorInquiry(id, { name, email, message, phone? })` and only sets `messageSent=true` on a 2xx response; surface a localised error toast (`useTranslations('errors')`) on failure. Reset `messageSent` when the drawer opens.
4. Route the three hardcoded strings (`✓ Overený maklér`, `Aktívne ponuky ({n})`, `Hodnotenia ({n})`) plus any other visible copy through `useTranslations('agentProfile')`; add the keys to `messages/sk.json` and `messages/en.json` (plus cs/de if the file has them). Interpolate the counts as `t('activeListings', { count })`.
5. Delete `_mock.ts` and its import from `page.tsx`. Keep the shape of the props/types in `realtor-api.ts` — the mocks were the only consumer.
6. Add a Vitest render test (`page.test.tsx` next to it) that mocks `getPublicRealtorProfile` returning null → asserts `notFound()` was thrown; a second case with a fixture profile asserts the localised chrome strings are rendered from the message catalog.
7. `pnpm -F @ppt/reality-web typecheck && pnpm -F @ppt/reality-web test -- --run && pnpm biome check frontend/apps/reality-web/src/{lib,app/[locale]/realtor}`.

## Alternatives considered
- **Keep the mocks behind a `NEXT_PUBLIC_USE_MOCK_AGENT=true` flag** — rejected because it hides the fact that the public route is unwired behind an env var, and every real deployment already has the flag off; the mock file is dead weight either way and a flag adds a new dimension of drift.
- **Move the fetches into a client `useEffect`** — rejected because the app is Next SSR (reality-web is Next.js 16) and public pages should hydrate with data (SEO + first-render UX for the listings/reviews sections); the sibling `listings/[slug]/page.tsx` is a Server Component for the same reason, and following the local pattern keeps the code readable.

## Root-cause trace
1. Symptom: every `realtor/<id>` URL renders the same Slovak mock agent regardless of the id and the requesting locale; submitting the contact drawer never sends anything to the server.
2. ← `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx:20` uses `const agent = MOCK_AGENT`; the `params.id` and `locale` are never consumed.
3. ← `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx:22-28` `handleSendMessage` mutates local UI state only — the drawer was stubbed before the inquiries endpoint was live.
4. ← `frontend/apps/reality-web/src/lib/realtor-api.ts` only exposes `me`-scoped helpers (`/api/v1/realtors/me`); no public-profile getter was ever added, so the page had nothing to call.
5. Origin: the agent-profile route landed as a UI-first stub during the agency-flow build (see sibling `_mock.ts` still present in `agency/`), intended to be wired against `realtors.rs` once that shipped, but the wiring PR never followed. The reality-server routes (`realtors.rs:86` `/api/v1/realtors/{user_id}/profile`) are now available, so the stub can retire.

## Test plan
- [ ] `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.test.tsx` — new; two render cases (404 → `notFound()`, hit → renders localized chrome + first listing/review from the fixture). IG3-quality: must fail on `main` because the current file imports `MOCK_AGENT` and never calls the fetcher.
- [ ] `frontend/apps/reality-web/src/lib/realtor-api.test.ts` — extend with unit coverage for `getPublicRealtorProfile` (200, 404, 5xx) and `submitRealtorInquiry` (2xx path only + throws on non-2xx).
- [ ] `pnpm -F @ppt/reality-web test -- --run` and `pnpm -F @ppt/reality-web typecheck`.
- [ ] Optional smoke (needs C3): `curl -sS $REALITY_API/api/v1/realtors/<seed-user>/profile | jq .` then load `http://localhost:3000/en/realtor/<seed-user>`.

## Out of scope
- Adding new endpoints on reality-server — the existing `/api/v1/realtors/{user_id}/profile`, listings, reviews, and inquiries endpoints are sufficient.
- Redesigning the drawer UX or the reviews section.
- SEO / metadata (og:image, JSON-LD) — separate vector once the data is real.
- The unrelated realtor-management modal (`components/agency/RealtorManagement.tsx`) — already landed as `code-review-reality-web-realtor-mgmt-untranslated`.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-realtor-profile-mock-unwired.md`.
- Mark the matching `backlog.json` row `status: "done"` and append the merged PR # to `sources`.
- If `code-review-reality-web-realtor-profile-hardcoded-slovak` is still open in backlog, mark it `status: "done"` too (this plan resolves both signals).
