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
