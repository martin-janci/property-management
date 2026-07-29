package three.two.bit.ppt.reality.auth

import java.security.SecureRandom

/** Cryptographically-secure RNG, seeded once per process. */
private val secureRandom = SecureRandom()

/**
 * Android CSRF nonce: 32 random bytes from [SecureRandom] rendered as a 64-char lowercase hex
 * string — matching the iOS `AuthManager.makeSsoState()` output shape.
 */
internal actual fun generateSsoNonce(): String {
    val bytes = ByteArray(32)
    secureRandom.nextBytes(bytes)
    return buildString(bytes.size * 2) {
        for (b in bytes) {
            val v = b.toInt() and 0xff
            append("0123456789abcdef"[v ushr 4])
            append("0123456789abcdef"[v and 0x0f])
        }
    }
}
