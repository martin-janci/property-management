# Installing xcschemes

Copy *.xcscheme files into `iosApp.xcodeproj/xcshareddata/xcschemes/` on a macOS machine.

```bash
cd mobile-native/iosApp
mkdir -p iosApp.xcodeproj/xcshareddata/xcschemes
cp xcschemes/*.xcscheme iosApp.xcodeproj/xcshareddata/xcschemes/
```

Add three build configurations in Xcode (Project > Info > Configurations):
- Development -> Configurations/Development.xcconfig
- Staging     -> Configurations/Staging.xcconfig
- Production  -> Configurations/Production.xcconfig

Run `scripts/generate-ios-icons.sh <icon-1024.png>` to generate badged icon sets.
