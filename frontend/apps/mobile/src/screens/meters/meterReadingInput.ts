/**
 * Meter-reading input sanitization + validation
 * (code-review-mobile-rn-meter-reading-no-numeric-validation).
 *
 * `keyboardType="decimal-pad"` is only a soft-keyboard *hint*: Android soft
 * keyboards, hardware/Bluetooth keyboards, autofill and clipboard paste can
 * all still put arbitrary text into a `TextInput`. Before this guard the
 * MeterReadingScreen submit handler only checked `reading.trim()` truthiness,
 * so a value like "abc", "1.2.3" or "12,,3" was queued verbatim to
 * `POST /api/v1/meter-readings` — a silent write of a garbage reading.
 *
 * These two pure helpers keep the invariant in one testable place:
 *  - `normalizeReadingInput` runs on every keystroke, so only characters that
 *    can form a single decimal number ever reach component state.
 *  - `isValidReading` is the submit-time guard: it rejects empty / malformed /
 *    non-finite / negative values before anything is queued.
 */

/**
 * Strip everything that cannot be part of a single non-negative decimal number
 * from raw `TextInput` text.
 *
 * - Comma (the decimal separator in sk/cs/de/hu/pl locales) is normalized to a
 *   dot so the value parses with `Number.parseFloat` and matches the API
 *   contract, which expects a dot-decimal string.
 * - Every non-digit, non-separator character is dropped.
 * - Only the first decimal separator is kept; later ones are dropped so
 *   "1.2.3" collapses to "1.23" rather than an unparseable string.
 */
export function normalizeReadingInput(raw: string): string {
  if (!raw) {
    return '';
  }

  let sawSeparator = false;
  let out = '';

  for (const ch of raw) {
    if (ch >= '0' && ch <= '9') {
      out += ch;
      continue;
    }
    if ((ch === '.' || ch === ',') && !sawSeparator) {
      sawSeparator = true;
      out += '.';
    }
    // anything else (letters, whitespace, symbols, extra separators) is dropped
  }

  return out;
}

/**
 * Submit-time guard: true only when `value` is a non-empty string that parses
 * to a finite, non-negative number. A bare "." (or "") is not a reading.
 */
export function isValidReading(value: string): boolean {
  const trimmed = value.trim();

  // Strict shape check: digits with at most one decimal point (e.g. "42",
  // "42.", "42.4", ".5"). We deliberately do NOT use `Number.parseFloat` as
  // the acceptance test — it leniently parses "12abc" -> 12, which is exactly
  // the silent-garbage class this guard exists to reject.
  if (!/^(?:\d+\.?\d*|\.\d+)$/.test(trimmed)) {
    return false;
  }

  const parsed = Number.parseFloat(trimmed);
  return Number.isFinite(parsed) && parsed >= 0;
}
