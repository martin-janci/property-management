/**
 * XSS-safe serialization of a value that is about to be inlined into an
 * HTML `<script>` element (e.g. `window.__X__=<here>;`).
 *
 * `JSON.stringify` alone is NOT safe in this position: the JSON grammar
 * happily emits the literal characters `<`, `>`, `&`, U+2028 and U+2029,
 * every one of which is meaningful to the surrounding HTML/JS context:
 *
 *   - `</script>` inside any string value closes the script element early,
 *     handing the rest of the payload to the HTML parser as markup -- a full
 *     HTML-injection / XSS window whenever the serialized data contains a
 *     single attacker-influenced string (per-request tenant config, env
 *     vars, user content, ...).
 *   - U+2028 (LINE SEPARATOR) and U+2029 (PARAGRAPH SEPARATOR) are valid
 *     inside a JSON string but were, until ES2019, illegal *unescaped* in a
 *     JavaScript string literal -- an old engine throws a SyntaxError and
 *     the whole bootstrap script fails to parse.
 *
 * Escaping these to their `\uXXXX` forms keeps the output valid JSON *and*
 * valid JS while making it impossible to break out of the script context.
 * The result is byte-for-byte equal to the input for the common case (no
 * special chars), so there is no behavioural change for well-formed data.
 *
 * This is the single hardened primitive; callers that inline JSON into a
 * `<script>` (layout tenant bootstrap, `/env.js` runtime config) MUST route
 * through it rather than hand-rolling a partial `.replace(/</g, ...)`.
 */

// The five characters that must not appear raw inside an inline <script>,
// mapped to their `\uXXXX` escapes. Keys for the two Unicode separators are
// computed from code points so this source file never contains the invisible
// U+2028 / U+2029 characters themselves.
const SCRIPT_ESCAPES: Record<string, string> = {
  '<': '\\u003c',
  '>': '\\u003e',
  '&': '\\u0026',
  [String.fromCharCode(0x2028)]: '\\u2028',
  [String.fromCharCode(0x2029)]: '\\u2029',
};

// Matches exactly the keys of SCRIPT_ESCAPES. The two Unicode separators are
// written as `\u2028` / `\u2029` escape sequences (not raw characters), so
// this regex literal -- and the whole source file -- stays pure ASCII.
const SCRIPT_UNSAFE = /[<>&\u2028\u2029]/g;

/**
 * Serialize `value` to JSON safe for direct inlining inside `<script>`.
 * Returns a string; throws the same errors as `JSON.stringify` for cyclic
 * or otherwise non-serializable input.
 */
export function serializeForScript(value: unknown): string {
  return JSON.stringify(value).replace(SCRIPT_UNSAFE, (ch) => SCRIPT_ESCAPES[ch] ?? ch);
}
