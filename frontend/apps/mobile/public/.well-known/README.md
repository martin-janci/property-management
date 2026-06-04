# Universal Links / App Links well-known files

gap-85-3 — these two files make tapping an `https://<APP_LINK_HOST>/<path>`
link open the **PPT Management** app (`three.two.bit.ppt.management`) directly
instead of the browser.

## What lives here

| File | Platform | Verifies |
|------|----------|----------|
| `apple-app-site-association` | iOS (Universal Links) | App bundle ↔ domain association |
| `assetlinks.json` | Android (App Links) | Signing cert ↔ domain association |

## Hosting requirement (NOT served by the app)

These files are **served by the web host** that owns `APP_LINK_HOST`
(default convention: `app.ppt.example.com`; real value injected per-env via
`APP_LINK_HOST` in `.env.<env>` / EAS). They must be reachable at, with
`Content-Type: application/json` and **no redirect**:

```
https://<APP_LINK_HOST>/.well-known/apple-app-site-association
https://<APP_LINK_HOST>/.well-known/assetlinks.json
```

The `apple-app-site-association` file MUST be served **without** a `.json`
extension (Apple requirement). Deploy these by copying this directory to the
web host's document root (`reality-web` / api-server static, or the CDN that
fronts `APP_LINK_HOST`).

## Placeholders to replace before production

Both files ship with build-time placeholders that CI / release must replace:

- `__APPLE_TEAM_ID__` — the Apple Developer Team ID (10 chars, e.g. `A1B2C3D4E5`).
  Find it in App Store Connect → Membership, or `eas credentials -p ios`.
- `__ANDROID_SIGNING_SHA256__` — the SHA-256 fingerprint of the **upload /
  signing** keystore, colon-separated upper-hex
  (e.g. `14:6D:E9:83:C5:73:06:50:D8:EE:B9:95:2F:34:FC:64:16:A0:83:42:E6:1D:BE:A8:8A:04:96:B2:3F:CF:44:E5`).
  For EAS-managed signing: `eas credentials -p android` → "Download / view"
  shows the SHA-256. For Play App Signing: Play Console → App integrity →
  "App signing key certificate" SHA-256.

> Android `autoVerify` and iOS associated-domain verification will silently
> fail (link opens the browser) if these placeholders are not replaced or the
> files are not served exactly as above.
