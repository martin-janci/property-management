# bug-rn-app-icons-missing-png-assets-pr-1554

**Vector:** bug
**Score:** 3
**Source:** Issue #1670 | PR #1554
**Confidence:** high

## Hypothesis
PR #1554 ("RN app icon generation for all sizes", Coverage 85-2) shipped the icon-source SVG and a generator script but committed zero PNG outputs. The base `assets/images/icon.png` and `adaptive-icon.png` referenced by `app.config.ts:249,336` are absent on `dev`, and the existing `resolveIconVariant()` fallback only degrades *badged* variants down to the base — it never recovers when the base file itself is missing. `expo prebuild` / EAS Build will fail or ship with no icon. The smallest correct change is to run the generator and commit the produced PNGs, and add an un-mocked `fs.existsSync` test that asserts the base assets are physically present on disk so the gap stays closed.

## Evidence
- Issue #1670 (filed 2026-06-22 by `ppt-review-merged` skill) — full post-merge review with file:line pointers
- PR #1554 — `frontend/apps/mobile/assets/` contains only `.svg`, `.json`, `.md`, and `.sh` files; no `.png`
- `frontend/apps/mobile/app.config.ts:119-148` — `resolveIconVariant()` falls back among badged variants only
- `frontend/apps/mobile/app.config.ts:249, 336` — production wiring still emits `./assets/images/icon.png` + `adaptive-icon.png`
- `frontend/apps/mobile/app.config.icon.test.ts` — mocks `fs.existsSync` to always-true (structurally cannot catch the dev breakage)

## Files
- `scripts/generate-rn-app-icons.sh`
- `frontend/apps/mobile/app.config.ts`
- `frontend/apps/mobile/app.config.icon.test.ts`
- `frontend/apps/mobile/assets`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `cd frontend/apps/mobile && ls assets/images/icon.png assets/images/adaptive-icon.png` — both files are missing on `dev`.
2. `cd frontend/apps/mobile && pnpm dlx expo prebuild --platform android` (or run EAS Build) — expected: prebuild succeeds with icons in place; actual: fails to locate `./assets/images/icon.png` or generates an APK with no launcher icon.

## Suggested approach
1. Run `./scripts/generate-rn-app-icons.sh` against `frontend/apps/mobile/assets/images/icon-source.svg`.
2. Commit the generated PNGs: at minimum `assets/images/icon.png`, `assets/images/adaptive-icon.png`, plus the badged variants the script emits. Keep the existing `assets/ios-icons/AppIcon*.appiconset/Contents.json` unchanged unless Step 6 is taken.
3. In `app.config.icon.test.ts`, add a test (or extend `allAssetsPresent`) that calls real `fs.existsSync(path.resolve(__dirname, 'assets/images/icon.png'))` — un-mocked — and asserts truthy. This is the failing-on-main test (IG3).
4. Optional (low-priority follow-up to mention in the PR body, not this PR): replace the notification-icon alpha derivation in `generate-rn-app-icons.sh` (currently keying off `p{0,0}` top-left pixel) with a foreground-shape derivation.
5. Run `pnpm -F mobile typecheck && pnpm -F mobile lint && pnpm -F mobile test app.config.icon` to verify.
6. Optional cleanup: drop `assets/ios-icons/` if the bare-workflow iOS appiconsets are confirmed unused (Expo managed workflow derives every iOS size from `icon` at prebuild). Skip if uncertain.

## Alternatives considered
- **Generate PNGs in a CI step and commit on first build** — rejected because it adds a `dev`-vs-CI drift surface and the assets are static deliverables, not derived artifacts.
- **Switch to remote-hosted icons in app.config.ts** — rejected because Expo prebuild and the Android/iOS build pipelines expect local PNGs at the configured paths; remote URLs would force a wider redesign.

## Root-cause trace
1. Symptom: `dev` `frontend/apps/mobile/assets/images/` lacks `icon.png` + `adaptive-icon.png`; production build path is broken.
2. ← PR #1554's commit shipped the generator + SVG but no committed outputs (binary-push limitation hit during the merge).
3. ← `app.config.icon.test.ts` mocks `fs.existsSync` to always-true (`allAssetsPresent` is forced true regardless of disk state), so neither local CI nor `pnpm test` caught the missing files.
4. Origin: PR #1554 (merge 2026-06-21 by `martin-janci`), Coverage 85-2.

## Test plan
- [ ] Add un-mocked `fs.existsSync` assertion in `frontend/apps/mobile/app.config.icon.test.ts` against `icon.png` and `adaptive-icon.png` — should fail on `main` before the PNGs are committed, pass after.
- [ ] Optional: snapshot test that `app.config.ts`'s emitted `icon` / `adaptiveIcon.foregroundImage` paths point to files that resolve.
- [ ] Local command: `cd frontend && pnpm -F mobile test app.config.icon && pnpm -F mobile typecheck && pnpm -F mobile lint`

## Out of scope
- Re-keying the notification-icon alpha derivation in `generate-rn-app-icons.sh` (mentioned only as a constructive follow-up in #1670; track separately).
- Removing `assets/ios-icons/AppIcon*.appiconset` documents (also a separate cleanup).
- Any change to the Expo build profile in `eas.json` — wiring is already correct, the issue is solely the missing assets.

## After-merge
- Move this file to `plans/_archive/bug-rn-app-icons-missing-png-assets-pr-1554.md`
- Mark `backlog.json` row `bug-rn-app-icons-missing-png-assets-pr-1554` as `status: "done"`
