/**
 * LayoutManifestsPage — TDD behavior tests (Task 5, Layout Editor MVP)
 *
 * Follows LayoutEditorPage.test.tsx conventions:
 *   - MemoryRouter + QueryClientProvider + AdminAuthProvider (seeded via sessionTokenStore)
 *   - vi.mock('./api') for the API module
 *   - vi.mock('react-i18next') for translation
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../../auth/AdminAuthContext';
import { sessionTokenStore } from '../../auth/tokenStore';
import { ToastProvider } from '../../components/Toast';
import type { ManifestRow } from './api';
import LayoutManifestsPage from './LayoutManifestsPage';

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
    listManifests: vi.fn(),
    putManifest: vi.fn(),
  };
});

// ---------------------------------------------------------------------------
// Import mocked functions after vi.mock is hoisted
// ---------------------------------------------------------------------------

import { listManifests, putManifest } from './api';

// ---------------------------------------------------------------------------
// Sample fixtures
// ---------------------------------------------------------------------------

const SAMPLE_MANIFESTS: ManifestRow[] = [
  {
    platform: 'web',
    manifest: {
      platform: 'web',
      components: {
        Hero: { required: false },
        Stats: { required: true },
        Banner: { required: false },
      },
    },
    updated_at: '2026-01-15T10:30:00Z',
  },
  {
    platform: 'mobile',
    manifest: {
      platform: 'mobile',
      components: {
        Card: { required: false },
      },
    },
    updated_at: '2026-01-16T08:00:00Z',
  },
];

const VALID_WEB_MANIFEST_JSON = JSON.stringify({
  platform: 'web',
  components: { Hero: { required: false }, NewSection: { required: true } },
});

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
            <LayoutManifestsPage />
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

describe('LayoutManifestsPage', () => {
  beforeEach(() => {
    vi.mocked(listManifests).mockResolvedValue(SAMPLE_MANIFESTS);
    vi.mocked(putManifest).mockResolvedValue(undefined);
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  // -------------------------------------------------------------------------
  // 1. Lists manifests from API
  // -------------------------------------------------------------------------

  it('lists stored manifests with platform, component count, and updated_at', async () => {
    renderPage();

    // Wait for both platform rows to appear in the table
    await waitFor(() => {
      expect(screen.getByText('web')).toBeTruthy();
      expect(screen.getByText('mobile')).toBeTruthy();
    });

    // Component counts visible (web has 3, mobile has 1) — wait for table cells
    await waitFor(() => {
      const allCells = document.querySelectorAll('td');
      const cellTexts = Array.from(allCells).map((c) => c.textContent?.trim());
      expect(cellTexts).toContain('3');
      expect(cellTexts).toContain('1');
    });

    // updated_at values rendered (locale-formatted — just check the year is present)
    const yearCells = screen.getAllByText(/2026/);
    expect(yearCells.length).toBeGreaterThanOrEqual(2);

    // Expand button exists — click to show <pre>
    const expandBtn = screen.getByTestId('expand-btn-web');
    await userEvent.setup().click(expandBtn);
    await waitFor(() => {
      const pres = document.querySelectorAll('pre');
      expect(pres.length).toBeGreaterThanOrEqual(1);
    });
  });

  // -------------------------------------------------------------------------
  // 2. Invalid JSON → inline error, putManifest NOT called
  // -------------------------------------------------------------------------

  it('shows inline error for invalid JSON and does not call putManifest', async () => {
    const user = userEvent.setup();
    renderPage();

    // Wait for page to load
    await waitFor(() => screen.getByLabelText(/manifest json/i));

    const textarea = screen.getByLabelText(/manifest json/i);
    fireEvent.change(textarea, { target: { value: '{not valid json}' } });

    const uploadBtn = screen.getByRole('button', { name: /upload/i });
    await user.click(uploadBtn);

    // Inline error shown
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeTruthy();
    });
    expect(screen.getByRole('alert').textContent).toMatch(/invalid json/i);

    // putManifest must NOT be called
    expect(putManifest).not.toHaveBeenCalled();
  });

  // -------------------------------------------------------------------------
  // 3. Platform mismatch → inline error, putManifest NOT called
  // -------------------------------------------------------------------------

  it('shows inline error when body platform does not match selected platform', async () => {
    const user = userEvent.setup();
    renderPage();

    await waitFor(() => screen.getByLabelText(/manifest json/i));

    // Select "mobile" platform
    const platformSelect = screen.getByLabelText(/platform/i);
    await user.selectOptions(platformSelect, 'mobile');

    // Paste a manifest that says "web"
    const mismatchJson = JSON.stringify({
      platform: 'web',
      components: { Hero: { required: false } },
    });

    const textarea = screen.getByLabelText(/manifest json/i);
    fireEvent.change(textarea, { target: { value: mismatchJson } });

    const uploadBtn = screen.getByRole('button', { name: /upload/i });
    await user.click(uploadBtn);

    // Inline error shown
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeTruthy();
    });
    expect(screen.getByRole('alert').textContent).toMatch(/platform mismatch/i);

    // putManifest must NOT be called
    expect(putManifest).not.toHaveBeenCalled();
  });

  // -------------------------------------------------------------------------
  // 3b. Array body → inline error, putManifest NOT called
  // -------------------------------------------------------------------------

  it('rejects array root and array components with inline error', async () => {
    const user = userEvent.setup();
    renderPage();

    await waitFor(() => screen.getByLabelText(/manifest json/i));

    const textarea = screen.getByLabelText(/manifest json/i);

    // Array root
    fireEvent.change(textarea, { target: { value: '[]' } });
    await user.click(screen.getByRole('button', { name: /upload/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeTruthy();
    });
    expect(screen.getByRole('alert').textContent).toMatch(/components/i);
    expect(putManifest).not.toHaveBeenCalled();

    // Reset and try {"components": []}
    fireEvent.change(textarea, { target: { value: '{"components": []}' } });
    await user.click(screen.getByRole('button', { name: /upload/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeTruthy();
    });
    expect(screen.getByRole('alert').textContent).toMatch(/components/i);
    expect(putManifest).not.toHaveBeenCalled();
  });

  // -------------------------------------------------------------------------
  // 4. Valid upload calls putManifest and shows success toast
  // -------------------------------------------------------------------------

  it('valid upload calls putManifest with correct args and shows success toast', async () => {
    const user = userEvent.setup();
    renderPage();

    await waitFor(() => screen.getByLabelText(/manifest json/i));

    // Platform already defaults to "web" — paste a valid web manifest
    const textarea = screen.getByLabelText(/manifest json/i);
    fireEvent.change(textarea, { target: { value: VALID_WEB_MANIFEST_JSON } });

    const uploadBtn = screen.getByRole('button', { name: /upload/i });
    await user.click(uploadBtn);

    await waitFor(() =>
      expect(putManifest).toHaveBeenCalledWith(
        'test-token',
        'web',
        expect.objectContaining({ components: expect.any(Object) })
      )
    );

    // Success toast must appear
    await waitFor(() => {
      expect(screen.getByText(/uploaded/i)).toBeTruthy();
    });
  });
});
