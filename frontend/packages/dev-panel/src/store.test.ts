// frontend/packages/dev-panel/src/store.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { getMode, setMode } from './store';

describe('dev panel store', () => {
  beforeEach(() => localStorage.clear());

  it('returns default when nothing stored', () => {
    expect(getMode('worktree')).toBe('worktree');
  });

  it('persists across calls', () => {
    setMode('mock');
    expect(getMode('local')).toBe('mock');
  });

  it('ignores invalid stored values', () => {
    localStorage.setItem('ppt-dev-panel-mode', 'garbage');
    expect(getMode('local')).toBe('local');
  });
});
