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
 *   - The central {@link queryKeys} factory (`lib/queryKeys.ts`).
 *   - The app's feature-local `*Keys` factories.
 *   - The shared `@ppt/api-client` `*Keys` factories consumed by ppt-web for
 *     auth-/tenant-scoped data. (The full api-client roster is intentionally
 *     NOT derived — most of it is used by other apps and is out of scope for
 *     ppt-web's logout purge.)
 */

import { messagingKeys, meterKeys, notificationTriggerKeys, reportKeys } from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import { aiChatKeys } from '../features/ai-chat/hooks/useAiChat';
import { notificationAnalyticsKeys } from '../features/notification-analytics/hooks/useNotificationAnalytics';
import { predictiveKeys } from '../features/predictive-maintenance/hooks/usePredictiveMaintenance';
import { sentimentKeys } from '../features/sentiment/hooks/useSentiment';
import { AUTHED_QUERY_KEY_ROOTS, queryKeys } from './queryKeys';

/**
 * Every query-key factory exposes an `all` tuple whose first segment is the
 * root key `removeQueries` prefix-matches against on logout.
 */
type KeyFactory = { readonly all: readonly [string, ...unknown[]] };

const rootOf = (factory: KeyFactory): string => factory.all[0];

const allowlist = new Set<string>(AUTHED_QUERY_KEY_ROOTS);

describe('AUTHED_QUERY_KEY_ROOTS logout-purge coverage', () => {
  // The central factory is the primary future-addition vector: adding a new
  // resource block to `queryKeys` must not silently escape the purge.
  describe('central queryKeys factory roots', () => {
    it.each(
      Object.entries(queryKeys).map(([name, factory]) => [name, rootOf(factory as KeyFactory)])
    )('covers queryKeys.%s (root "%s")', (_name, root) => {
      expect(allowlist).toContain(root);
    });
  });

  // Feature-local key factories owned by ppt-web (analytics dashboards, AI chat).
  describe('feature-local key factories', () => {
    const featureLocalFactories: Record<string, KeyFactory> = {
      aiChatKeys,
      notificationAnalyticsKeys,
      predictiveKeys,
      sentimentKeys,
    };

    it.each(Object.entries(featureLocalFactories).map(([name, f]) => [name, rootOf(f)]))(
      'covers %s (root "%s")',
      (_name, root) => {
        expect(allowlist).toContain(root);
      }
    );
  });

  // Shared @ppt/api-client factories consumed by ppt-web for auth-/tenant-scoped
  // data. `notificationTriggerKeys` is the exact root PR #2650 missed.
  describe('consumed @ppt/api-client key factories', () => {
    const consumedApiClientFactories: Record<string, KeyFactory> = {
      meterKeys,
      messagingKeys,
      notificationTriggerKeys,
      reportKeys,
    };

    it.each(Object.entries(consumedApiClientFactories).map(([name, f]) => [name, rootOf(f)]))(
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
