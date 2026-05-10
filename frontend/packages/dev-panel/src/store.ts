// frontend/packages/dev-panel/src/store.ts
export type ApiMode = 'local' | 'worktree' | 'mock';

const KEY = 'ppt-dev-panel-mode';

/** Type-narrowing predicate for env-supplied or user-supplied mode strings. */
export function isApiMode(v: unknown): v is ApiMode {
  return v === 'local' || v === 'worktree' || v === 'mock';
}

/**
 * Parse an arbitrary value (e.g. a build-time env var) into an `ApiMode`.
 * Falls back to `'local'` when the value is missing or invalid, so a typo in
 * `VITE_API_DEFAULT` / `NEXT_PUBLIC_API_DEFAULT` can't put the dev panel into
 * an unsupported state.
 */
export function parseMode(v: unknown): ApiMode {
  return isApiMode(v) ? v : 'local';
}

/**
 * Resolve the active mode: localStorage override (if valid) ▸ caller's
 * `defaultMode` argument (validated, since TS-level `as ApiMode` casts at the
 * call site are unsound) ▸ hard fallback to `'local'`.
 *
 * Validating `defaultMode` at runtime defends against build-time env-var typos
 * propagating into the dev panel and breaking the mode switcher when the value
 * doesn't match any of the buttons.
 */
export function getMode(defaultMode: ApiMode): ApiMode {
  try {
    const v = localStorage.getItem(KEY);
    if (isApiMode(v)) return v;
  } catch {}
  return isApiMode(defaultMode) ? defaultMode : 'local';
}

export function setMode(mode: ApiMode): void {
  try {
    localStorage.setItem(KEY, mode);
  } catch {}
}

const SNAPSHOT_KEY = 'ppt-dev-panel-snapshot';

export function saveSnapshot(snapshot: unknown): void {
  try {
    localStorage.setItem(SNAPSHOT_KEY, JSON.stringify(snapshot));
  } catch {}
}

export function loadSnapshot<T = unknown>(): T | null {
  try {
    const raw = localStorage.getItem(SNAPSHOT_KEY);
    return raw ? (JSON.parse(raw) as T) : null;
  } catch {
    return null;
  }
}
