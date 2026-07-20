/**
 * LayoutEditorPage — TDD behavior tests (Task 4, Layout Editor MVP)
 *
 * Mirrors CapabilitiesAdminPage.test.tsx wrapper/mocking approach:
 *   - MemoryRouter + QueryClientProvider + AdminAuthProvider (seeded via sessionTokenStore)
 *   - vi.mock('./api') for the API module
 *   - vi.mock('react-i18next') for translation
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../../auth/AdminAuthContext';
import { sessionTokenStore } from '../../auth/tokenStore';
import { ToastProvider } from '../../components/Toast';
import { type ConfigEnvelope, LayoutApiError, type ManifestRow, type ScreenSummary } from './api';
import LayoutEditorPage from './LayoutEditorPage';

// ---------------------------------------------------------------------------
// Mock i18next
// ---------------------------------------------------------------------------

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (_key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? _key,
    i18n: { changeLanguage: vi.fn() },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ---------------------------------------------------------------------------
// Mock the API module
// ---------------------------------------------------------------------------

vi.mock('./api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./api')>();
  return {
    ...actual,
    listScreens: vi.fn(),
    getConfig: vi.fn(),
    putDraft: vi.fn(),
    putRails: vi.fn(),
    publish: vi.fn(),
    rollback: vi.fn(),
    kill: vi.fn(),
    unkill: vi.fn(),
    listManifests: vi.fn(),
  };
});

// ---------------------------------------------------------------------------
// Import mocked functions after vi.mock is hoisted
// ---------------------------------------------------------------------------

import { getConfig, listManifests, listScreens, publish, putDraft, rollback } from './api';

// ---------------------------------------------------------------------------
// Sample fixtures
// ---------------------------------------------------------------------------

const SAMPLE_SCREENS: ScreenSummary[] = [
  {
    screen: 'home/dashboard',
    draft: { screen: 'home/dashboard', version: 1, sections: [{ type: 'Hero', visible: true }] },
    published: null,
    published_version: 0,
    rails: { hideable: [], mode_editable: [], reorderable: false, prop_whitelist: {} },
    updated_at: '2026-01-01T00:00:00Z',
  },
  {
    screen: 'home/profile',
    draft: { screen: 'home/profile', version: 1, sections: [] },
    published: null,
    published_version: 0,
    rails: { hideable: [], mode_editable: [], reorderable: false, prop_whitelist: {} },
    updated_at: '2026-01-01T00:00:00Z',
  },
];

const SAMPLE_MANIFESTS: ManifestRow[] = [
  {
    platform: 'web',
    manifest: {
      platform: 'web',
      components: {
        Hero: { required: false },
        Stats: { required: false },
      },
    },
    updated_at: '2026-01-01T00:00:00Z',
  },
];

const SAMPLE_ENVELOPE: ConfigEnvelope = {
  config: {
    screen: 'home/dashboard',
    draft: { screen: 'home/dashboard', version: 1, sections: [{ type: 'Hero', visible: true }] },
    published: null,
    published_version: 0,
    rails: { hideable: [], mode_editable: [], reorderable: false, prop_whitelist: {} },
  },
  versions: [{ version: 1, published_at: '2026-01-01T00:00:00Z', published_by: 'admin' }],
  kills: [],
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, retryDelay: 0 } },
  });
}

function renderPage() {
  sessionStorage.clear();
  sessionTokenStore.set('test-token');

  const qc = makeQueryClient();

  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <AdminAuthProvider>
          <ToastProvider>
            <LayoutEditorPage />
          </ToastProvider>
        </AdminAuthProvider>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('LayoutEditorPage', () => {
  beforeEach(() => {
    vi.mocked(listScreens).mockResolvedValue(SAMPLE_SCREENS);
    vi.mocked(getConfig).mockResolvedValue(SAMPLE_ENVELOPE);
    vi.mocked(putDraft).mockResolvedValue(undefined);
    vi.mocked(rollback).mockResolvedValue(undefined);
    vi.mocked(publish).mockResolvedValue(undefined);
    vi.mocked(listManifests).mockResolvedValue(SAMPLE_MANIFESTS);
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  // -------------------------------------------------------------------------
  // 1. Screen list + config on selection
  // -------------------------------------------------------------------------

  it('renders screen list from listScreens and loads config on selection', async () => {
    const user = userEvent.setup();
    renderPage();

    // Wait for options to populate
    await waitFor(() => {
      const sel = screen.getByRole('combobox', { name: /screen/i });
      if (!sel.querySelector('option[value="home/dashboard"]')) throw new Error('not loaded yet');
    });

    const select = screen.getByRole('combobox', { name: /screen/i });
    // Both screens should be options
    expect(select.querySelector('option[value="home/dashboard"]')).toBeTruthy();
    expect(select.querySelector('option[value="home/profile"]')).toBeTruthy();

    // Select the screen
    await user.selectOptions(select, 'home/dashboard');

    // getConfig should be called with the token and screen
    await waitFor(() => expect(getConfig).toHaveBeenCalledWith('test-token', 'home/dashboard'));

    // Section type should appear in the tree (manifest component name)
    await waitFor(() => {
      const items = screen.getAllByText('Hero');
      expect(items.length).toBeGreaterThan(0);
    });
  });

  // -------------------------------------------------------------------------
  // 2. Eye-toggle → Save Draft calls putDraft
  // -------------------------------------------------------------------------

  it('eye-toggle then Save Draft calls putDraft with edited sections array', async () => {
    const user = userEvent.setup();
    renderPage();

    // Wait for options to populate, then select a screen
    await waitFor(() => {
      const sel = screen.getByRole('combobox', { name: /screen/i });
      if (!sel.querySelector('option[value="home/dashboard"]')) throw new Error('not loaded yet');
    });
    await user.selectOptions(screen.getByRole('combobox', { name: /screen/i }), 'home/dashboard');

    // Wait for the Hero section to appear
    await waitFor(() => screen.getByTestId('hide-btn-Hero'));

    // Click eye toggle to change visible state
    await user.click(screen.getByTestId('hide-btn-Hero'));

    // Now Save Draft should be available (dirty)
    const saveDraftBtn = await screen.findByRole('button', { name: /save draft/i });
    await user.click(saveDraftBtn);

    await waitFor(() =>
      expect(putDraft).toHaveBeenCalledWith(
        'test-token',
        'home/dashboard',
        expect.objectContaining({
          sections: expect.arrayContaining([
            expect.objectContaining({ type: 'Hero', visible: false }),
          ]),
        })
      )
    );
  });

  // -------------------------------------------------------------------------
  // 3. Publish 422 renders error list verbatim
  // -------------------------------------------------------------------------

  it('publish 422 LayoutApiError renders errors verbatim in persistent Alert', async () => {
    vi.mocked(publish).mockRejectedValue(new LayoutApiError(422, ['boom']));

    const user = userEvent.setup();
    renderPage();

    // Wait for options then select a screen
    await waitFor(() => {
      const sel = screen.getByRole('combobox', { name: /screen/i });
      if (!sel.querySelector('option[value="home/dashboard"]')) throw new Error('not loaded yet');
    });
    await user.selectOptions(screen.getByRole('combobox', { name: /screen/i }), 'home/dashboard');

    // Wait for config to load
    await waitFor(() => expect(getConfig).toHaveBeenCalled());

    // Click Publish
    const publishBtn = await screen.findByRole('button', { name: /^publish$/i });
    await user.click(publishBtn);

    // Error text must appear
    expect(await screen.findByText('boom')).toBeTruthy();
  });

  // -------------------------------------------------------------------------
  // 4. Rollback button calls rollback
  // -------------------------------------------------------------------------

  it('rollback button (confirm mocked) calls rollback with correct args', async () => {
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    const user = userEvent.setup();
    renderPage();

    // Wait for options then select a screen
    await waitFor(() => {
      const sel = screen.getByRole('combobox', { name: /screen/i });
      if (!sel.querySelector('option[value="home/dashboard"]')) throw new Error('not loaded yet');
    });
    await user.selectOptions(screen.getByRole('combobox', { name: /screen/i }), 'home/dashboard');

    // Wait for version history to appear (version 1)
    await waitFor(() => screen.getByRole('button', { name: /rollback/i }));

    await user.click(screen.getByRole('button', { name: /rollback/i }));

    await waitFor(() => expect(rollback).toHaveBeenCalledWith('test-token', 'home/dashboard', 1));
  });
});
