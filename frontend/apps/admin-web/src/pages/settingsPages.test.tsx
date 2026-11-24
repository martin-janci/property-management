/**
 * Platform settings + mobile config pages load the stored document and PATCH
 * it back. Both used to render hard-coded defaults read-only ("write endpoint
 * is not implemented") although the endpoints exist on api-server.
 */

import { CapabilityProvider } from '@ppt/admin-ui';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactElement } from 'react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { sessionTokenStore } from '../auth/tokenStore';
import { ToastProvider } from '../components/Toast';
import MobileConfigPage, { flagsPatch, flagValuesFromDoc } from './MobileConfigPage';
import PlatformPage from './platform';

function renderPage(page: ReactElement, capabilities: string[]) {
  sessionStorage.clear();
  sessionTokenStore.set('test-token');
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false, retryDelay: 0 } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <AdminAuthProvider>
          <CapabilityProvider
            value={{ isPlatformPrincipal: true, capabilities: capabilities as never[] }}
          >
            <ToastProvider>{page}</ToastProvider>
          </CapabilityProvider>
        </AdminAuthProvider>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

function json(body: unknown, status = 200): Response {
  return { ok: status < 400, status, json: async () => body } as Response;
}

/** The `[url, init]` of the first PATCH the page sent. */
function patchCall(): [string, RequestInit] {
  const call = vi.mocked(fetch).mock.calls.find(([, init]) => init?.method === 'PATCH');
  if (!call) throw new Error('no PATCH request was sent');
  return [String(call[0]), call[1] as RequestInit];
}

beforeEach(() => {
  vi.spyOn(console, 'warn').mockImplementation(() => {});
  vi.spyOn(console, 'error').mockImplementation(() => {});
  vi.spyOn(globalThis, 'fetch');
});

afterEach(() => {
  sessionStorage.clear();
  vi.restoreAllMocks();
});

describe('PlatformPage', () => {
  const stored = {
    'platform.maintenance_mode': false,
    'platform.signup_enabled': true,
    'platform.support_email': 'help@example.test',
    updated_at: '2026-11-01T10:00:00Z',
  };

  it('shows the stored settings and saves edits with PATCH', async () => {
    vi.mocked(fetch).mockImplementation(async (_url, init) =>
      json(init?.method === 'PATCH' ? { ...stored, updated_at: '2026-11-02T10:00:00Z' } : stored)
    );
    renderPage(<PlatformPage />, ['site_settings_write']);

    const email = (await screen.findByDisplayValue('help@example.test')) as HTMLInputElement;
    await userEvent.clear(email);
    await userEvent.type(email, 'ops@example.test');
    await userEvent.click(screen.getByRole('button', { name: /save/i }));

    await waitFor(() => expect(patchCall()).toBeDefined());
    const [url, init] = patchCall();
    expect(url).toBe('/api/v1/platform-admin/settings');
    expect((init.headers as Record<string, string>).Authorization).toBe('Bearer test-token');
    expect(JSON.parse(String(init.body))).toEqual({
      'platform.maintenance_mode': false,
      'platform.signup_enabled': true,
      'platform.support_email': 'ops@example.test',
    });
  });

  it('reports a load failure instead of showing made-up defaults', async () => {
    vi.mocked(fetch).mockResolvedValue(json({ message: 'nope' }, 500));
    renderPage(<PlatformPage />, ['site_settings_write']);

    expect(await screen.findByRole('alert')).toBeDefined();
    expect(screen.queryByRole('button', { name: /save/i })).toBeNull();
  });
});

describe('MobileConfigPage', () => {
  const stored = {
    min_ios_version: '2.3.0',
    min_android_version: null,
    flags: { 'rn.new_onboarding': true, other: 'kept' },
    updated_at: '2026-11-01T10:00:00Z',
  };

  it('maps the nested flags object to and from the flat form values', () => {
    expect(flagValuesFromDoc(stored)).toEqual({
      'rn.new_onboarding': true,
      'rn.push_notifications_v2': false,
    });
    expect(flagsPatch({ 'rn.new_onboarding': false, 'rn.push_notifications_v2': true })).toEqual({
      flags: { 'rn.new_onboarding': false, 'rn.push_notifications_v2': true },
    });
  });

  it('shows the stored version floor and saves it with PATCH', async () => {
    vi.mocked(fetch).mockImplementation(async () => json(stored));
    renderPage(<MobileConfigPage />, ['mobile_config_write']);

    const ios = (await screen.findByDisplayValue('2.3.0')) as HTMLInputElement;
    await userEvent.clear(ios);
    await userEvent.type(ios, '2.4.0');
    await userEvent.click(screen.getAllByRole('button', { name: /save/i })[0]);

    await waitFor(() => expect(patchCall()).toBeDefined());
    const [url, init] = patchCall();
    expect(url).toBe('/api/v1/admin/mobile-config');
    expect(JSON.parse(String(init.body))).toEqual({
      min_ios_version: '2.4.0',
      min_android_version: '',
    });
  });
});
