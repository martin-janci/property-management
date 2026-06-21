# ppt-mobile — Property Management (Expo / React Native)

The Property Management mobile app (`three.two.bit.ppt.management`). Built with
Expo + React Native and talks to `api-server` (port 8080).

For agent-specific conventions, see [`CLAUDE.md`](./CLAUDE.md).

## Environment variable setup

The app is configured per environment (development / staging / production)
through `.env.<APP_ENV>` files. This section is the developer-facing reference
for which variables exist, how `APP_ENV` selects a file, and how to run each
environment.

### How config flows (single source of truth)

```
.env.<APP_ENV>  ──dotenv──▶  app.config.ts  ──▶  Constants.expoConfig.extra  ──▶  src/config/api.ts (JS)
                                            └──▶  ios.infoPlist (native iOS keys)
```

- `app.config.ts` is the **only** loader. On startup it picks an environment
  (see below), loads the matching `.env.<env>` via `dotenv`, and injects the
  values into both `Constants.expoConfig.extra` (read by JS) and
  `ios.infoPlist` (read by native iOS).
- JS code never reads `process.env` directly — it calls helpers in
  [`src/config/api.ts`](./src/config/api.ts), which only consult
  `Constants.expoConfig.extra`.
- `EXPO_PUBLIC_*` variables are **not** used here (removed in issue #523).
  They cannot reach the iOS `Info.plist` and would require hand-syncing two
  copies of every value. Do not reintroduce them. The `Constants` re-export in
  [`src/config/constants.ts`](./src/config/constants.ts) (`API_BASE_URL`) reads
  through `getApiBaseUrl()` for the same reason — it never touches
  `process.env`.

### Reading config in JS vs. native iOS

There are two consumers of the resolved env values, both fed by `app.config.ts`:

| Consumer | Reads from | API |
| -------- | ---------- | --- |
| JavaScript / TypeScript | `Constants.expoConfig.extra` | `getApiBaseUrl()`, `getEnvironment()`, `getWsBaseUrl()`, `isDebugMode()` in [`src/config/api.ts`](./src/config/api.ts) (via `expo-constants`). Never read `process.env` at module scope. |
| Native iOS (Swift / Obj-C) | `Info.plist` | `Bundle.main.object(forInfoDictionaryKey: "API_BASE_URL")` / `"ENVIRONMENT"`. |

`app.config.ts` injects the following keys into `ios.infoPlist` so native code
can read the same per-environment values without a second config path:

| `Info.plist` key | Source env var | Notes |
| ---------------- | -------------- | ----- |
| `API_BASE_URL`   | `API_BASE_URL` | REST base URL — identical to `extra.API_BASE_URL`. |
| `ENVIRONMENT`    | `ENVIRONMENT`  | Logical environment label (`development` / `staging` / `production`). |

These keys are written into the generated `ios/` project at `expo prebuild`
time; the committed `ios/xcconfig/*.xcconfig` files carry only native
build-layer markers (`PPT_APP_ENV`, bundle id) and deliberately do **not**
duplicate the runtime values above, to avoid drift (see GH #1410).

### How `APP_ENV` selects the config file

`app.config.ts` resolves the environment in this order:

| `APP_ENV` value | Loads          |
| --------------- | -------------- |
| `development`   | `.env.development` |
| `staging`       | `.env.staging` |
| `production`    | `.env.production` |

If `APP_ENV` is unset, it defaults to `development` in `__DEV__` (Metro dev
server) and `production` in release builds. Set it inline on the command:

```bash
APP_ENV=staging expo start
```

### Variables

| Variable        | Required | Example (development)        | Description |
| --------------- | :------: | ---------------------------- | ----------- |
| `API_BASE_URL`  | yes      | `http://localhost:8080`      | REST base URL for `api-server`. |
| `WS_BASE_URL`   | yes      | `ws://localhost:8080`        | WebSocket base URL (derived from `API_BASE_URL` if omitted). |
| `ENVIRONMENT`   | yes      | `development`                | Logical environment label exposed to the app and iOS `Info.plist`. |
| `DEBUG_MODE`    | yes      | `true`                       | Enables verbose/debug behaviour; `false` in production. |
| `APP_LINK_HOST` | yes      | `app.ppt.example.com`        | HTTPS host for Universal Links (iOS) / App Links (Android). Drives `ios.associatedDomains`, the Android `https` intent filter, and the JS route matcher (`src/qrcode/universalLinks.ts`). The matching `/.well-known/apple-app-site-association` and `/.well-known/assetlinks.json` files must be served from this host. |

### Per-environment values

| Variable        | development             | staging                                | production |
| --------------- | ----------------------- | -------------------------------------- | ---------- |
| `API_BASE_URL`  | `http://localhost:8080` | `https://staging-api.ppt.example.com`  | `__SET_BY_CI__` (overridden by CI) |
| `WS_BASE_URL`   | `ws://localhost:8080`   | `wss://staging-api.ppt.example.com`    | `__SET_BY_CI__` (overridden by CI) |
| `ENVIRONMENT`   | `development`           | `staging`                              | `production` |
| `DEBUG_MODE`    | `true`                  | `true`                                 | `false` |
| `APP_LINK_HOST` | `app.ppt.example.com`   | `staging-app.ppt.example.com`          | `app.ppt.example.com` (override in CI) |

> **Production secrets:** `.env.production` intentionally ships the
> `__SET_BY_CI__` sentinel for `API_BASE_URL` / `WS_BASE_URL`. `app.config.ts`
> has a placeholder guard that **fails the build** if that sentinel reaches a
> real production build. CI must supply the real hosts via EAS project secrets:
>
> ```bash
> eas secret:create --scope project --name API_BASE_URL --value https://api.<prod-host>
> eas secret:create --scope project --name WS_BASE_URL  --value wss://api.<prod-host>
> ```
>
> These land in `process.env` at build time and are picked up by
> `app.config.ts`. See the `production` profile in [`eas.json`](./eas.json).

### First-time setup

The committed `.env.development`, `.env.staging`, and `.env.production` files
already carry the non-secret defaults above, so a local dev run works out of
the box. To customise values locally, copy the template and edit:

```bash
cp .env.example .env.development   # then edit API_BASE_URL etc. as needed
```

`.env.example` is the documented template; `.env.local` (if you create one) is
gitignored for machine-specific overrides.

## Running per environment

`pnpm`-friendly scripts are defined in [`package.json`](./package.json). Run
them from the repo root with `-F mobile` or from this directory directly.

| Goal | Command |
| ---- | ------- |
| Dev (default `development`) | `pnpm -F mobile start` |
| Staging (Metro dev server)  | `pnpm -F mobile start:staging` |
| Production (Metro dev server) | `pnpm -F mobile start:production` |
| Dev on Android device/emulator | `pnpm -F mobile android` |
| Staging on Android | `pnpm -F mobile android:staging` |
| Dev on iOS simulator | `pnpm -F mobile ios` |
| Staging on iOS | `pnpm -F mobile ios:staging` |

Any environment can also be selected by setting `APP_ENV` inline:

```bash
APP_ENV=staging expo start
APP_ENV=production expo start
```

> **Android localhost note:** when `API_BASE_URL` is unset in `__DEV__`, the app
> falls back to `http://10.0.2.2:8080` on Android (emulator → host loopback) and
> `http://localhost:8080` on iOS. See `getApiBaseUrl()` in `src/config/api.ts`.

### Release builds (EAS)

EAS build profiles in [`eas.json`](./eas.json) set `APP_ENV` for you:

```bash
pnpm -F mobile build:android:staging      # eas build --platform android --profile staging
pnpm -F mobile build:android:production    # eas build --platform android --profile production
pnpm -F mobile build:ios:staging
pnpm -F mobile build:ios:production
```

The `eas` CLI must be installed globally (`npm install -g eas-cli`) — it is
intentionally not a devDependency. See [`CLAUDE.md`](./CLAUDE.md) for details.
