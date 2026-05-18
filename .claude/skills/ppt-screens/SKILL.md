---
name: ppt-screens
description: Author + validate screen-map docs, keep them in sync with routes, and run screen smoke checks via Chrome MCP / Playwright.
when_to_use: The plan adds, removes, or changes a route in ppt-web / reality-web / mobile, or modifies a `docs/screens/<product>/*.md` file, or needs visual proof that a frontend change works.
mode: both
capabilities: [C4, C6]
tags: [frontend, workflow]
---

# PPT Screens

`docs/screens/<product>/<id>.md` is the per-screen source of truth (purpose,
layout sketch, data sources, accessibility notes, design ref). The
`@ppt/screen-map` package parses, validates, and renders these files. Routes
in `frontend/apps/{ppt-web,reality-web,mobile}` must stay in sync — every
real screen has a doc, every doc maps to a real route. The daily research
routine watches for drift via the `screen-map-drift` and `screen-map-orphan`
signals.

## When to invoke

- A plan adds / removes / refactors a route in any of:
  - `frontend/apps/ppt-web/src/{App.tsx,routes/**}`
  - `frontend/apps/reality-web/src/app/**`
  - `frontend/apps/mobile/src/**` (React Native; no Expo Router — see [mobile structure note](#mobile-app-structure-note))
- A plan modifies an existing `docs/screens/{ppt,reality}/*.md` (mobile docs not yet seeded; see note)
- A plan needs visual verification (Chrome / Playwright) — IG5 evidence
  for any frontend-touching plan that doesn't have a pure unit test
- The daily research brief flagged a `screen-map-drift` signal you're
  picking up as a `test-gap` plan

## What it gives you

- Authoring template and frontmatter rules
- `@ppt/screen-map` CLI invocations (`validate`, `discover`, `scan`)
- Sync rules — what must change together
- Visual smoke patterns: Chrome MCP (local) + Playwright codegen
- Cross-references to the screen-map CI gate

## Inputs

- A target product (`ppt`, `reality`, `mobile`) and screen slug
- For visual smoke: the dev stack must be up (`ppt-dev-stack` brings it up)

## Authoring a new screen

1. Pick a slug — short, kebab-case, unique within the product
   (e.g. `building-list`, `host-onboarding`, `unit-detail`).
2. Copy the template:
   ```bash
   cp docs/screens/_template.md docs/screens/<product>/<slug>.md
   ```
3. Fill the frontmatter (Zod-validated by `@ppt/screen-map`). Required
   fields include the screen `id`, `product`, `route`, `status`,
   `owners`, and any `design_ref`. See `frontend/packages/screen-map/src/`
   for the canonical schema.
4. Add the matching route in the app:
   - `ppt-web`: add to `frontend/apps/ppt-web/src/App.tsx` or
     `frontend/apps/ppt-web/src/routes/<area>/<file>.tsx`
   - `reality-web`: `frontend/apps/reality-web/src/app/<route>/page.tsx`
   - `mobile`: `frontend/apps/mobile/src/screens/<Screen>.tsx` registered
     in `frontend/apps/mobile/src/App.tsx` (React Native — no Expo Router)
5. **Commit both files in the same PR.** The screen-map CI workflow
   triggers on either path; submitting one without the other will trip
   the `screen-map-drift` signal on the next routine run even if CI is
   happy with it (the routine sees the asymmetry).

## Modifying an existing screen

If you change a route's path, header, data flow, or accessibility behavior,
update the matching `docs/screens/<product>/<id>.md` in the same PR:

- Route renamed? Update the `route` field in frontmatter.
- New API call? Add to `data_sources`.
- New role-gated visibility? Update the access section in the body.

The screen body markdown is preserved verbatim by `@ppt/screen-map` — only
frontmatter is structured, the rest is free-form per the template.

## Validating

The CLI is wired through pnpm:

```bash
cd frontend
pnpm --filter @ppt/screen-map test                              # unit tests for parser / validator
pnpm --filter @ppt/screen-map run cli -- validate --strict      # validate all docs/screens/**
pnpm --filter @ppt/screen-map run cli -- discover               # list parsed screens (Phase-1+)
```

The `-- ` (double-dash) is what forwards args past the pnpm script wrapper
reliably across pnpm versions. Cold checkouts need `pnpm install` first.
The pre-commit hook runs the same `cli -- validate --strict` invocation,
so passing locally means CI will pass.

## Visual smoke (when a plan ticks C4)

For changes you can't fully verify with a unit test:

### Local — Chrome MCP

Fastest signal. Assumes the dev stack is up (`stack up pm-local` or
`docker compose -f docker-compose.dev.yml up -d`).

1. `mcp__Claude_in_Chrome__list_connected_browsers` → pick the device id
2. `mcp__Claude_in_Chrome__select_browser` with that id
3. `mcp__Claude_in_Chrome__tabs_context_mcp` (creates the tab group)
4. `mcp__Claude_in_Chrome__tabs_create_mcp` for a fresh tab
5. `mcp__Claude_in_Chrome__navigate` to the screen URL (e.g.
   `http://localhost:3000/buildings`)
6. `mcp__Claude_in_Chrome__get_page_text` / `read_page` (accessibility tree) /
   `read_console_messages` / `read_network_requests` to inspect

If the Chrome MCP doesn't honour navigation (sometimes the isolated tab
group misbehaves on first use), close the tab and create a new one — the
second navigate usually works. If it still doesn't, fall back to Playwright.

### Local — Playwright codegen (scripted)

For flows you want to keep as a regression test:

```bash
npx playwright codegen http://localhost:3000/<route>
```

Codegen opens an instrumented browser; click through the flow once and
save the output to the app's e2e directory. For `ppt-web` that's
`frontend/apps/ppt-web/e2e/<flow>.spec.ts` (e.g. `auth.spec.ts`,
`home.spec.ts`, `navigation.spec.ts`, `pages.spec.ts`). Then:

```bash
cd frontend
pnpm --filter @ppt/ppt-web exec playwright test
```

(Verify the package name in `frontend/apps/ppt-web/package.json` — pnpm
filters silently match nothing if the name is wrong.)

### Cloud (via ppt-bridge)

`ppt_browser_open` is **v2 skeleton** — currently returns `not_implemented`
even when a host has the `browser` capability. The intended shape (when
v2 lands) is `ppt_browser_open(url, viewport)` → returns DOM text +
screenshot URL. Until then, cloud routine sessions can only smoke-test
URLs that respond to HEAD/GET without browser rendering (e.g. via curl).

## Deterministic verification

```bash
# 1. docs/screens/ is non-empty for each seeded product (ppt + reality).
# Mobile docs are intentionally not seeded yet — see "Mobile app structure
# note" below; mobile is expected to read 0 until screens land for it.
for p in ppt reality; do
  count=$(find "docs/screens/$p" -maxdepth 2 -name '*.md' -type f 2>/dev/null | wc -l | tr -d ' ')
  [ "$count" -gt 0 ] && printf "%-8s %s files\n" "$p" "$count" || echo "EMPTY: docs/screens/$p"
done
# expected: ppt, reality each report >0 files

# 2. screen-map package present
test -f frontend/packages/screen-map/package.json && echo OK
# expected: OK

# 3. screen-map.yml workflow present
test -f .github/workflows/screen-map.yml && echo OK
# expected: OK

# 4. template present
test -f docs/screens/_template.md && echo OK
# expected: OK
```

## Smoke check (single command)

```bash
find docs/screens/ppt docs/screens/reality -name '*.md' -type f -not -name '_template.md' -not -name 'README.md' 2>/dev/null | grep -q .
```

> Asserts at least one parsed-eligible screen doc exists under a seeded
> product directory. Scoped to `ppt/` + `reality/` so top-level
> `_template.md` / `README.md` and unrelated `*.md` files added later
> don't accidentally pass the check. Doesn't run the validator (that
> needs `pnpm install`), just confirms the tree shape.

## After-task verification

After authoring or modifying a screen + route:

```bash
# 1. paired files in the diff
git diff --name-only main..HEAD | grep -E '(frontend/apps/(ppt-web|reality-web|mobile)/|docs/screens/)' | sort
# expected: at least one route file AND at least one docs/screens/ file

# 2. screen-map validator clean
cd frontend && pnpm --filter @ppt/screen-map cli validate --strict

# 3. (if C4 was ticked) visual smoke captured in PR body under "IG5 — Screen evidence"
```

## Mobile app structure note

`frontend/apps/mobile/` is a **React Native / Expo** app **without Expo
Router** — there is no `frontend/apps/mobile/app/` directory. The actual
shape is:

- `frontend/apps/mobile/src/App.tsx` — root component + screen registry
- `frontend/apps/mobile/src/screens/<Screen>.tsx` — one file per screen

Treat any documentation or signal definition that mentions
`frontend/apps/mobile/app/**` as legacy / stale and update it to
`frontend/apps/mobile/src/**`. The routine's `screen-map-drift` trigger
paths and the screen-map workflow already use `src/**`.

Mobile screen docs under `docs/screens/mobile/` are not yet seeded — the
directory legitimately doesn't exist on `main`. Verification blocks here
explicitly drop mobile until the first batch lands; signal definitions
and brief templates do the same.

## Cross-references

- `frontend/packages/screen-map/README.md` — package overview + phase scope
- `docs/superpowers/specs/2026-05-07-screen-map-system-design.md` — full design
- [`ppt-frontend`](../ppt-frontend/SKILL.md) — app layout, route file locations
- [`ppt-pr-create`](../ppt-pr-create/SKILL.md) — where to paste IG5 visual evidence
- [`ppt-tests`](../ppt-tests/SKILL.md) — choosing test runners
- `.github/workflows/screen-map.yml` — CI gate (same trigger paths as the
  routine's `screen-map-drift` signal)
