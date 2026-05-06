// frontend/packages/dev-panel/src/store.ts
export type ApiMode = 'local' | 'worktree' | 'mock';

const KEY = 'ppt-dev-panel-mode';

export function getMode(defaultMode: ApiMode): ApiMode {
  try {
    const v = localStorage.getItem(KEY);
    if (v === 'local' || v === 'worktree' || v === 'mock') return v;
  } catch {}
  return defaultMode;
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
