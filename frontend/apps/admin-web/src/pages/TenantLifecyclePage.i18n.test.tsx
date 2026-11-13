/**
 * TenantLifecyclePage — i18n wiring regression test.
 *
 * Guards against a regression to hard-coded English copy on the tenant
 * lifecycle page. The `react-i18next` mock below echoes the translation KEY
 * (ignoring any `defaultValue`), so a string that is routed through `t(...)`
 * renders as its key while a hard-coded literal renders as the literal.
 *
 * Before the i18n refactor these cards rendered literal English ("Export",
 * "Purge · irreversible", "Restore", "Tenant slug") and this file fails. After
 * the refactor the cards render their keys and it passes — exactly the
 * fails-before-fix / passes-after shape IG3 asks for.
 */

import { CapabilityProvider } from '@ppt/admin-ui';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { sessionTokenStore } from '../auth/tokenStore';
import { ToastProvider } from '../components/Toast';
import TenantLifecyclePage from './TenantLifecyclePage';

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
              capabilities: ['tenant_export', 'tenant_purge', 'tenant_restore'] as never[],
            }}
          >
            <ToastProvider>{children}</ToastProvider>
          </CapabilityProvider>
        </AdminAuthProvider>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe('TenantLifecyclePage — i18n wiring (IG3 regression)', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({ ok: false, status: 404 } as Response);
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  it('routes the tenant-picker and card titles through i18n keys', () => {
    render(
      <Providers>
        <TenantLifecyclePage />
      </Providers>
    );

    // Picker + the three lifecycle card titles must render their keys, proving
    // they are no longer hard-coded English literals.
    expect(screen.getByText('admin.tenantLifecycle.picker.label')).toBeDefined();
    expect(screen.getByText('admin.tenantLifecycle.export.title')).toBeDefined();
    expect(screen.getByText('admin.tenantLifecycle.purge.title')).toBeDefined();
    expect(screen.getByText('admin.tenantLifecycle.restore.title')).toBeDefined();

    // And the old hard-coded copy must be gone.
    expect(screen.queryByText('Purge · irreversible')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Load' })).toBeNull();
  });
});
