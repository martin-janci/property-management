# mobile — React Native Mobile App

## Env config — single source of truth

- `.env.<APP_ENV>` → loaded by `app.config.ts` via dotenv →
  `Constants.expoConfig.extra` + `ios.infoPlist`.
- JS code reads via `src/config/api.ts` (which only consults
  `Constants.expoConfig.extra`).
- `EXPO_PUBLIC_*` duplicates were removed in #523 — do **not** reintroduce
  them; they cannot reach iOS Info.plist and require hand-syncing.
- Production builds: CI must override `API_BASE_URL` / `WS_BASE_URL`. The
  shipped `.env.production` carries a `__SET_BY_CI__` sentinel that
  `app.config.ts` rejects if encountered.

## `start` vs `run` scripts

- `pnpm start` / `pnpm android` / `pnpm ios` — `expo start [--android|--ios]`,
  Metro dev server + dev client. No native toolchain required.
- `pnpm android:build` / `pnpm ios:build` — `expo run:android` /
  `expo run:ios`. Trigger a native prebuild + compile, require full Xcode /
  Android SDK + NDK.

## Screen-Map integration

When implementing or modifying a screen in this app:

1. Identify the screen-map id under the `ppt/` product (mobile screens share screen-maps with ppt-web — they're platforms of the same logical concept).
2. **Before coding**: run `/screens edit ppt/<id>` to load full context.
3. **After coding**: update `implementations.mobile` block (`buildStatus` / `apiStatus` / `redesignStatus`).
4. Add an Agent Log entry: `<date> — agent: <what changed>`.
5. Run `/screens validate`.
6. New screens: `/screens update` then `/screens init --add "<Screen Name>"`.
