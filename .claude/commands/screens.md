# /screens — Screen-Map dispatcher

Dispatch into a screen-map subcommand. Phase 3a supports all 7 subcommands.

## Usage

```bash
/screens validate                                       # Phase 1
/screens validate --strict

/screens init --product=ppt                             # Phase 2
/screens init --product=reality --designs=designs/2026-q2.zip
/screens init --product=ppt --add="Custom screen 1"

/screens edit ppt/building-detail                       # Phase 2
/screens edit reality/property-detail --playwright

/screens review                                         # Phase 2
/screens review --product=ppt --preview=staging

/screens update                                         # Phase 3a NEW
/screens update --strict

/screens render                                         # Phase 3a NEW
/screens render --scope=ppt
/screens render --out=/tmp/diagrams

/screens query                                          # Phase 3a NEW
/screens query "product:ppt"
/screens query "implementations.ppt-web.redesignStatus:in-progress" --format=md
```

## Implementation

Parse `$ARGUMENTS` for the first token (subcommand) and the rest (forwarded flags).

- `validate` → invoke the `screen-map-validate` skill.
- `init` → invoke the `screen-map-init` skill (chat-driven grouping).
- `edit <id>` → invoke the `screen-edit` skill.
- `review` → invoke the `screen-map-review` skill.
- `update` → invoke the `screen-map-update` skill.
- `render` → invoke the `screen-render` skill.
- `query <expr>` → invoke the `screen-query` skill.
- Missing/unknown subcommand → print this usage block.
