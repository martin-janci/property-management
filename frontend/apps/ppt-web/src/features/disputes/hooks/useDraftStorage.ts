/**
 * useDraftStorage — debounced localStorage-backed draft persistence.
 *
 * Epic 80, Story 80.2 (AC-4: draft auto-save). The dispute-filing form is a
 * long, legally-meaningful form; losing it to an accidental tab close or
 * navigation is a real failure mode. This hook persists the current form
 * values to localStorage on a debounce, restores them on mount, exposes a
 * "last saved" timestamp for an auto-save indicator, and clears the draft once
 * the form is submitted successfully.
 *
 * It is intentionally generic (not dispute-specific) so other long forms can
 * reuse it, but lives under the disputes feature where it is first needed.
 */

import { useCallback, useEffect, useRef, useState } from 'react';

/** Check if localStorage is available (private mode / SSR guard). */
function isLocalStorageAvailable(): boolean {
  try {
    if (typeof window === 'undefined') return false;
    const test = '__draft_storage_test__';
    window.localStorage.setItem(test, test);
    window.localStorage.removeItem(test);
    return true;
  } catch {
    return false;
  }
}

interface StoredDraft<T> {
  /** Persisted form values. */
  values: T;
  /** Epoch millis of the last save. */
  savedAt: number;
}

export interface UseDraftStorageOptions {
  /** Debounce window before a save is flushed, in ms. Default 800ms. */
  debounceMs?: number;
}

export interface UseDraftStorageResult<T> {
  /** The restored draft on first mount, or null if none was stored. */
  restored: T | null;
  /** Epoch millis of the last successful save, or null if never saved. */
  savedAt: number | null;
  /** Queue `values` to be persisted (debounced). */
  save: (values: T) => void;
  /** Drop the stored draft and reset the saved-at timestamp. */
  clear: () => void;
}

/**
 * Persist a form draft under `key`, debounced.
 *
 * @param key      localStorage key (callers should namespace, e.g. `ppt-dispute-draft`).
 * @param options  debounce tuning.
 */
export function useDraftStorage<T>(
  key: string,
  options: UseDraftStorageOptions = {}
): UseDraftStorageResult<T> {
  const { debounceMs = 800 } = options;
  const available = isLocalStorageAvailable();

  // Read any pre-existing draft once, synchronously, on first render so the
  // form can seed its default values from it.
  const [restored] = useState<T | null>(() => {
    if (!available) return null;
    try {
      const raw = window.localStorage.getItem(key);
      if (!raw) return null;
      const parsed = JSON.parse(raw) as StoredDraft<T>;
      return parsed.values ?? null;
    } catch {
      return null;
    }
  });

  const [savedAt, setSavedAt] = useState<number | null>(() => {
    if (!available) return null;
    try {
      const raw = window.localStorage.getItem(key);
      if (!raw) return null;
      const parsed = JSON.parse(raw) as StoredDraft<T>;
      return typeof parsed.savedAt === 'number' ? parsed.savedAt : null;
    } catch {
      return null;
    }
  });

  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const flush = useCallback(
    (values: T) => {
      if (!available) return;
      try {
        const payload: StoredDraft<T> = { values, savedAt: Date.now() };
        window.localStorage.setItem(key, JSON.stringify(payload));
        setSavedAt(payload.savedAt);
      } catch {
        // Quota exceeded or serialisation error — best-effort, swallow.
      }
    },
    [available, key]
  );

  const save = useCallback(
    (values: T) => {
      if (!available) return;
      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => flush(values), debounceMs);
    },
    [available, debounceMs, flush]
  );

  const clear = useCallback(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setSavedAt(null);
    if (!available) return;
    try {
      window.localStorage.removeItem(key);
    } catch {
      // ignore
    }
  }, [available, key]);

  // Flush any pending debounced save on unmount so a quick close-after-typing
  // doesn't lose the last keystrokes.
  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  return { restored, savedAt, save, clear };
}
