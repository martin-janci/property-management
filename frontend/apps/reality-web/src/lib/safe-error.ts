/**
 * Safe error logging helper.
 *
 * API-client (axios/fetch-wrapper) rejections embed the originating request —
 * including its body — on fields such as `config.data`, `request`, and
 * `response.config.data`. Logging the raw error object (e.g.
 * `console.error('…', error)`) therefore serializes those payloads to the
 * console, leaking secret-bearing request bodies: CRM `apiKey` / instance
 * credentials, feed-source credentials, realtor-invite tokens, etc.
 *
 * `toSafeLogMessage` extracts only the human-readable message string and never
 * returns the carrier object, so callers can log a diagnostic without leaking
 * the payload. This mirrors the app's existing `sanitize-*` family
 * (`sanitize-url`, `sanitize-css`, tenant-config) which keep untrusted or
 * sensitive values out of a dangerous sink.
 */
export function toSafeLogMessage(error: unknown): string {
  if (error instanceof Error) {
    // `name`/`message` are plain strings; the carrier object (and any
    // `config`/`request`/`response` it hangs off) is deliberately dropped.
    return error.name && error.name !== 'Error' ? `${error.name}: ${error.message}` : error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  return 'Unknown error';
}
