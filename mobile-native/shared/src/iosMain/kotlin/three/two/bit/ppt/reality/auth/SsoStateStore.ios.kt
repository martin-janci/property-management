package three.two.bit.ppt.reality.auth

import platform.Foundation.NSUUID

/**
 * iOS CSRF nonce.
 *
 * The KMP [SsoStateStore] is not exercised by the iOS app — SwiftUI owns its own nonce in
 * `AuthManager.beginSsoFlow()` / `consumeSsoState(_:)` — but the shared module still needs an
 * `actual` for every target to compile. This mirrors the UUID-based fallback in
 * `AuthManager.makeSsoState()`: two v4 UUIDs concatenated and stripped to 64 hex chars.
 */
internal actual fun generateSsoNonce(): String {
    val a = NSUUID().UUIDString().replace("-", "").lowercase()
    val b = NSUUID().UUIDString().replace("-", "").lowercase()
    return a + b
}
