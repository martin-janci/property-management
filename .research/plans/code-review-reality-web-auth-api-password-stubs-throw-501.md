# code-review-reality-web-auth-api-password-stubs-throw-501

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-09-04 (Phase 1.5 — reality-web segment)
**Confidence:** high

## Hypothesis

Three reality-web self-serve credential pages (`/account/password`, `/auth/forgot-password`, `/auth/reset-password`) render UI, wire form submit, and call auth-api client functions (`changePassword`, `requestPasswordReset`, `confirmPasswordReset`) that **unconditionally throw** `AuthApiError('Password reset is not available yet. Please contact support.', 501, 'NOT_IMPLEMENTED')` — the stub bodies just throw. The three UI flows always dead-end from the user's perspective. Smallest correct change: hide the three UI entry points behind a single `NEXT_PUBLIC_REALITY_PASSWORD_FLOWS_ENABLED` feature flag (default off), and add a shared "not available" empty state that reads the same locale key the current thrown message uses. Wiring the client stubs to real endpoints is out of scope for this plan — the reality-server endpoints they reference (`/api/v1/users/password-reset`, `/password-reset/confirm`, `/users/me/password`) do not exist yet and would require a separate reality-server plan.

## Evidence

- `frontend/apps/reality-web/src/lib/auth-api.ts:121-149` — all three functions throw `AuthApiError(…, 501, 'NOT_IMPLEMENTED')` with a `// TODO: wire when reality-server exposes …` comment.
- `frontend/apps/reality-web/src/app/[locale]/account/password/page.tsx:48` — `await changePassword(currentPassword, newPassword)` inside form submit.
- `frontend/apps/reality-web/src/app/[locale]/auth/reset-password/page.tsx:72` — `await confirmPasswordReset(token, password)` inside submit.
- `frontend/apps/reality-web/src/app/[locale]/auth/forgot-password/page.tsx:33` — `await requestPasswordReset(trimmed)` inside submit.
- Reality-server side has no matching routes — the stubs' `// TODO: wire when reality-server exposes …` comments name endpoints that do not exist today.

## Files

- `frontend/apps/reality-web/src/lib/auth-api.ts`
- `frontend/apps/reality-web/src/app/[locale]/account/password/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/auth/reset-password/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/auth/forgot-password/page.tsx`

## Dependencies

<!-- No task_id dependencies. Wiring to real reality-server endpoints is out of scope. -->

## Required capabilities

- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
- Neither C4 nor C5 is ticked → cloud-ok.

Mode: cloud-ok

## Repro steps

1. Serve reality-web locally (or against staging) and visit `/en/auth/forgot-password`.
2. Type any email, click Send. Expected: informative "check your inbox" empty state OR the page is not reachable. Actual: form submit surfaces `AuthApiError(501, 'NOT_IMPLEMENTED')` from `requestPasswordReset` and the page renders the fallback error — a user-facing dead-end.
3. Repeat for `/en/auth/reset-password?token=xyz` and `/en/account/password` — both dead-end the same way.

## Suggested approach

1. Introduce a `NEXT_PUBLIC_REALITY_PASSWORD_FLOWS_ENABLED` env var (defaults to `"false"`) read via a small helper (`src/lib/feature-flags.ts` if absent, else colocated in `auth-api.ts`).
2. In `src/lib/auth-api.ts`, keep the three functions but have them read the flag first: when disabled, throw a typed `AuthApiError('Password self-service is not available yet — contact support.', 501, 'NOT_IMPLEMENTED')` — behavior identical to today, but centralized.
3. In each of the three page components, add a top-level guard: if the flag is off, render a translated `NotAvailableCard` (share the message via a new `auth.passwordFlowsDisabled` i18n key). No form. This removes the dead-end submit path.
4. Update `messages/{en,sk,cs,de}.json` with the new `auth.passwordFlowsDisabled` key (short one-line message + support-contact link).
5. Hide the "Forgot password?" link on `/auth/login` when the flag is off — otherwise the flow entry is a broken cul-de-sac.
6. Update `docs/screens/reality/` frontmatter for `password`, `forgot-password`, `reset-password` entries (if they exist) to note `buildStatus: gated-by-flag` and add an Agent Log entry.
7. Add a `next.config.js` or README note enumerating the flag so it's discoverable.

## Alternatives considered

- **Wire the three client stubs directly to new reality-server endpoints** — rejected because reality-server does not expose `/api/v1/users/password-reset[/confirm]` or `PUT /api/v1/users/me/password` today. That's a separate backend plan; conflating the two blows scope and blocks landing.
- **Delete the three pages outright** — rejected because they represent shipped UI surfaces referenced from `docs/screens/reality/`, and deleting them would drop screen coverage entries and hide the intended product feature; a flag preserves the intent while removing the broken path.

## Root-cause trace

1. Symptom: `AuthApiError(501, 'NOT_IMPLEMENTED')` thrown on submit of `/account/password`, `/auth/forgot-password`, `/auth/reset-password`.
2. ← `frontend/apps/reality-web/src/lib/auth-api.ts:123`, `:132`, `:144` — the three functions throw unconditionally.
3. ← The `// TODO: wire when reality-server exposes …` comments show intent: the client was scaffolded before its server-side. Reality-server never exposed the matching routes.
4. Origin: initial commit of `auth-api.ts` (pre-cursor) — no PR merged since has touched these three functions.

## Test plan

- [ ] `frontend/apps/reality-web/src/lib/auth-api.test.ts` — new: assert that when the flag is `"false"` (default), all three functions throw `AuthApiError(501,'NOT_IMPLEMENTED')`, and when `"true"`, they attempt a fetch (mock-verified). This is the failing-on-main assertion the IG3 test needs.
- [ ] Vitest for each of the three pages — assert that when the flag is off, the guarded `NotAvailableCard` renders instead of the form, and that submit is not present in the DOM.
- [ ] Vitest for the `/auth/login` page — assert that the "Forgot password?" link is not rendered when the flag is off.
- [ ] Run: `cd frontend && pnpm --filter @ppt/reality-web test`

## Out of scope

- Server-side endpoints for password reset / change (`reality-server`) — separate plan needed.
- Any change to api-server SSO flow.
- Mobile / KMP portal client (`mobile-native/`) — different auth surface.
- Redesign of the `/account/password` page layout — this plan only gates.

## After-merge

- Move this file to `plans/_archive/code-review-reality-web-auth-api-password-stubs-throw-501.md`
- Mark `backlog.json` row `code-review-reality-web-auth-api-password-stubs-throw-501` as `status: "done"`
