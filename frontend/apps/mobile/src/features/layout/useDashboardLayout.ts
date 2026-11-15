import { useEffect, useRef, useState } from 'react';
import { useAuth } from '../../contexts/AuthContext';
import { apiRequest } from '../../hooks/useApi';
import { readCachedLayout, writeCachedLayout } from './layoutCache';
import { DEFAULT_DASHBOARD_LAYOUT } from './registry';
import type { ResolvedScreen } from './types';

export function useDashboardLayout(screen: string): { layout: ResolvedScreen } {
  // The persisted layout embeds the org's tenant override, so the cache key
  // MUST be scoped to the logged-in identity — otherwise org A's dashboard
  // layout activates for org B on a shared device (issue #2486). The hook runs
  // inside an authenticated screen, so `user.id` is available; we use it as the
  // tenant scope for the cache key.
  const { user } = useAuth();
  const scopeId = user?.id ?? null;
  const [layout, setLayout] = useState<ResolvedScreen>(DEFAULT_DASHBOARD_LAYOUT);
  const warnedRef = useRef(false);

  useEffect(() => {
    let cancelled = false;

    // Without an authenticated identity we cannot tenant-scope the cache key,
    // so we must NOT read or write a shared cache entry — fall back to the
    // default layout + live fetch only (issue #2486).
    if (!scopeId) {
      return;
    }
    const sid = scopeId;

    async function activate() {
      // Phase 1: read cache at mount (activation — allowed at launch time)
      const cached = await readCachedLayout(sid, screen);
      const hadCachedLayout = cached !== null;
      if (!cancelled && cached !== null) {
        setLayout(cached);
      }

      // Phase 2: background fetch — refreshes the persisted cache. When a cached
      // layout was already activated above we keep next-launch semantics (write
      // cache only, never set state) so an established user's dashboard never
      // reshuffles mid-session. But on a cache MISS the screen is still showing
      // the generic DEFAULT layout, so leaving the fetched layout un-applied
      // would strand the user on the default for the whole session (new users,
      // and the first launch after the #2486 logout cache-sweep). In that case
      // we apply the fetched layout to state as soon as it arrives — this is the
      // only path that renders the freshly-fetched layout.
      try {
        const fresh = await apiRequest<ResolvedScreen>(
          `/api/v1/layout/resolved/${screen}?platform=mobile`
        );
        if (!cancelled) {
          // Shape-check before writing/applying
          if (
            fresh &&
            typeof fresh === 'object' &&
            fresh.screen === screen &&
            Array.isArray(fresh.sections)
          ) {
            await writeCachedLayout(sid, screen, fresh);
            if (!cancelled && !hadCachedLayout) {
              setLayout(fresh);
            }
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
  }, [screen, scopeId]);

  return { layout };
}
