# App icon assets (Property Management mobile)

Epic 85 — Story 85.2: Build Configuration by Environment (app icon variants).

`app.config.ts` selects the launcher icon **per environment** (`APP_ENV`) so a
DEV or staging build is instantly distinguishable from production on the home
screen. The selection logic lives in `resolveIconVariant()` in `app.config.ts`;
this file documents the asset convention it expects.

## Required assets

All icons are **1024 × 1024 PNG**. Adaptive-icon foregrounds use a transparent
background (Android draws the `#1A73E8` background layer behind them).

| Environment   | `APP_ENV`     | Launcher icon (`icon`)          | Android adaptive foreground            | Badge |
| ------------- | ------------- | ------------------------------- | -------------------------------------- | ----- |
| Production    | `production`  | `icon.png`                      | `adaptive-icon.png`                    | none  |
| Staging       | `staging`     | `icon-staging.png`              | `adaptive-icon-staging.png`            | `STG` |
| Development   | `development` | `icon-dev.png`                  | `adaptive-icon-dev.png`                | `DEV` |

Plus the push-notification icon referenced by the `expo-notifications` plugin:

| Purpose            | File                    |
| ------------------ | ----------------------- |
| Notification icon  | `notification-icon.png` |

## Badge convention

The badged variants are the **production** icon with a small ribbon/banner in
the lower portion reading `DEV` or `STG`. Keep the badge inside the adaptive
icon safe zone (centre 66% diameter) so it is not clipped by round/squircle
masks on Android.

These are **design assets** — produce them in the design tool and drop them
here. They are intentionally not generated in code (the repo does not commit
binary icon assets in this branch). Until the badged variants are added,
`app.config.ts` falls back to the un-badged production icon and logs a warning
at build time, so builds never fail on a missing file.

## How selection works

`APP_ENV` is set per EAS build profile (see `eas.json`):

- `development` / `simulator` profiles → `APP_ENV=development` → DEV-badged icon
- `staging` profile → `APP_ENV=staging` → STG-badged icon
- `production` profile → `APP_ENV=production` → clean icon

Locally: `APP_ENV=staging expo start` previews the staging icon.
