# App icon assets (Property Management mobile)

Epic 85 — Story 85.2: Build Configuration by Environment (app icon variants).

`app.config.ts` selects the launcher icon **per environment** (`APP_ENV`) so a
DEV or staging build is instantly distinguishable from production on the home
screen. The selection logic lives in `resolveIconVariant()` in `app.config.ts`;
this file documents the asset convention it expects.

## Required assets

All icons are **1024 x 1024 PNG**. Adaptive-icon foregrounds use a transparent
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

## Generating icons

All required PNG sizes are produced from the single source SVG by the
generation script:

```bash
# Requires: ImageMagick (brew install imagemagick / apt install imagemagick)
./scripts/generate-rn-app-icons.sh
# or pass a custom source:
./scripts/generate-rn-app-icons.sh path/to/icon.png
```

The script reads `assets/images/icon-source.svg` (committed, text/XML) and
writes:
- `assets/images/icon.png` + `adaptive-icon.png` — production (no badge)
- `assets/images/icon-dev.png` + `adaptive-icon-dev.png` — DEV badge (red)
- `assets/images/icon-staging.png` + `adaptive-icon-staging.png` — STG badge (amber)
- `assets/images/notification-icon.png` — 96x96 monochrome white
- `assets/ios-icons/AppIcon.appiconset/*.png` — all Apple Universal sizes (supplementary)
- `assets/ios-icons/AppIcon-Dev.appiconset/*.png` — DEV-badged iOS sizes
- `assets/ios-icons/AppIcon-Staging.appiconset/*.png` — STG-badged iOS sizes

Note: `ios/` is gitignored (Expo managed: `expo prebuild` generates the Xcode project).
The iOS icon sets land in `assets/ios-icons/` so they can be committed.

After running the script, commit the generated PNGs:

```bash
git add frontend/apps/mobile/assets/images/*.png
git add frontend/apps/mobile/assets/ios-icons/
git commit -m "chore(mobile): add generated app icons for all sizes"
```

**Why binaries are not committed by CI bots:** `mcp__github__push_files` only
accepts UTF-8 text. Binary PNGs must be committed by a maintainer running the
script locally.

## Badge convention

The badged variants composite a coloured ribbon across the bottom of the icon
reading `DEV` (red) or `STG` (amber). Keep the design content inside the
adaptive icon safe zone (centre 72% of canvas / 737x737 px) so it is not
clipped by round/squircle launcher masks on Android and iOS.

The un-badged production icon is the source; the badges are applied by the
generation script. Until the binary PNGs are committed (see above),
`app.config.ts` falls back to the un-badged production icon and logs a warning
at build time — builds never fail on a missing file.

## How selection works

`APP_ENV` is set per EAS build profile (see `eas.json`):

- `development` / `simulator` profiles -> `APP_ENV=development` -> DEV-badged icon
- `staging` profile -> `APP_ENV=staging` -> STG-badged icon
- `production` profile -> `APP_ENV=production` -> clean icon

Locally: `APP_ENV=staging expo start` previews the staging icon.
