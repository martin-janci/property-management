# Generating iosApp.xcodeproj (and its schemes)

The Xcode project is **generated, not committed** — its single source of truth is
`mobile-native/iosApp/project.yml` (an [XcodeGen](https://github.com/yonaskolb/XcodeGen)
manifest, added in Epic 82 / Story 82.1). The three build configurations
(Development / Staging / Production, each wired to its `Configurations/*.xcconfig`)
and the three schemes below are all declared in that manifest, so there is no
longer any manual Xcode clicking to do.

## One-time setup (run on a macOS machine with Xcode)

```bash
# 1. Install XcodeGen (once)
brew install xcodegen

# 2. Generate iosApp.xcodeproj from project.yml
cd mobile-native/iosApp
xcodegen generate

# 3. Open it (schemes + configs are already wired)
open iosApp.xcodeproj
```

`scripts/build-ios.sh` runs `xcodegen generate` automatically when the project
is missing, so CI does not need this step performed by hand.

The generated `iosApp.xcodeproj` is git-ignored (see `iosApp/.gitignore`) — only
`project.yml` and the `.xcscheme` files in this directory are tracked. The
schemes are declared in `project.yml`; the raw `.xcscheme` files here are kept
as a reference / fallback for anyone who still drives the project by hand.

### Manual fallback (no XcodeGen)

If you cannot use XcodeGen, you can still hand-create the project and drop the
schemes in directly:

```bash
cd mobile-native/iosApp
mkdir -p iosApp.xcodeproj/xcshareddata/xcschemes
cp xcschemes/*.xcscheme iosApp.xcodeproj/xcshareddata/xcschemes/
# Then add the Development/Staging/Production configurations in
# Project > Info > Configurations and point each at its Configurations/*.xcconfig.
```

## App Icon sets

Run `scripts/generate-ios-icons.sh <your-1024px-source-icon.png>` to generate
the three icon sets (production / dev-DEV-badge / staging-STG-badge) into
`iosApp/Resources/Assets.xcassets/`.

After generating, in Xcode set the App Icon source per scheme:
- `RealityPortal-Dev`     → AppIcon-Dev
- `RealityPortal-Staging` → AppIcon-Staging
- `RealityPortal-Prod`    → AppIcon

## Build scripts (CI)

| Script | Purpose |
|--------|---------|
| `scripts/build-mobile.sh [env] [platform]` | Orchestrator — both platforms |
| `scripts/build-android.sh [env]`           | Android only |
| `scripts/build-ios.sh [env]`               | iOS only |

Environments: `development`, `staging`, `production`
