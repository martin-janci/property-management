# mobile — React Native Mobile App

## Screen-Map integration

When implementing or modifying a screen in this app:

1. Identify the screen-map id under the `ppt/` product (mobile screens share screen-maps with ppt-web — they're platforms of the same logical concept).
2. **Before coding**: run `/screens edit ppt/<id>` to load full context.
3. **After coding**: update `implementations.mobile` block (`buildStatus` / `apiStatus` / `redesignStatus`).
4. Add an Agent Log entry: `<date> — agent: <what changed>`.
5. Run `/screens validate`.
6. New screens: `/screens update` then `/screens init --add "<Screen Name>"`.
