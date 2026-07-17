import AsyncStorage from '@react-native-async-storage/async-storage';
import {
  CACHE_PREFIX,
  LAST_SYNC_KEY,
  QUEUE_KEY,
  WIDGET_CONFIG_KEY,
  WIDGET_DATA_KEY,
} from './localCacheKeys';

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
    const keys = await AsyncStorage.getAllKeys();
    const toRemove = keys.filter(
      (key) =>
        key.startsWith(CACHE_PREFIX) ||
        key === QUEUE_KEY ||
        key === LAST_SYNC_KEY ||
        key === WIDGET_DATA_KEY ||
        key === WIDGET_CONFIG_KEY
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
}
