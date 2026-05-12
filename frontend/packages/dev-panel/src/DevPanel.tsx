// frontend/packages/dev-panel/src/DevPanel.tsx
import type React from 'react';
import { useState } from 'react';
import {
  type ApiMode,
  applyColorScheme,
  type ColorScheme,
  getColorScheme,
  getMode,
  isApiMode,
  isColorScheme,
  setColorScheme,
  setMode,
} from './store';

export interface DevPanelProps {
  defaultMode: ApiMode;
  onModeChange: (mode: ApiMode) => void;
  onReseedMock?: () => void;
  onSnapshotState?: () => void;
}

export const DevPanel: React.FC<DevPanelProps> = ({
  defaultMode,
  onModeChange,
  onReseedMock,
  onSnapshotState,
}) => {
  const [mode, setLocalMode] = useState<ApiMode>(() => getMode(defaultMode));
  // getColorScheme() returns `null` when no explicit preference is stored
  // (the inline bootstrap then falls through to prefers-color-scheme).
  // The select UI surfaces that as 'system' so the user can choose
  // between the three states; choosing 'system' removes the key again.
  const [scheme, setLocalScheme] = useState<ColorScheme>(() => getColorScheme() ?? 'system');

  const applyMode = (m: ApiMode) => {
    setMode(m);
    setLocalMode(m);
    onModeChange(m);
  };

  const applyScheme = (s: ColorScheme) => {
    setColorScheme(s);
    setLocalScheme(s);
    applyColorScheme(s);
  };

  return (
    <div
      style={{
        position: 'fixed',
        bottom: 8,
        right: 8,
        zIndex: 99999,
        background: '#222',
        color: '#fff',
        padding: '8px 10px',
        borderRadius: 6,
        fontFamily: 'monospace',
        fontSize: 12,
        opacity: 0.9,
        display: 'flex',
        flexDirection: 'column',
        gap: 4,
        // colorScheme: 'dark' tells the browser to render native form
        // controls (incl. <select> and its dropdown panel) with a dark
        // user-agent stylesheet, so the dev panel looks consistent in
        // both site themes.
        colorScheme: 'dark',
      }}
    >
      <div>
        <label htmlFor="ppt-dev-panel-mode">API:&nbsp;</label>
        <select
          id="ppt-dev-panel-mode"
          aria-label="API mode (local, worktree, or mock)"
          value={mode}
          onChange={(e) => {
            const next = e.target.value;
            if (isApiMode(next)) applyMode(next);
          }}
        >
          <option value="local">local</option>
          <option value="worktree">worktree</option>
          <option value="mock">mock</option>
        </select>
      </div>

      <div>
        <label htmlFor="ppt-dev-panel-scheme">Theme:&nbsp;</label>
        <select
          id="ppt-dev-panel-scheme"
          aria-label="Color scheme (light, dark, or system)"
          value={scheme}
          onChange={(e) => {
            const next = e.target.value;
            if (isColorScheme(next)) applyScheme(next);
          }}
        >
          <option value="light">light</option>
          <option value="dark">dark</option>
          <option value="system">system</option>
        </select>
      </div>

      {mode === 'mock' && onReseedMock && (
        <button type="button" onClick={onReseedMock} style={{ marginTop: 4 }}>
          Re-seed mock
        </button>
      )}
      {onSnapshotState && (
        <button type="button" onClick={onSnapshotState} style={{ marginTop: 4 }}>
          Snapshot state
        </button>
      )}
    </div>
  );
};
