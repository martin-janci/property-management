# security-mobile-rn-nfc-device-clock-expiry

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review mobile-rn 2026-08-02 (tier1d slice), verified in-run 2026-08-03
**Confidence:** high

## Hypothesis
`NFCAccessController.validateAccess` decides credential expiry on the mobile client by comparing `new Date(credential.validUntil)` against `now = new Date()` (the device clock). On any Android/iOS device without MDM time-lock — the default for BYOD residents — a user can roll the system clock back and an expired physical-access credential is accepted as valid; the granted decision is derived and dispatched entirely on the device, so the server never sees the bypass. The fix is to stop treating the local clock as an authority: the expiry decision must ride on a server-issued, per-session token (or a live challenge/response) whose validity the server checks against its own wall clock, and any local wall-clock check kept for offline-only paths must be paired with a monotonic-anchor / trusted-time source so a rewound device clock cannot resurrect an expired credential.

## Evidence
- `frontend/apps/mobile/src/nfc/NFCAccessController.ts:274` — `if (new Date(credential.validUntil) < now)` uses `now = new Date()` (line 250) which reads the OS clock verbatim; there is no monotonic or trusted-time anchor anywhere in the file.
- `frontend/apps/mobile/src/nfc/NFCAccessController.ts:249-346` — the full `validateAccess` decision (status → expiry → access-point membership → time-restriction) all runs client-side and returns `granted: true` in-process; the resulting `AccessAttemptResult` is delivered to the reader without a server round-trip.
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts` (321 lines) — pairs the controller and also lacks any server-side freshness check; the credential blob is cached and re-used for the entire local session.
- (test-gap sibling) both files ship with zero test coverage, so no existing regression pins the current behaviour (`test-gap-mobile-rn-nfc-modules-untested` in `backlog.json`).

## Files
- `frontend/apps/mobile/src/nfc/NFCAccessController.ts`
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (security fix, backward-tracing required)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security-visible surface; expect controversy on the offline-mode UX trade-off)

Mode: cloud-ok

Rationale: no ADB or browser required — the fix is TypeScript changes + Jest unit tests around the pure `validateAccess` decision; a physical NFC round-trip is out of scope per *Out of scope*.

## Repro steps
1. Provision an active NFC credential for a test resident with `validUntil = 2026-01-01T00:00:00Z` and the current device clock set to 2026-08-03.
2. Confirm `validateAccess` denies with `denialReason: 'credential_expired'` (baseline, current behaviour under a correct clock).
3. Roll the device system clock back to 2025-12-15T12:00:00Z (Settings → General → Date & Time → disable Automatic → set manually).
4. Re-run `validateAccess` for the same credential + access point.
5. Expected: still denied — expiry decision must not depend on user-controlled clock. Actual on `dev` today: **granted** — the credential is accepted because `new Date('2026-01-01') < new Date()` is false when the OS clock says 2025-12-15.

## Suggested approach
1. In `NFCAccessController.ts`, extract the expiry check into a pure `isCredentialExpired(credential, now)` helper. Do **not** delete the existing wall-clock comparison — it becomes the fallback path for the offline-only branch, but it will no longer be the sole authority.
2. Add a `TrustedNow` interface (`{ now(): Date; source: 'server' | 'monotonic-anchor' | 'device-fallback' }`) resolved at controller construction time. Wire two production implementations:
   - **`ServerTrustedNow`** — reads the last-seen server timestamp from the same secure store as the credential (updated on every successful token refresh in `hooks/useApi`). Returns `max(server_time + monotonic_delta_since_last_sync, device_clock)`; source = `'server'` on fresh sync, `'monotonic-anchor'` when the delta was computed via `performance.now()`-style monotonic clock.
   - **`DeviceFallbackNow`** — `new Date()`. Only used when the credential's `offline_grace_minutes` window is still open (a field the server includes when it issues the credential); source = `'device-fallback'`.
3. `validateAccess` calls `isCredentialExpired(credential, trustedNow.now())`. If `source == 'device-fallback'` **and** the credential is within its declared `offline_grace_minutes`, allow the current behaviour but mark the `AccessAttemptResult` with a new `trustSource: 'device-fallback'` field so the reader/audit downstream can log the reduced assurance.
4. In `NFCCredentialManager.ts`, persist the server-issued `issued_at` and `offline_grace_minutes` alongside the credential blob; expose a `refreshTrustedTime(): Promise<void>` that the app calls whenever it comes online (piggyback on the token-refresh path fixed by `code-review-mobile-rn-useapi-no-refresh-retry`).
5. Emit an audit event locally (queued for upload) whenever `granted && trustSource == 'device-fallback'`, so a rolled-back-clock incident is at least visible after the device reconnects.
6. Add a `docs/security/nfc-credential-freshness.md` note describing the trust hierarchy so downstream integrations (the reality-server side) know the guarantees.

## Alternatives considered
- **Server-side check on every tap** — rejected because NFC access must work when the device has no connectivity (garage doors in basements, foreign roaming); a full server round-trip on every tap would break the primary use-case. The `TrustedNow + offline_grace_minutes` design keeps offline access working within a bounded, server-issued window.
- **Rely on `Date.now()` + Android `SystemClock.elapsedRealtime()` monotonic-only** — rejected because a device that boots after a rolled-back clock still starts monotonic at zero; a monotonic anchor is only useful *since the last trusted sync*. The chosen design pairs monotonic delta with a server-issued anchor, which is the standard mobile-security pattern.

## Root-cause trace
1. Symptom: rolled-back device clock accepts an expired NFC credential; `AccessAttemptResult.granted == true` for a credential whose `validUntil` is in the past by server time.
2. ← `frontend/apps/mobile/src/nfc/NFCAccessController.ts:274` — expiry comparison `new Date(credential.validUntil) < now` uses `now = new Date()` (line 250).
3. ← `frontend/apps/mobile/src/nfc/NFCAccessController.ts:250` — `const now = new Date();` binds directly to `Date` constructor, no injected clock, no monotonic anchor.
4. ← `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts` — credential is issued once and cached; no `issued_at` / `offline_grace_minutes` server field is persisted, so there is nothing to constrain the fallback path even if one existed.
5. Origin: the NFC module was authored as a client-only flow (no PR # traced during this run — the file's first commit predates the current cursor window; the earliest reference in the codebase is the initial NFC feature branch that landed the file). The latent assumption "the device is trusted infrastructure" was never revisited when the module was wired into the shipping mobile app.

## Test plan
- [ ] `frontend/apps/mobile/src/nfc/__tests__/NFCAccessController.expiry.test.ts` — unit-tests the extracted `isCredentialExpired(credential, now)` helper with a fixed clock argument; covers (a) valid credential + `now < validUntil` → not expired, (b) expired credential + `now > validUntil` → expired, (c) valid credential + rolled-back device clock + expired grace window → **expired** (this is the failing-on-main regression).
- [ ] `frontend/apps/mobile/src/nfc/__tests__/NFCAccessController.trusted-now.test.ts` — verifies `validateAccess` denies with `trustSource: 'server'` when a `ServerTrustedNow` stub returns a time past `validUntil`, regardless of what `new Date()` returns.
- [ ] `frontend/apps/mobile/src/nfc/__tests__/NFCAccessController.offline-grace.test.ts` — verifies the `DeviceFallbackNow` path still grants while inside `offline_grace_minutes` and denies once outside it, and stamps `trustSource: 'device-fallback'` on the result.
- [ ] Command: `pnpm --filter mobile test src/nfc`

## Out of scope
- End-to-end NFC hardware testing (requires physical reader + credential card; the plan pins the decision logic in unit tests). File a follow-up if a hardware smoke test is desired.
- Backend / reality-server changes to the credential-issuance API. The `offline_grace_minutes` field is documented as an expected server contract in the new `docs/security/nfc-credential-freshness.md` note; wiring it into the server is a paired backend task tracked separately once this plan lands.
- Fixing the sibling `test-gap-mobile-rn-nfc-modules-untested` beyond the tests explicitly listed above. The general coverage gap in `NFCCredentialManager.ts` is not addressed here; this plan only pins the expiry-decision surface.

## After-merge
- Move this file to `plans/_archive/security-mobile-rn-nfc-device-clock-expiry.md`
- Mark the matching `backlog.json` row as `status: "done"`
