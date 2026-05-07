'use client';
import { type ApiMode, DevPanel, getMode } from '@ppt/dev-panel';
import { useEffect, useState } from 'react';

export function DevPanelMount() {
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (process.env.NODE_ENV !== 'development') return;
    if (typeof window === 'undefined') return;
    const defaultMode = (process.env.NEXT_PUBLIC_API_DEFAULT as ApiMode) || 'local';
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

  const defaultMode = (process.env.NEXT_PUBLIC_API_DEFAULT as ApiMode) || 'local';
  return <DevPanel defaultMode={defaultMode} onModeChange={() => window.location.reload()} />;
}
