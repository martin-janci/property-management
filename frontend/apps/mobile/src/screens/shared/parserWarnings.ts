/**
 * Dev-only guards for the defensive API-response parsers (issue #2129).
 *
 * The mobile screens consume api-server responses without a generated fetch
 * client and validate them defensively: malformed rows are dropped and bad
 * top-level shapes degrade to an empty state instead of crashing. That
 * resilience has a downside — a backend serde-naming or shape drift would
 * silently render as "no data" in the field. These helpers make such a
 * contract regression loud in development builds (`__DEV__`) while staying
 * silent in production.
 */

function warn(parser: string, detail: string): void {
  console.warn(
    `[api-parser] ${parser}: ${detail} — possible backend contract drift (serde naming or shape change).`
  );
}

/**
 * Warn when a list parser degraded a non-empty payload to an empty list —
 * either the top-level shape was unrecognized (`candidates === null`) or
 * every candidate row was dropped by row validation.
 */
export function warnIfListDegraded(
  parser: string,
  input: unknown,
  candidates: readonly unknown[] | null,
  keptCount: number
): void {
  if (!__DEV__ || input == null) return;
  if (candidates === null) {
    warn(parser, 'unrecognized top-level response shape');
  } else if (candidates.length > 0 && keptCount === 0) {
    warn(parser, `dropped all ${candidates.length} candidate rows`);
  }
}

/** Warn when a detail parser returned `null` for a non-null payload. */
export function warnIfParseFailed(parser: string, input: unknown, result: unknown): void {
  if (__DEV__ && input != null && result === null) {
    warn(parser, 'unrecognized response shape');
  }
}
