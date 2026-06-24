/**
 * Client-side providers for the authenticated accounting-web shell.
 *
 * Wraps the (app) route group with:
 *   1. a tanstack-query QueryClient (the SDK hooks need a client in context), and
 *   2. one-time SDK configuration — base URL (config/api.ts) + Bearer-token
 *      reader (lib/auth.ts) — so @ppt/accounting-api-client knows where the
 *      accounting-server is and how to read the JWT (EPIC-ACC-17).
 *
 * Mirrors reality-web's QueryProvider; configuration runs synchronously during
 * the first render via useState initialiser so the SDK is wired before any
 * hook fires.
 */

'use client';

import { configureAccountingApi } from '@ppt/accounting-api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { type ReactNode, useState } from 'react';
import { getAccountingApiBase } from '@/config/api';
import { getAccessToken } from '@/lib/auth';

interface AppProvidersProps {
  children: ReactNode;
}

export function AppProviders({ children }: AppProvidersProps) {
  const [queryClient] = useState(() => {
    // Wire the SDK once, before any hook runs. configureAccountingApi is
    // idempotent, and the token provider is read lazily on every request so it
    // always reflects the current session.
    configureAccountingApi({
      baseUrl: getAccountingApiBase(),
      getToken: getAccessToken,
    });

    return new QueryClient({
      defaultOptions: {
        queries: {
          staleTime: 60 * 1000, // 1 minute
          refetchOnWindowFocus: false,
          retry: 1,
        },
      },
    });
  });

  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
}
