// frontend/packages/dev-panel/src/DevPanel.tsx
import type React from 'react';
import { useState } from 'react';
import { type ApiMode, getMode, isApiMode, setMode } from './store';

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
  const apply = (m: ApiMode) => {
    setMode(m);
    setLocalMode(m);
    onModeChange(m);
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
        opacity: 0.85,
      }}
    >
      <div>
        <label htmlFor="ppt-dev-panel-mode">API:&nbsp;</label>
        <select
          id="ppt-dev-panel-mode"
          aria-label="API mode (local, worktree, or mock)"
          value={mode}
          onChange={(e) => {
            // Defense in depth: a tampered DOM (devtools-edit, future option
            // added to one place but not another, etc.) could deliver a value
            // that isn't a valid ApiMode. Validate before persisting to
            // localStorage and notifying the parent.
            const next = e.target.value;
            if (isApiMode(next)) {
              apply(next);
            }
          }}
        >
          <option value="local">local</option>
          <option value="worktree">worktree</option>
          <option value="mock">mock</option>
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
