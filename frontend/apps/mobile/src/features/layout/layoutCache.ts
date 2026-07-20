import AsyncStorage from '@react-native-async-storage/async-storage';
import { LAYOUT_CACHE_KEY } from '../../services/localCacheKeys';
import type { ResolvedScreen } from './types';

export async function readCachedLayout(screen: string): Promise<ResolvedScreen | null> {
  const key = LAYOUT_CACHE_KEY(screen);
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

export async function writeCachedLayout(screen: string, layout: ResolvedScreen): Promise<void> {
  const key = LAYOUT_CACHE_KEY(screen);
  try {
    await AsyncStorage.setItem(key, JSON.stringify(layout));
  } catch {
    // silently ignore write failures
  }
}
