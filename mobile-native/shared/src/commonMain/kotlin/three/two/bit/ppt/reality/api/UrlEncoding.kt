package three.two.bit.ppt.reality.api

import io.ktor.http.*

/**
 * Percent-encode a value before splicing it into a request path so that ids containing `/`, `?`,
 * `#`, or `..` cannot smuggle extra path segments or query parameters into the request
 * (path-injection). Deep-link / nav-sourced ids are untrusted.
 *
 * [encodeURLPathPart] percent-encodes everything that is not valid inside a single path segment.
 * This is the single shared implementation used by every reality repository — do not re-inline a
 * private copy (see #2195).
 */
internal fun String.asPathSegment(): String = encodeURLPathPart()
