# Installing xcschemes into iosApp.xcodeproj

These scheme files must be placed inside `iosApp.xcodeproj/xcshareddata/xcschemes/`
to be tracked in source control and shared with the team.

## One-time setup (run on a macOS machine with Xcode)

```bash
cd mobile-native/iosApp

# 1. Create the xcshareddata/xcschemes directory if it does not exist
mkdir -p iosApp.xcodeproj/xcshareddata/xcschemes

# 2. Copy all scheme files
cp xcschemes/*.xcscheme iosApp.xcodeproj/xcshareddata/xcschemes/

# 3. Open Xcode and add the three build configurations
#    (matching the xcconfig files) to the project:
#    - Development  (uses Configurations/Development.xcconfig)
#    - Staging      (uses Configurations/Staging.xcconfig)
#    - Production   (uses Configurations/Production.xcconfig)
#
#    In Xcode: Project > Info > Configurations > "+" button
#    Then in each configuration's row, set the xcconfig under iosApp target.

# 4. Commit the installed schemes
git add iosApp.xcodeproj/xcshareddata/xcschemes/
git commit -m "chore(ios): install dev/staging/prod xcschemes"
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
