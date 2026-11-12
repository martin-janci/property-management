/**
 * Registry Feature Module
 *
 * Exports for building registries feature (Epic 57).
 *
 * INTENTIONALLY UNWIRED (PAP-55 / WS-C, board decision on PAP-28): this
 * feature is fully built with a live api-server backend but is deliberately
 * NOT mounted into the router or navigation — no `routes/groups/registry.tsx`,
 * no `<Route>`, no nav link, and absent from `@ppt/sitemap`. The pages below
 * reading as "orphaned" is the intended state, not dead code. The slug lives
 * in `src/features/unwired-features.ts` and is enforced by the CI guard
 * `src/test/unwired-features.test.ts`, which FAILS if these pages are wired in
 * without the explicit, reviewed act of removing `registry` from
 * `UNWIRED_FEATURES`. Do not wire or delete this feature on code-review
 * grounds alone — wiring requires a scoped phase with UX sign-off; the code is
 * retained per the board decision.
 */

export * from './components';
export * from './hooks';
export * from './pages';
