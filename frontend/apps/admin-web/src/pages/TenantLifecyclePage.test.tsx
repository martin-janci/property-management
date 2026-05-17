/**
 * TenantLifecyclePage — purge wizard + capability gating tests.
 *
 * NOTE: The main page's `PurgeWizard` is only rendered when the loaded tenant
 * has `softDeletedAt != null` — and the page currently hard-codes a tenant
 * fixture where that is `null`. Rather than mutate the page's internal state
 * from tests (brittle) or plumb a `defaultTenant` prop just for tests
 * (production code change), we render the exported `PurgeWizard` component
 * directly with a soft-deleted fixture for wizard-flow tests, and render the
 * full page for capability-gating tests.
 */

import { CapabilityProvider } from '@ppt/admin-ui';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { sessionTokenStore } from '../auth/tokenStore';
import { ToastProvider } from '../components/Toast';
import TenantLifecyclePage, { PurgeWizard, type TenantInfo } from './TenantLifecyclePage';

// ---------------------------------------------------------------------------
// i18next mock (Toast uses useTranslation)
// ---------------------------------------------------------------------------
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn() },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ---------------------------------------------------------------------------
// Fixtures + helpers
// ---------------------------------------------------------------------------

const DEMO_SLUG = 'dunajska-residences';

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, retryDelay: 0 } },
  });
}

interface RenderOptions {
  capabilities?: string[];
}

function Providers({
  capabilities = ['tenant_purge', 'tenant_export', 'tenant_restore'],
  children,
}: RenderOptions & { children: React.ReactNode }) {
  sessionStorage.clear();
  sessionTokenStore.set('test-token');
  const qc = makeQueryClient();
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <AdminAuthProvider>
          <CapabilityProvider
            value={{ isPlatformPrincipal: true, capabilities: capabilities as never[] }}
          >
            <ToastProvider>{children}</ToastProvider>
          </CapabilityProvider>
        </AdminAuthProvider>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

function renderPage(opts: RenderOptions = {}) {
  return render(
    <Providers {...opts}>
      <TenantLifecyclePage />
    </Providers>
  );
}

/** Soft-deleted, recently-exported tenant — opens the wizard at step 2. */
function softDeletedTenant(): TenantInfo {
  return {
    id: 'demo',
    slug: DEMO_SLUG,
    name: 'Dunajská Residences',
    softDeletedAt: new Date('2026-05-15T00:00:00Z'),
    // Recent export so step 1 auto-advances (exportAgeDays < 7).
    lastExportAt: new Date('2026-05-16T09:34:00Z'),
    lastExportBy: 'martin@rlt.sk',
    lastExportSizeGb: 2.3,
  };
}

function renderWizard(opts: RenderOptions = {}) {
  const onCancel = vi.fn();
  const result = render(
    <Providers {...opts}>
      <PurgeWizard tenant={softDeletedTenant()} onCancel={onCancel} />
    </Providers>
  );
  return { ...result, onCancel };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('TenantLifecyclePage — capability gating', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch');
    // Pin "now" to a date after the demo export so the wizard's step-1 check
    // is satisfied. Page tests don't open the wizard, but pinning is harmless.
    vi.spyOn(Date, 'now').mockReturnValue(new Date('2026-05-17T00:00:00Z').getTime());
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  it('hides Start export button when tenant_export capability is absent', () => {
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 404 } as Response);
    renderPage({ capabilities: ['tenant_purge'] });
    expect(screen.queryByRole('button', { name: /start export/i })).toBeNull();
  });

  it('shows Start export button when tenant_export is present', () => {
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 404 } as Response);
    renderPage({ capabilities: ['tenant_export'] });
    expect(screen.getByRole('button', { name: /start export/i })).toBeDefined();
  });
});

describe('PurgeWizard — step 2 (slug confirmation)', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch');
    // 1 day after the demo export → step 1 auto-advances (exportAgeDays < 7).
    vi.spyOn(Date, 'now').mockReturnValue(new Date('2026-05-17T09:34:00Z').getTime());
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  it('renders the slug confirmation input', async () => {
    renderWizard();
    const input = await screen.findByLabelText(`Type "${DEMO_SLUG}" to confirm`);
    expect(input).toBeDefined();
    expect((input as HTMLInputElement).tagName).toBe('INPUT');
  });

  it('"Continue to MFA" is disabled until typed text exactly matches the slug', async () => {
    const user = userEvent.setup();
    renderWizard();

    const input = (await screen.findByLabelText(
      `Type "${DEMO_SLUG}" to confirm`
    )) as HTMLInputElement;
    const continueBtn = screen.getByRole('button', {
      name: /Continue to MFA/i,
    }) as HTMLButtonElement;

    // Initially empty → disabled.
    expect(continueBtn.disabled).toBe(true);

    // Partial match → still disabled.
    await user.type(input, DEMO_SLUG.slice(0, -3));
    expect(continueBtn.disabled).toBe(true);

    // Finish typing → enabled.
    await user.type(input, DEMO_SLUG.slice(-3));
    expect(input.value).toBe(DEMO_SLUG);
    expect(continueBtn.disabled).toBe(false);
  });
});

describe('PurgeWizard — step 3 (audit reason + purge POST)', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch');
    vi.spyOn(Date, 'now').mockReturnValue(new Date('2026-05-17T09:34:00Z').getTime());
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  /** Advance the wizard from step 2 → step 3 (slug already validated). */
  async function advanceToStep3(user: ReturnType<typeof userEvent.setup>) {
    const input = (await screen.findByLabelText(
      `Type "${DEMO_SLUG}" to confirm`
    )) as HTMLInputElement;
    await user.type(input, DEMO_SLUG);
    await user.click(screen.getByRole('button', { name: /Continue to MFA/i }));
  }

  it('renders AuditReasonPrompt when advanced to step 3', async () => {
    const user = userEvent.setup();
    renderWizard();
    await advanceToStep3(user);

    // AuditReasonPrompt's textarea has aria-label="Audit reason".
    await waitFor(() => expect(screen.getByLabelText('Audit reason')).toBeDefined());
  });

  it('shows MFA-required toast on 401 from purge endpoint', async () => {
    const user = userEvent.setup();
    vi.mocked(fetch).mockImplementation(async (input, init) => {
      const url = input.toString();
      if (url.includes('/purge') && init?.method === 'POST') {
        return { ok: false, status: 401 } as Response;
      }
      return { ok: false, status: 404 } as Response;
    });

    renderWizard();
    await advanceToStep3(user);

    // Fill audit reason (>=30 chars) and TOTP (6 digits).
    await user.type(
      screen.getByLabelText('Audit reason'),
      'Decommissioning per platform request 2026-05-17 ticket A1'
    );
    for (let i = 0; i < 6; i++) {
      await user.type(screen.getByLabelText(`Digit ${i + 1} of 6`), '1');
    }

    const purgeBtn = screen.getByRole('button', { name: /Verify and purge/i });
    await waitFor(() => expect((purgeBtn as HTMLButtonElement).disabled).toBe(false));
    await user.click(purgeBtn);

    await waitFor(() => expect(screen.getByText(/MFA required/i)).toBeDefined());
  });

  it('shows Network error toast when purge fetch throws', async () => {
    const user = userEvent.setup();
    vi.mocked(fetch).mockImplementation(async (input, init) => {
      const url = input.toString();
      if (url.includes('/purge') && init?.method === 'POST') {
        throw new TypeError('Network error');
      }
      return { ok: false, status: 404 } as Response;
    });

    renderWizard();
    await advanceToStep3(user);

    await user.type(
      screen.getByLabelText('Audit reason'),
      'Decommissioning per platform request 2026-05-17 ticket A1'
    );
    for (let i = 0; i < 6; i++) {
      await user.type(screen.getByLabelText(`Digit ${i + 1} of 6`), '1');
    }

    const purgeBtn = screen.getByRole('button', { name: /Verify and purge/i });
    await waitFor(() => expect((purgeBtn as HTMLButtonElement).disabled).toBe(false));
    await user.click(purgeBtn);

    await waitFor(() => expect(screen.getByText(/Network error/i)).toBeDefined());
  });
});
