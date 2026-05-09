# @ppt/screen-map

TS API + CLI for the screen-map system (see `docs/superpowers/specs/2026-05-07-screen-map-system-design.md`).

## What it does

- Parses `docs/screens/<product>/<id>.md` into typed objects.
- Validates frontmatter (Zod) and cross-references `@ppt/sitemap`.
- Writes back changes preserving body markdown verbatim.
- Exposes a CLI used by the `/screens validate` slash command and the pre-commit hook.

## Quickstart

```bash
pnpm --filter @ppt/screen-map test
pnpm --filter @ppt/screen-map cli validate --strict
```

## Phase 1 scope

Foundation: types, parse, write, validate, discover, CLI (`validate` subcommand).

Phase 2 adds: scan (route detection), DesignSource, Visual Review server.
Phase 3 adds: render, query, agent self-management glue.
