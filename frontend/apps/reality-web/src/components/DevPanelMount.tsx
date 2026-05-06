'use client';
import { DevPanel, type ApiMode } from '@ppt/dev-panel';
import { useEffect, useState } from 'react';

export function DevPanelMount() {
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (process.env.NODE_ENV !== 'development') return;
    const initialMode =
      typeof window !== 'undefined'
        ? (localStorage.getItem('ppt-dev-panel-mode') ?? 'local')
        : 'local';
    if (initialMode === 'mock') {
      import('../mocks/browser').then(({ worker }) => {
        void worker.start({ onUnhandledRequest: 'bypass' });
      });
    }
  }, []);

  if (process.env.NODE_ENV !== 'development') return null;
  if (!mounted) return null; // avoid SSR mismatch

  const defaultMode = (process.env.NEXT_PUBLIC_API_DEFAULT as ApiMode) || 'local';
  return (
    <DevPanel
      defaultMode={defaultMode}
      onModeChange={() => window.location.reload()}
    />
  );
}
