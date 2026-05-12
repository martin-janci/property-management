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

/**
 * Color-scheme preference shared with the inline bootstrap script in
 * apps/reality-web/src/app/[locale]/layout.tsx — both read/write the same
 * `ppt-color-scheme` localStorage key so a toggle in the DevPanel is picked
 * up on the next reload (and immediately via the `applyColorScheme` helper
 * below, which patches the live document's data-color-scheme attribute).
 */
export type ColorScheme = 'light' | 'dark' | 'system';

const SCHEME_KEY = 'ppt-color-scheme';

export function isColorScheme(v: unknown): v is ColorScheme {
  return v === 'light' || v === 'dark' || v === 'system';
}

/**
 * Read the stored color-scheme preference.
 *
 * Storage uses an absent key (returns `null`) to mean "follow system",
 * because the inline bootstrap script needs that absence-signal to fall
 * through to `prefers-color-scheme` on first paint. Returns the literal
 * `'light'` or `'dark'` only when the user made an explicit non-system
 * choice. `'system'` is therefore never a return value from this function
 * — callers UI-bind it to `null`.
 */
export function getColorScheme(): 'light' | 'dark' | null {
  try {
    const v = localStorage.getItem(SCHEME_KEY);
    if (v === 'light' || v === 'dark') return v;
  } catch {}
  return null;
}

export function setColorScheme(scheme: ColorScheme): void {
  try {
    if (scheme === 'system') {
      // 'system' is encoded as "no preference" so the bootstrap script
      // falls back to `prefers-color-scheme` on next load.
      localStorage.removeItem(SCHEME_KEY);
    } else {
      localStorage.setItem(SCHEME_KEY, scheme);
    }
  } catch {}
}

/**
 * Apply a color scheme to `<html>` immediately (no reload). Mirrors what
 * the inline bootstrap script does at first paint.
 */
export function applyColorScheme(scheme: ColorScheme): void {
  if (typeof document === 'undefined') return;
  const resolved =
    scheme === 'system'
      ? window.matchMedia('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light'
      : scheme;
  document.documentElement.setAttribute('data-color-scheme', resolved);
}
