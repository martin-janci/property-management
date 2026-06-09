# Screen-Map

Project-management layer for both products (PPT and Reality). One file per logical screen; each file mixes a typed YAML frontmatter (status, endpoints, relations) with free-form markdown (functionality checklist, states, notes, agent log).

See the design spec: [`docs/superpowers/specs/2026-05-07-screen-map-system-design.md`](../superpowers/specs/2026-05-07-screen-map-system-design.md).

## Layout

```
docs/screens/
├── README.md
├── _template.md           # copy when creating a new screen
├── ppt/                   # PPT product (ppt-web + mobile)
│   └── <kebab-id>.md
└── reality/               # Reality product (reality-web + mobile-native)
    └── <kebab-id>.md
```

## Frontmatter contract

Authoritative status lives in the frontmatter; markdown body holds the things humans read and edit. The `@ppt/screen-map` package validates the shape; Phase 2 skills mutate it.

Every entry must have:

- `id: <product>/<kebab-slug>` matching the file path.
- `product: ppt` or `reality`.
- `implementations.<platform>` for every platform that *exists or will exist*; use `n/a` statuses if the platform is intentionally not in scope.

See `_template.md` for a full skeleton.

## Tooling

- `/screens validate` — run the validator against this whole tree.
- Pre-commit hook auto-validates any `docs/screens/**` you stage.
- CI re-runs `validate --strict` on PRs that touch `docs/screens/**` or route files.

Phase 2/3 add: init, update, review (with a visual UI), edit, render (mermaid), query.

## ppt-web: hidden built-but-unwired features (not in nav)

Per the Stable Beta board decision ([PAP-55](/PAP/issues/PAP-55) / WS-C, recorded
on [PAP-28](/PAP/issues/PAP-28)), the following **18 ppt-web features are fully
built with live backends but intentionally NOT wired into the router or nav** —
they are hidden from customers now and scheduled for deliberate wiring later. The
code is retained, not deleted.

Because they are not navigable, they have **no screen-map entries** here by design
(a screen-map describes a reachable screen). When one is wired in a future phase,
create its screen-map at that time and remove its slug from the registry below.

`insurance` · `marketplace` · `forms` · `onboarding` · `critical-notifications` ·
`migration` · `subscription` · `government-portal` · `integrations` ·
`compliance` · `registry` · `multi-currency` · `portfolio-performance` ·
`api-ecosystem` · `delegation` · `person-months` · `data-residency` · `packages`

Source of truth / regression guard:
`frontend/apps/ppt-web/src/features/unwired-features.ts` and its test
`frontend/apps/ppt-web/src/test/unwired-features.test.ts` (CI fails if any of
these is re-wired into the routing surface without being removed from the list).
(`meters` / `leases` are owned by [PAP-20](/PAP/issues/PAP-20); `voting` by
[PAP-19](/PAP/issues/PAP-19) — those are being wired, not hidden.)
