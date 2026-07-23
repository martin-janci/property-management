import AsyncStorage from '@react-native-async-storage/async-storage';
import { LAYOUT_CACHE_KEY } from '../../services/localCacheKeys';
import type { ResolvedScreen } from './types';

/**
 * Read the persisted layout for `screen`, scoped to the logged-in user/org
 * `scopeId`. The resolved layout embeds the org's tenant override, so the key
 * is tenant-scoped: a different `scopeId` produces a different key and can
 * never read a foreign tenant's cached layout (issue #2486). The shape-guard
 * below still cannot detect cross-tenant reuse on its own — scoping the key is
 * what closes that gap.
 */
export async function readCachedLayout(
  scopeId: string,
  screen: string
): Promise<ResolvedScreen | null> {
  const key = LAYOUT_CACHE_KEY(scopeId, screen);
  try {
    const raw = await AsyncStorage.getItem(key);
    if (!raw) return null;
    let parsed: unknown;
    try {
      parsed = JSON.parse(raw);
    } catch {
      await AsyncStorage.removeItem(key);
      return null;
    }
    if (
      !parsed ||
      typeof parsed !== 'object' ||
      (parsed as ResolvedScreen).screen !== screen ||
      !Array.isArray((parsed as ResolvedScreen).sections)
    ) {
      await AsyncStorage.removeItem(key);
      return null;
    }
    return parsed as ResolvedScreen;
  } catch {
    return null;
  }
}

export async function writeCachedLayout(
  scopeId: string,
  screen: string,
  layout: ResolvedScreen
): Promise<void> {
  const key = LAYOUT_CACHE_KEY(scopeId, screen);
  try {
    await AsyncStorage.setItem(key, JSON.stringify(layout));
  } catch {
    // silently ignore write failures
  }
}
