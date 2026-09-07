/// <reference types="vitest/globals" />
/**
 * Enforcement test for the logout cache-purge allowlist.
 *
 * `AUTHED_QUERY_KEY_ROOTS` (see {@link ./queryKeys}) is the list `AuthContext`
 * iterates on logout to `removeQueries({ queryKey: [root] })` — purging every
 * tenant-/user-scoped subtree so cached data never leaks into the next session
 * on a shared workstation. Historically it was a hand-maintained list with no
 * compile/test enforcement, so a newly-added query-key factory root could ship
 * uncovered and leak across sessions (Issue #712; PR #2650 missed
 * `notification-triggers`).
 *
 * This test closes that gap by *deriving* the set of roots from the query-key
 * factories themselves and asserting each is present in the allowlist. Adding a
 * new resource to any covered factory therefore fails here until its root is
 * also added to `AUTHED_QUERY_KEY_ROOTS` — the addition is caught automatically
 * instead of relying on a reviewer to remember.
 *
 * Coverage is scoped to the factories ppt-web actually owns or consumes:
 *   - The central {@link queryKeys} factory (`lib/queryKeys.ts`) — derived from
 *     `Object.entries(queryKeys)`.
 *   - The app's feature-local `*Keys` factories — auto-discovered from every
 *     `features/**\/hooks/*.{ts,tsx}` module via `import.meta.glob`, so a new
 *     hook file exporting a `*Keys` factory can't be silently omitted the way a
 *     hand-maintained list would allow (Issue #2948, follow-up to PR #2943).
 *   - The shared `@ppt/api-client` `*Keys` factories consumed by ppt-web for
 *     auth-/tenant-scoped data. These stay an explicit list: the full
 *     api-client roster is used by other apps and is intentionally NOT derived
 *     — most of its roots are out of scope for ppt-web's logout purge.
 */

import { messagingKeys, meterKeys, notificationTriggerKeys, reportKeys } from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import { AUTHED_QUERY_KEY_ROOTS, queryKeys } from './queryKeys';

/**
 * Every query-key factory exposes an `all` tuple whose first segment is the
 * root key `removeQueries` prefix-matches against on logout.
 */
type KeyFactory = { readonly all: readonly [string, ...unknown[]] };

/**
 * A value is a root key-factory iff it carries a non-empty `all` tuple whose
 * first segment is the string root. This deliberately excludes nested
 * sub-factories (e.g. `attachmentKeys`, which spreads another factory's `all`
 * and has no `all` of its own — its root is already covered by its parent).
 */
const isKeyFactory = (val: unknown): val is KeyFactory => {
  if (typeof val !== 'object' || val === null || !('all' in val)) return false;
  const all = (val as { all: unknown }).all;
  return Array.isArray(all) && all.length > 0 && typeof all[0] === 'string';
};

/**
 * Extract the root segment, guarding the tuple shape so a future non-factory
 * entry produces a clear "not a key factory" failure rather than a confusing
 * `toContain(undefined)`.
 */
const rootOf = (name: string, factory: KeyFactory): string => {
  expect(Array.isArray(factory.all), `${name}.all must be an array`).toBe(true);
  expect(factory.all.length, `${name}.all must be a non-empty tuple`).toBeGreaterThan(0);
  expect(typeof factory.all[0], `${name}.all[0] (root) must be a string`).toBe('string');
  return factory.all[0];
};

const allowlist = new Set<string>(AUTHED_QUERY_KEY_ROOTS);

/**
 * Auto-discover every feature-local `*Keys` factory. `import.meta.glob` is
 * resolved statically by Vite/Vitest, so any new
 * `features/<x>/hooks/use<X>.ts` exporting a `<x>Keys` factory is picked up
 * automatically — no hand-maintained list to forget. Colocated test files are
 * excluded so their `describe`/`it` blocks don't execute on import.
 */
const featureModules = import.meta.glob<Record<string, unknown>>(
  ['../features/**/hooks/*.{ts,tsx}', '!../features/**/hooks/*.{test,spec}.{ts,tsx}'],
  { eager: true }
);

const featureLocalFactories: Record<string, KeyFactory> = {};
for (const mod of Object.values(featureModules)) {
  for (const [name, val] of Object.entries(mod)) {
    if (/Keys$/.test(name) && isKeyFactory(val)) {
      featureLocalFactories[name] = val;
    }
  }
}

describe('AUTHED_QUERY_KEY_ROOTS logout-purge coverage', () => {
  // The central factory is the primary future-addition vector: adding a new
  // resource block to `queryKeys` must not silently escape the purge.
  describe('central queryKeys factory roots', () => {
    it.each(
      Object.entries(queryKeys).map(([name, factory]) => [
        name,
        rootOf(name, factory as KeyFactory),
      ])
    )('covers queryKeys.%s (root "%s")', (_name, root) => {
      expect(allowlist).toContain(root);
    });
  });

  // Feature-local key factories owned by ppt-web (analytics dashboards, AI chat),
  // auto-discovered from `features/**/hooks/*` so a new one can't be omitted.
  describe('feature-local key factories (auto-discovered)', () => {
    // Guard against a silently-empty glob (e.g. a moved directory or a typo'd
    // pattern) vacuously passing with zero coverage.
    it('discovers at least one feature-local factory', () => {
      expect(Object.keys(featureModules).length).toBeGreaterThan(0);
      expect(Object.keys(featureLocalFactories).length).toBeGreaterThan(0);
    });

    it.each(Object.entries(featureLocalFactories).map(([name, f]) => [name, rootOf(name, f)]))(
      'covers %s (root "%s")',
      (_name, root) => {
        expect(allowlist).toContain(root);
      }
    );
  });

  // Shared @ppt/api-client factories consumed by ppt-web for auth-/tenant-scoped
  // data. Kept explicit — the full api-client roster is out of ppt-web's logout
  // scope. `notificationTriggerKeys` is the exact root PR #2650 missed.
  describe('consumed @ppt/api-client key factories', () => {
    const consumedApiClientFactories: Record<string, KeyFactory> = {
      meterKeys,
      messagingKeys,
      notificationTriggerKeys,
      reportKeys,
    };

    it.each(Object.entries(consumedApiClientFactories).map(([name, f]) => [name, rootOf(name, f)]))(
      'covers %s (root "%s")',
      (_name, root) => {
        expect(allowlist).toContain(root);
      }
    );
  });

  it('has no duplicate roots', () => {
    expect(allowlist.size).toBe(AUTHED_QUERY_KEY_ROOTS.length);
  });
});
