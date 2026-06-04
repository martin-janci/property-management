# ppt-web E2E specs (Playwright) — MANUAL-ONLY

These Playwright specs (`*.spec.ts` + `fixtures.ts`, driven by
`../playwright.config.ts`) are **not executed in CI**. Their non-execution is
intentional and documented here — not a silent gap.

## Why they are not run in CI

Running headed browsers on every `frontend/**` PR is slow and prone to
flakiness; wiring a Playwright job into `frontend.yml` is an ops decision that
is deliberately out of scope. So CI does **not** run these specs.

## What CI *does* do

CI type-checks these specs so that compile/import breakage fails fast. The root
`pnpm typecheck` (run in `.github/workflows/frontend.yml`) invokes ppt-web's
`typecheck`, which now also runs `typecheck:e2e`:

```jsonc
"typecheck":     "tsc --noEmit && pnpm typecheck:e2e",
"typecheck:e2e": "tsc --noEmit -p tsconfig.e2e.json"
```

`tsconfig.e2e.json` type-checks `e2e/**` and `playwright.config.ts` against the
`@playwright/test` types. A spec that no longer compiles (e.g. a renamed import
or a removed selector helper) will turn the typecheck job red, even though the
spec itself is never executed.

> Context: `auth-refresh.spec.ts` once shipped logically broken on `dev` (its
> redirect assertions targeted a route that was not behind `<ProtectedRoute>`).
> Typecheck-gating cannot catch that class of *semantic* rot, but it does stop
> *compile-level* rot from accumulating unnoticed.

## Running them locally

The specs target a running dev server (`baseURL: http://localhost:3000`); the
Playwright config starts `pnpm dev` for you via its `webServer` block.

```bash
# from the repo root (frontend/ workspace)
pnpm --filter @ppt/web exec playwright install   # one-time: download browsers
pnpm --filter @ppt/web test:e2e                  # runs `playwright test`
```

Some specs (real login/logout in `auth.spec.ts`) additionally need the backend
API running with seeded test users; they fail gracefully when it is absent.
