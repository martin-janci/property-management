package three.two.bit.ppt.reality.util

/**
 * Heuristic check for "the server couldn't be reached" failures — connection refused, DNS
 * resolution failure, socket/connect timeouts, generic IO errors.
 *
 * Ktor surfaces these as a spread of platform-specific exception types (`ConnectException`,
 * `UnresolvedAddressException`, `ConnectTimeoutException`, `SocketTimeoutException`, `IOException`,
 * …), so rather than importing each per-platform type we match on class name and message. The UI
 * uses this to swap the raw technical exception text for friendly copy.
 */
fun Throwable.isNetworkError(): Boolean {
    var cause: Throwable? = this
    while (cause != null) {
        val name = cause::class.simpleName.orEmpty()
        val msg = cause.message.orEmpty()
        if (NETWORK_EXCEPTION_NAMES.any { name.contains(it, ignoreCase = true) }) return true
        if (NETWORK_MESSAGE_HINTS.any { msg.contains(it, ignoreCase = true) }) return true
        cause = cause.cause?.takeIf { it !== cause }
    }
    return false
}

private val NETWORK_EXCEPTION_NAMES =
    listOf(
        "ConnectException",
        "ConnectTimeoutException",
        "SocketTimeoutException",
        "HttpRequestTimeoutException",
        "UnresolvedAddressException",
        "UnknownHostException",
        "IOException",
    )

private val NETWORK_MESSAGE_HINTS =
    listOf(
        "failed to connect",
        "connection refused",
        "unable to resolve host",
        "network is unreachable",
        "no route to host",
        "timeout",
    )
