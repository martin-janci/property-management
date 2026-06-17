/** Returns true when localStorage is accessible (guards against private-mode and SSR). */
export function isLocalStorageAvailable(): boolean {
  try {
    if (typeof window === 'undefined') return false;
    const key = '__storage_test__';
    window.localStorage.setItem(key, key);
    window.localStorage.removeItem(key);
    return true;
  } catch {
    return false;
  }
}
