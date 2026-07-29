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

import {
  getConfig,
  listManifests,
  listScreens,
  publish,
  putDraft,
  putRails,
  rollback,
} from './api';

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

  const result = render(
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

  return { ...result, queryClient: qc };
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

  // -------------------------------------------------------------------------
  // 5. Background refetch does NOT clobber dirty local state
  // -------------------------------------------------------------------------

  it('dirty edits survive a background refetch (reseed policy: no clobber on invalidation)', async () => {
    const user = userEvent.setup();
    const { queryClient } = renderPage();

    // Wait for screens, then select
    await waitFor(() => {
      const sel = screen.getByRole('combobox', { name: /screen/i });
      if (!sel.querySelector('option[value="home/dashboard"]')) throw new Error('not loaded yet');
    });
    await user.selectOptions(screen.getByRole('combobox', { name: /screen/i }), 'home/dashboard');

    // Wait for Hero section to appear, then toggle visibility (dirty)
    await waitFor(() => screen.getByTestId('hide-btn-Hero'));
    await user.click(screen.getByTestId('hide-btn-Hero'));

    // Save Draft should be enabled (dirty flag set)
    const saveDraftBtn = screen.getByTestId('save-draft-btn');
    expect((saveDraftBtn as HTMLButtonElement).disabled).toBe(false);

    // Simulate background refetch — invalidation triggers a new network fetch,
    // but since epoch and screen haven't changed, the seed effect must NOT fire.
    await queryClient.refetchQueries({
      queryKey: ['admin', 'platform', 'layout', 'config', 'home/dashboard'],
    });

    // Dirty flag must still be set and Save Draft still enabled
    expect((screen.getByTestId('save-draft-btn') as HTMLButtonElement).disabled).toBe(false);

    // The Hero toggle button still has the 'Show section' title (visible=false after toggle)
    // — the seed effect must NOT have fired and reset it to visible=true.
    const hideBtn = screen.getByTestId('hide-btn-Hero');
    expect(hideBtn.getAttribute('title')).toMatch(/show section/i);
  });

  // -------------------------------------------------------------------------
  // 6. Rollback success reseeds from the refetched envelope
  // -------------------------------------------------------------------------

  it('rollback success reseeds tree from the refetched envelope draft', async () => {
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    // After rollback, getConfig resolves with a different draft (Stats section)
    const ROLLBACK_ENVELOPE: ConfigEnvelope = {
      ...SAMPLE_ENVELOPE,
      config: {
        ...SAMPLE_ENVELOPE.config,
        draft: {
          screen: 'home/dashboard',
          version: 0,
          sections: [{ type: 'Stats', visible: true }],
        },
      },
    };

    // First call returns original (Hero), subsequent calls return rollback result (Stats)
    vi.mocked(getConfig)
      .mockResolvedValueOnce(SAMPLE_ENVELOPE)
      .mockResolvedValue(ROLLBACK_ENVELOPE);

    vi.mocked(rollback).mockResolvedValue(undefined);

    const user = userEvent.setup();
    renderPage();

    await waitFor(() => {
      const sel = screen.getByRole('combobox', { name: /screen/i });
      if (!sel.querySelector('option[value="home/dashboard"]')) throw new Error('not loaded yet');
    });
    await user.selectOptions(screen.getByRole('combobox', { name: /screen/i }), 'home/dashboard');

    // Wait for initial Hero to appear
    await waitFor(() => screen.getByTestId('hide-btn-Hero'));

    // Click Rollback (version 1)
    await user.click(screen.getByRole('button', { name: /rollback/i }));

    await waitFor(() => expect(rollback).toHaveBeenCalledWith('test-token', 'home/dashboard', 1));

    // After rollback + reseed, tree must now show Stats (from ROLLBACK_ENVELOPE)
    await waitFor(() => {
      expect(screen.getByTestId('hide-btn-Stats')).toBeTruthy();
    });
  });

  // -------------------------------------------------------------------------
  // 7. Publish dirty warning shows when dirty; disappears after Save Draft
  // -------------------------------------------------------------------------

  it('dirty warning appears next to Publish when dirty; disappears after successful Save Draft', async () => {
    vi.mocked(putDraft).mockResolvedValue(undefined);

    const user = userEvent.setup();
    renderPage();

    await waitFor(() => {
      const sel = screen.getByRole('combobox', { name: /screen/i });
      if (!sel.querySelector('option[value="home/dashboard"]')) throw new Error('not loaded yet');
    });
    await user.selectOptions(screen.getByRole('combobox', { name: /screen/i }), 'home/dashboard');

    await waitFor(() => screen.getByTestId('hide-btn-Hero'));

    // Initially no dirty warning
    expect(screen.queryByTestId('publish-dirty-warning')).toBeNull();

    // Make a dirty edit
    await user.click(screen.getByTestId('hide-btn-Hero'));

    // Warning should now appear
    expect(await screen.findByTestId('publish-dirty-warning')).toBeTruthy();
    expect(screen.getByTestId('publish-dirty-warning').textContent).toMatch(/unsaved changes/i);

    // Save Draft
    await user.click(screen.getByTestId('save-draft-btn'));

    // After successful save, warning disappears
    await waitFor(() => {
      expect(screen.queryByTestId('publish-dirty-warning')).toBeNull();
    });

    // Rails dirty warning: also test via putRails path
    expect(putRails).not.toHaveBeenCalled(); // guard
  });

  // -------------------------------------------------------------------------
  // 8. Dirty screen change — confirm=false keeps selection; confirm=true switches
  // -------------------------------------------------------------------------

  it('dirty state + screen change with confirm=false keeps current selection and edits', async () => {
    vi.spyOn(window, 'confirm').mockReturnValue(false);

    const user = userEvent.setup();
    renderPage();

    await waitFor(() => {
      const sel = screen.getByRole('combobox', { name: /screen/i });
      if (!sel.querySelector('option[value="home/dashboard"]')) throw new Error('not loaded yet');
    });

    // Select first screen and make a dirty edit
    await user.selectOptions(screen.getByRole('combobox', { name: /screen/i }), 'home/dashboard');
    await waitFor(() => screen.getByTestId('hide-btn-Hero'));
    await user.click(screen.getByTestId('hide-btn-Hero'));
    expect(screen.getByTestId('publish-dirty-warning')).toBeTruthy();

    // Attempt to switch to a different screen — confirm returns false
    await user.selectOptions(screen.getByRole('combobox', { name: /screen/i }), 'home/profile');

    // Selection must remain home/dashboard
    const select = screen.getByRole('combobox', { name: /screen/i }) as HTMLSelectElement;
    expect(select.value).toBe('home/dashboard');

    // Dirty warning still visible (edits intact)
    expect(screen.getByTestId('publish-dirty-warning')).toBeTruthy();
  });

  it('dirty state + screen change with confirm=true switches screen', async () => {
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    const user = userEvent.setup();
    renderPage();

    await waitFor(() => {
      const sel = screen.getByRole('combobox', { name: /screen/i });
      if (!sel.querySelector('option[value="home/dashboard"]')) throw new Error('not loaded yet');
    });

    // Select first screen and make a dirty edit
    await user.selectOptions(screen.getByRole('combobox', { name: /screen/i }), 'home/dashboard');
    await waitFor(() => screen.getByTestId('hide-btn-Hero'));
    await user.click(screen.getByTestId('hide-btn-Hero'));
    expect(screen.getByTestId('publish-dirty-warning')).toBeTruthy();

    // Set up a mock config for the second screen
    vi.mocked(getConfig).mockResolvedValue({
      config: {
        screen: 'home/profile',
        draft: { screen: 'home/profile', version: 0, sections: [] },
        published: null,
        published_version: 0,
        rails: { hideable: [], mode_editable: [], reorderable: false, prop_whitelist: {} },
      },
      versions: [],
      kills: [],
    });

    // Switch screens — confirm returns true
    await user.selectOptions(screen.getByRole('combobox', { name: /screen/i }), 'home/profile');

    // Selection must have changed
    const select = screen.getByRole('combobox', { name: /screen/i }) as HTMLSelectElement;
    expect(select.value).toBe('home/profile');
  });

  // -------------------------------------------------------------------------
  // Publish is disabled while a draft/rails save is in flight
  // -------------------------------------------------------------------------

  it('Publish is disabled while Save Draft is pending', async () => {
    // Never-resolving putDraft keeps the save mutation pending
    vi.mocked(putDraft).mockImplementation(() => new Promise(() => {}));
    // Drop calls accumulated by earlier tests — this test asserts "not called"
    vi.mocked(publish).mockClear();

    const user = userEvent.setup();
    renderPage();

    await waitFor(() => {
      const sel = screen.getByRole('combobox', { name: /screen/i });
      if (!sel.querySelector('option[value="home/dashboard"]')) throw new Error('not loaded yet');
    });
    await user.selectOptions(screen.getByRole('combobox', { name: /screen/i }), 'home/dashboard');
    await waitFor(() => screen.getByTestId('hide-btn-Hero'));

    const publishBtn = screen.getByTestId('publish-btn') as HTMLButtonElement;
    expect(publishBtn.disabled).toBe(false);

    // Dirty the draft and start a save that never resolves
    await user.click(screen.getByTestId('hide-btn-Hero'));
    await user.click(screen.getByTestId('save-draft-btn'));

    await waitFor(() => {
      expect((screen.getByTestId('publish-btn') as HTMLButtonElement).disabled).toBe(true);
    });

    // Clicking Publish while a save is pending must not fire the mutation
    await user.click(screen.getByTestId('publish-btn'));
    expect(publish).not.toHaveBeenCalled();
  });

  // -------------------------------------------------------------------------
  // Queries are token-gated
  // -------------------------------------------------------------------------

  it('does not fire screens/manifests queries without a token', async () => {
    // Drop calls accumulated by earlier tests — this test asserts "not called"
    vi.mocked(listScreens).mockClear();
    vi.mocked(listManifests).mockClear();
    // Render WITHOUT seeding the session token store
    sessionStorage.clear();
    const qc = makeQueryClient();
    render(
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

    // Give effects a tick to (not) fire
    await new Promise((r) => setTimeout(r, 50));
    expect(listScreens).not.toHaveBeenCalled();
    expect(listManifests).not.toHaveBeenCalled();
  });
});
