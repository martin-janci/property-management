# reality-web — Reality Portal Web App

## Screen-Map integration

When implementing or modifying a route in this app:

1. Identify the screen-map id (typically `reality/<kebab-slug>`).
2. **Before coding**: run `/screens edit reality/<id>` to load full context.
3. **After coding**: update `implementations.reality-web` block (`buildStatus` / `apiStatus` / `redesignStatus`).
4. Add an Agent Log entry: `<date> — agent: <what changed>`.
5. Run `/screens validate`.
6. New routes: `/screens update` then `/screens init --add "<Screen Name>"`.
