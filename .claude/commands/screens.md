# /screens — Screen-Map dispatcher

Dispatch into a screen-map subcommand. Phase 2 supports `validate`, `init`, `edit`, `review`. Phase 3 will add `update`, `render`, `query`.

## Usage

```bash
/screens validate                            # Phase 1
/screens validate --strict

/screens init --product=ppt                  # Phase 2 NEW
/screens init --product=reality --designs=designs/2026-q2.zip
/screens init --product=ppt --add="Custom screen 1" --add="Custom screen 2"

/screens edit ppt/building-detail            # Phase 2 NEW
/screens edit reality/property-detail --playwright

/screens review                              # Phase 2 NEW
/screens review --product=ppt --preview=staging
```

## Implementation

Parse `$ARGUMENTS` for the first token (subcommand) and the rest (forwarded flags).

- `validate` → invoke the `screen-map-validate` skill.
- `init` → invoke the `screen-map-init` skill (chat-driven grouping).
- `edit <id>` → invoke the `screen-edit` skill.
- `review` → invoke the `screen-map-review` skill.
- `update | render | query` → respond:
  "This subcommand is part of Phase 3 of the screen-map plan and is not yet wired up. See `docs/superpowers/specs/2026-05-07-screen-map-system-design.md` Section 5."
- Missing/unknown subcommand → print this usage block.
