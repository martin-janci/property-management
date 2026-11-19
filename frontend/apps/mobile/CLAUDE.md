# mobile — React Native Mobile App

## Lint gates (React Native) — warn-only rollout

`biome.json` (nested, `extends: "//"`) adopts the rules that close the
chronic mobile-rn lint-gap risk (stale-closure "ranked-ballot" bugs +
hardcoded-string i18n fallback bugs). These map the ESLint-era intent onto
the workspace's Biome gate:

| Intent (ESLint)              | Biome rule / plugin                         | Severity |
| ---------------------------- | ------------------------------------------- | -------- |
| `react-hooks/rules-of-hooks` | `correctness.useHookAtTopLevel`             | `warn`   |
| `react-hooks/exhaustive-deps`| `correctness.useExhaustiveDependencies`     | `warn`   |
| `i18next/no-literal-string`  | `biome-plugins/no-hardcoded-strings.grit`   | `warn`   |

All three are **blocking** (they run inside the mandatory `pnpm check`
lint job — mobile `src/` is in the workspace Biome scope) but **warn-only
for one sprint**: warnings surface in CI/local output without failing the
gate, giving a sprint to migrate the existing backlog.

### Flip-back: tracked, not a prose TODO (issue #3584)

The warn-only window is **dated and gated**, not a "remember next sprint"
note. Promote all three severities back to `error` by **2026-12-01**
(`useHookAtTopLevel` / `useExhaustiveDependencies` in `biome.json`, and
`severity="error"` in `biome-plugins/no-hardcoded-strings.grit`) once the
backlog is clear. Do not weaken test gates to get green — fix the flagged
code instead.

Enforcement lives in `biome-lint-gate.test.ts` (runs in `pnpm --filter
mobile test`): it pins the rules/plugin as present today and, **once the
`2026-12-01` deadline passes, fails CI until the severities are back at
`error`** — so the flip-back cannot be silently forgotten. If more time is
genuinely needed, bump `FLIP_BACK_DEADLINE` in that test *with a justification
recorded on [issue #3584](https://github.com/martin-janci/property-management/issues/3584)*
rather than deleting the guard.

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

## EAS CLI — install globally, not as a devDep

`pnpm build:android:staging` / `build:ios:staging` / `submit:*` shell out to
the `eas` binary. **You must install it globally** — it is intentionally NOT
in `devDependencies` (issue #715: it bundled the full Node CLI, ~80–100 MB
through the workspace, and CI already installs it via
`expo/expo-github-action@v8` with `eas-version: latest`).

```bash
npm install -g eas-cli                # one-time, on your workstation
eas login                             # or set EXPO_TOKEN in the env
eas whoami                            # sanity-check
```

CI builds (`.github/workflows/eas-build-{android,ios}.yml`) use the
expo-github-action's global install — they do not consult the package.json.

## Screen-Map integration

When implementing or modifying a screen in this app:

1. Identify the screen-map id under the `ppt/` product (mobile screens share screen-maps with ppt-web — they're platforms of the same logical concept).
2. **Before coding**: run `/screens edit ppt/<id>` to load full context.
3. **After coding**: update `implementations.mobile` block (`buildStatus` / `apiStatus` / `redesignStatus`).
4. Add an Agent Log entry: `<date> — agent: <what changed>`.
5. Run `/screens validate`.
6. New screens: `/screens update` then `/screens init --add "<Screen Name>"`.
