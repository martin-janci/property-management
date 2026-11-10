#!/usr/bin/env node
// Enforce a stable key order across the next-intl `messages/*.json` files.
//
// Background (issue #3309, follow-up to PR #3260): a formatter run reordered
// cs.json / sk.json wholesale (~9k lines of churn) instead of appending new
// keys, which buries the real translation diff and causes noisy conflicts.
//
// Canonical order = the key order of the reference locale (en.json). Every
// other locale must list the keys it shares with en in en's relative order,
// recursively, at every nesting level. New keys are appended to en.json (and
// to each locale) in the same spot, so edits append rather than reorder.
//
// Usage:
//   node scripts/normalize-message-order.mjs          # rewrite locales in place
//   node scripts/normalize-message-order.mjs --check   # exit 1 if any file is out of order
//
// The vitest regression guard lives at
// src/i18n/message-key-order.test.ts and shares the same rules.

import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const MESSAGES_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'messages');
export const REFERENCE_LOCALE = 'en';

const isPlainObject = (value) =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

/**
 * Flatten an object's keys into a depth-first, insertion-ordered list of
 * dotted paths (leaves only). Order reflects the JSON file's key order.
 */
export function flattenKeys(obj, prefix = '') {
  const out = [];
  for (const key of Object.keys(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    const value = obj[key];
    if (isPlainObject(value)) out.push(...flattenKeys(value, path));
    else out.push(path);
  }
  return out;
}

/**
 * Return a copy of `target` whose keys follow `reference`'s order, recursively.
 * Keys present in `target` but not in `reference` are appended last, keeping
 * their original relative order (so locale-only keys are never dropped).
 */
export function reorderToReference(reference, target) {
  if (!isPlainObject(reference) || !isPlainObject(target)) return target;
  const result = {};
  for (const key of Object.keys(reference)) {
    if (Object.hasOwn(target, key)) result[key] = reorderToReference(reference[key], target[key]);
  }
  for (const key of Object.keys(target)) {
    if (!Object.hasOwn(result, key)) result[key] = target[key];
  }
  return result;
}

/**
 * Compare a locale's key order against the reference. Only keys the locale
 * actually has are considered, so a locale missing a translation is a parity
 * problem (checked elsewhere), not an order problem. Returns the first
 * divergence, or null when the order is already canonical.
 */
export function findOrderDivergence(referenceKeys, localeKeys) {
  const localeSet = new Set(localeKeys);
  const expected = referenceKeys.filter((key) => localeSet.has(key));
  const referenceSet = new Set(referenceKeys);
  // Keys the locale has that en does not — ignored for order (they sort last).
  const actual = localeKeys.filter((key) => referenceSet.has(key));
  const len = Math.min(expected.length, actual.length);
  for (let i = 0; i < len; i++) {
    if (expected[i] !== actual[i]) {
      return { index: i, expected: expected[i], actual: actual[i] };
    }
  }
  return null;
}

function serialize(obj) {
  return `${JSON.stringify(obj, null, 2)}\n`;
}

function main() {
  const check = process.argv.includes('--check');
  const files = readdirSync(MESSAGES_DIR)
    .filter((f) => f.endsWith('.json'))
    .sort();
  const referenceFile = `${REFERENCE_LOCALE}.json`;
  if (!files.includes(referenceFile)) {
    console.error(`[messages] reference locale ${referenceFile} not found in ${MESSAGES_DIR}`);
    process.exit(1);
  }
  const reference = JSON.parse(readFileSync(join(MESSAGES_DIR, referenceFile), 'utf8'));
  const referenceKeys = flattenKeys(reference);

  let drift = 0;
  for (const file of files) {
    if (file === referenceFile) continue;
    const path = join(MESSAGES_DIR, file);
    const raw = readFileSync(path, 'utf8');
    const target = JSON.parse(raw);
    const divergence = findOrderDivergence(referenceKeys, flattenKeys(target));
    if (!divergence) continue;
    drift += 1;
    if (check) {
      console.error(
        `[messages] ${file} key order diverges from ${referenceFile} at index ${divergence.index}: ` +
          `expected "${divergence.expected}", found "${divergence.actual}"`
      );
      continue;
    }
    const reordered = serialize(reorderToReference(reference, target));
    if (reordered !== raw) {
      writeFileSync(path, reordered);
      console.log(`[messages] normalized ${file} to ${referenceFile} key order`);
    }
  }

  if (check && drift > 0) {
    console.error(
      `\n[messages] ${drift} file(s) out of order. Run: pnpm --filter @ppt/reality-web messages:normalize`
    );
    process.exit(1);
  }
  if (!check) {
    console.log(
      drift === 0
        ? '[messages] all locales already canonical'
        : `[messages] normalized ${drift} file(s)`
    );
  }
}

// Only run when invoked directly (not when imported by the vitest guard).
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main();
}
