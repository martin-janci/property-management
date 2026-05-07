'use client';
import { DevPanel, getMode, parseMode } from '@ppt/dev-panel';
import { useEffect, useState } from 'react';

// Validate NEXT_PUBLIC_API_DEFAULT against the allowed ApiMode values so a typo
// in env can't put the dev panel into an unknown mode. parseMode falls back to
// 'local' on missing/invalid input.
const defaultMode = parseMode(process.env.NEXT_PUBLIC_API_DEFAULT);

export function DevPanelMount() {
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (process.env.NODE_ENV !== 'development') return;
    if (typeof window === 'undefined') return;
    // Reuse the dev-panel store's parsing/validation rather than reading
    // localStorage directly (avoids drift if the storage key/format changes).
    if (getMode(defaultMode) === 'mock') {
      import('../mocks/browser').then(({ worker }) => {
        void worker.start({ onUnhandledRequest: 'bypass' });
      });
    }
  }, []);

  if (process.env.NODE_ENV !== 'development') return null;
  if (!mounted) return null; // avoid SSR mismatch

  return <DevPanel defaultMode={defaultMode} onModeChange={() => window.location.reload()} />;
}
