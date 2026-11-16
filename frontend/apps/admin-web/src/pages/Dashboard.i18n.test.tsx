/**
 * Dashboard — i18n wiring regression test.
 *
 * Guards against a regression to hard-coded English copy on the admin
 * dashboard. The `react-i18next` mock below echoes the translation KEY
 * (ignoring any `defaultValue`), so a string that is routed through `t(...)`
 * renders as its key while a hard-coded literal renders as the literal.
 *
 * Before the i18n refactor the stat tiles, cards and quick actions rendered
 * literal English ("Tenants active", "Recent high-risk events · 24h", "Quick
 * actions", …) and this file fails. After the refactor they render their keys
 * and it passes — exactly the fails-before-fix / passes-after shape IG3 asks
 * for. Mirrors TenantLifecyclePage.i18n.test.tsx.
 */

import { CapabilityProvider } from '@ppt/admin-ui';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { sessionTokenStore } from '../auth/tokenStore';
import { MfaWindowProvider } from '../components/MfaWindowChip';
import { Dashboard } from './Dashboard';

// Echo the translation key (NOT the defaultValue) so we can assert that every
// user-facing string is routed through `t(...)`.
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn() },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

function Providers({ children }: { children: ReactNode }) {
  sessionStorage.clear();
  sessionTokenStore.set('test-token');
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <AdminAuthProvider>
          <CapabilityProvider
            value={{
              isPlatformPrincipal: true,
              capabilities: ['audit_read', 'users_impersonate', 'memberships_grant'] as never[],
            }}
          >
            <MfaWindowProvider>{children}</MfaWindowProvider>
          </CapabilityProvider>
        </AdminAuthProvider>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe('Dashboard — i18n wiring (IG3 regression)', () => {
  beforeEach(() => {
    // 404 everything so the cards render their empty states (which are also
    // routed through i18n keys).
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({ ok: false, status: 404 } as Response);
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  it('routes stat tiles, card titles and quick actions through i18n keys', async () => {
    render(
      <Providers>
        <Dashboard />
      </Providers>
    );

    // Stat tiles render their keys once loaded.
    await waitFor(() =>
      expect(screen.getByText('admin.dashboard.tiles.tenantsActive.label')).toBeDefined()
    );
    expect(screen.getByText('admin.dashboard.tiles.operatorsOnline.label')).toBeDefined();
    expect(screen.getByText('admin.dashboard.tiles.pendingMerges.label')).toBeDefined();
    expect(screen.getByText('admin.dashboard.tiles.highRisk.label')).toBeDefined();

    // Card titles + quick actions render their keys.
    expect(screen.getByText('admin.dashboard.highRisk.title')).toBeDefined();
    expect(screen.getByText('admin.dashboard.impersonation.title')).toBeDefined();
    expect(screen.getByText('admin.dashboard.merges.title')).toBeDefined();
    expect(screen.getByText('admin.dashboard.quickActions.title')).toBeDefined();

    // And the old hard-coded copy must be gone.
    expect(screen.queryByText('Tenants active')).toBeNull();
    expect(screen.queryByText('Recent high-risk events · 24h')).toBeNull();
    expect(screen.queryByText('Quick actions')).toBeNull();
    expect(screen.queryByText('Operators online')).toBeNull();
  });
});
