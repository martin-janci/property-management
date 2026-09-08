import AsyncStorage from '@react-native-async-storage/async-storage';
import { NFCCredentialManager } from '../nfc/NFCCredentialManager';
import { CACHE_PREFIX, LAYOUT_PREFIX, TENANT_SCOPED_EXACT_KEYS } from './localCacheKeys';

/**
 * Purge every AsyncStorage namespace that holds tenant-scoped local data
 * (issue #2399, follow-up to #2361).
 *
 * `AuthContext.login`/`logout` already call `queryClient.clear()`, but that
 * only wipes TanStack Query's **in-memory** cache. The two AsyncStorage-backed
 * caches below outlive the process, so without this a prior org's data survives
 * a login/logout and even an app restart:
 *   - `ppt_cache_*` + `ppt_offline_queue` + `ppt_last_sync` (`useOfflineSupport`)
 *   - `@ppt/widget_data` + `@ppt/widget_configs` (`WidgetDataProvider`)
 *   - `ppt_layout_*` (`useDashboardLayout` — the resolved screen layout embeds
 *     the org's tenant override, so a stale entry would let a prior tenant's
 *     dashboard customization activate for the next account on a shared device;
 *     issue #2486). The key is a function of a dynamic `scopeId`/`screen`, so it
 *     is swept by prefix rather than matched against a fixed key.
 *   - the fixed feature-cache keys in `TENANT_SCOPED_EXACT_KEYS` — NFC access
 *     log, QR scan history, offline feedback drafts + pending-flush queue, and
 *     FAQ votes. Without these a prior tenant's physical-access/scan history
 *     survives a handoff, and queued offline feedback can flush under the next
 *     user's token (issue #2947, follow-up to #2361/#2399).
 *   - NFC building-access credential material, via
 *     `NFCCredentialManager.clearAllLocalCredentials()`. Unlike everything
 *     above this lives in `expo-secure-store` (encrypted at rest), not
 *     AsyncStorage, plus a legacy unencrypted AsyncStorage migration blob — so
 *     the AsyncStorage sweep alone left the PRIOR tenant's actual credentials
 *     readable, and the next tenant could adopt them before their own fetch
 *     (issue #2953, follow-up to #2947). It runs in its own best-effort block
 *     so an AsyncStorage failure above cannot skip the more sensitive purge.
 *
 * Server RLS still prevents an actual data breach on refetch, so the exposure
 * this closes is stale-display + queued-write misattribution (a prior user's
 * queued fault/meter writes replaying under the next user's token), not a
 * server leak.
 *
 * Call it right where the query cache is cleared on a session change:
 *   `await resetLocalData(); queryClient.clear();`
 *
 * Deliberately NOT called on biometric unlock — that restores the *same*
 * stored user's session, so its cache is still valid and should be kept.
 *
 * Best-effort: storage failures are logged and swallowed so a purge hiccup can
 * never wedge login/logout.
 */
export async function resetLocalData(): Promise<void> {
  try {
    const exact = new Set<string>(TENANT_SCOPED_EXACT_KEYS);
    const keys = await AsyncStorage.getAllKeys();
    const toRemove = keys.filter(
      (key) => key.startsWith(CACHE_PREFIX) || key.startsWith(LAYOUT_PREFIX) || exact.has(key)
    );

    if (toRemove.length > 0) {
      // `removeMany` is this project's async-storage v3 batch-delete (see the
      // scoped-storage type stub in `types/expo-modules.d.ts`, matching
      // `useOfflineSupport.clearCache`).
      await AsyncStorage.removeMany(toRemove);
    }
  } catch (error) {
    console.error('Failed to reset local data:', error);
  }

  // NFC credential material lives in SecureStore (+ a legacy AsyncStorage blob),
  // which the sweep above does not cover. Purge it in a separate best-effort
  // block so a failure in the AsyncStorage sweep can never leave the more
  // sensitive building-access credentials behind (issue #2953).
  try {
    await NFCCredentialManager.clearAllLocalCredentials();
  } catch (error) {
    console.error('Failed to purge NFC credentials:', error);
  }
}
