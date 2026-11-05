// Shared, dependency-free helpers for the stable message-key-order guard
// (issue #3309). Used by src/i18n/message-key-order.test.ts.
//
// The node autofix/lint CLI at scripts/normalize-message-order.mjs keeps its
// own copy of this logic because it must run under plain node (no TS loader)
// to rewrite the files; keep the two in sync if you change the rules.

type Json = unknown;

const isPlainObject = (value: Json): value is Record<string, Json> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

/**
 * Flatten an object's keys into a depth-first, insertion-ordered list of
 * dotted leaf paths. Order reflects the JSON file's own key order.
 */
export function flattenKeys(obj: Record<string, Json>, prefix = ''): string[] {
  const out: string[] = [];
  for (const key of Object.keys(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    const value = obj[key];
    if (isPlainObject(value)) out.push(...flattenKeys(value, path));
    else out.push(path);
  }
  return out;
}

export interface OrderDivergence {
  index: number;
  expected: string;
  actual: string;
}

/**
 * Compare a locale's key order against the reference locale. Only keys the
 * locale actually has are considered, so a missing translation is a parity
 * problem (checked elsewhere), not an order problem. Returns the first
 * divergence, or null when the order is already canonical.
 */
export function findOrderDivergence(
  referenceKeys: string[],
  localeKeys: string[]
): OrderDivergence | null {
  const localeSet = new Set(localeKeys);
  const expected = referenceKeys.filter((key) => localeSet.has(key));
  const referenceSet = new Set(referenceKeys);
  const actual = localeKeys.filter((key) => referenceSet.has(key));
  const len = Math.min(expected.length, actual.length);
  for (let i = 0; i < len; i++) {
    if (expected[i] !== actual[i]) {
      return { index: i, expected: expected[i], actual: actual[i] };
    }
  }
  return null;
}
