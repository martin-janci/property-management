# ppt-web — Property Management Web App

## Screen-Map integration

When implementing or modifying a route in this app:

1. Identify the screen-map id (typically `ppt/<kebab-slug>`).
2. **Before coding**: run `/screens edit ppt/<id>` to load full context (related screens, endpoints, recent agent log).
3. **After coding**: update the screen-map's `implementations.ppt-web` block:
   - `buildStatus`: `planned` → `in-progress` → `shipped`.
   - `apiStatus`: `stub` / `partial` / `complete` based on backend reality.
   - `redesignStatus`: only flip to `applied` if a Figma frame was the source of truth.
4. Add an Agent Log entry: `<date> — agent: <what changed>`.
5. Run `/screens validate` to confirm cross-references are clean.
6. If the change adds a new route not yet in the screen-map: run `/screens update` to surface drift, then `/screens init --add "<Screen Name>"` to create the new entry.
