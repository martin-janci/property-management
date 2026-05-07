# /screens — Screen-Map dispatcher

Dispatch into a screen-map subcommand. Phase 1 supports only `validate`; Phase 2/3 will add `init`, `update`, `review`, `edit`, `render`, `query`.

## Usage

```bash
/screens validate                # validate all screen-maps
/screens validate --strict       # exit non-zero on any error (CI mode)
```

## Implementation

Parse `$ARGUMENTS` for the first token (subcommand) and the rest (forwarded flags).

- If subcommand is `validate`: invoke the `screen-map-validate` skill with the remaining args.
- If subcommand is `init`, `update`, `review`, `edit`, `render`, or `query`: respond
  "This subcommand is part of Phase 2/3 of the screen-map plan and is not yet wired up. See `docs/superpowers/specs/2026-05-07-screen-map-system-design.md` Section 5."
- If subcommand is missing or unknown: print this usage block.
