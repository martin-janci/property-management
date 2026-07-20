import { useEffect, useRef, useState } from 'react';
import { apiRequest } from '../../hooks/useApi';
import { readCachedLayout, writeCachedLayout } from './layoutCache';
import { DEFAULT_DASHBOARD_LAYOUT } from './registry';
import type { ResolvedScreen } from './types';

export function useDashboardLayout(screen: string): { layout: ResolvedScreen } {
  const [layout, setLayout] = useState<ResolvedScreen>(DEFAULT_DASHBOARD_LAYOUT);
  const warnedRef = useRef(false);

  useEffect(() => {
    let cancelled = false;

    async function activate() {
      // Phase 1: read cache at mount (activation — allowed at launch time)
      const cached = await readCachedLayout(screen);
      if (!cancelled && cached !== null) {
        setLayout(cached);
      }

      // Phase 2: background fetch — writes cache ONLY (never sets state)
      try {
        const fresh = await apiRequest<ResolvedScreen>(
          `/api/v1/layout/resolved/${screen}?platform=mobile`
        );
        if (!cancelled) {
          // Shape-check before writing
          if (
            fresh &&
            typeof fresh === 'object' &&
            fresh.screen === screen &&
            Array.isArray(fresh.sections)
          ) {
            await writeCachedLayout(screen, fresh);
          }
        }
      } catch (err) {
        if (!warnedRef.current) {
          warnedRef.current = true;
          console.warn('layout: background fetch failed', err);
        }
      }
    }

    activate();
    return () => {
      cancelled = true;
    };
  }, [screen]);

  return { layout };
}
