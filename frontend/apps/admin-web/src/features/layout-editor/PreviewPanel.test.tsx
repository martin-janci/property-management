/**
 * PreviewPanel tests — Task 5, Layout Editor preview bridge
 *
 * House rules (admin-web):
 * - NO jest-dom matchers (no toBeInTheDocument, toHaveTextContent, etc.)
 * - Use .toBeTruthy(), .toBeNull(), .textContent, .getAttribute(), etc.
 * - NO @ppt/ui-kit imports
 */

import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

const mockSendConfig = vi.fn();
const mockDispose = vi.fn();
let capturedOpts: Parameters<typeof import('@ppt/shared').connectPreviewParent>[0] | null = null;

vi.mock('@ppt/shared', () => ({
  connectPreviewParent: vi.fn((opts) => {
    capturedOpts = opts;
    return { sendConfig: mockSendConfig, dispose: mockDispose };
  }),
}));

vi.mock('./api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./api')>();
  return {
    ...actual,
    previewResolve: vi.fn(),
  };
});

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (_key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? _key,
    i18n: { changeLanguage: vi.fn() },
  }),
}));

// ---------------------------------------------------------------------------
// Imports (after mocks)
// ---------------------------------------------------------------------------

import type { ResolvedScreenLike } from '@ppt/shared';
import { connectPreviewParent } from '@ppt/shared';
import type { ScreenConfig } from './api';
import { previewResolve } from './api';
import { PreviewPanel } from './PreviewPanel';

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

const DRAFT: ScreenConfig = {
  screen: 'home/dashboard',
  version: 1,
  sections: [{ type: 'Hero' }],
};

const RESOLVED: ResolvedScreenLike = {
  screen: 'home/dashboard',
  version: 1,
  sections: [{ type: 'Hero', presentation: 'visible' }],
};

const VALID_URL = 'http://localhost:3001/dashboard';

function renderPanel(overrides: Partial<React.ComponentProps<typeof PreviewPanel>> = {}) {
  const onSectionSelected = vi.fn();
  const utils = render(
    <PreviewPanel
      screen="home/dashboard"
      platform="web"
      draft={DRAFT}
      token="test-token"
      onSectionSelected={onSectionSelected}
      {...overrides}
    />
  );
  return { ...utils, onSectionSelected };
}

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------

beforeEach(() => {
  capturedOpts = null;
  mockSendConfig.mockReset();
  mockDispose.mockReset();
  vi.mocked(previewResolve).mockReset();
  vi.mocked(connectPreviewParent).mockReset();
  vi.mocked(connectPreviewParent).mockImplementation((opts) => {
    capturedOpts = opts;
    return { sendConfig: mockSendConfig, dispose: mockDispose };
  });
  // Clear localStorage between tests (guard: jsdom may not expose it)
  try {
    localStorage.clear();
  } catch {
    /* ignore */
  }
});

afterEach(() => {
  vi.useRealTimers();
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('PreviewPanel', () => {
  /**
   * Test 1: Load builds the iframe src with appended params, preserving
   * existing query params in the URL.
   */
  it('Test 1: Load builds iframe src with preview params appended, preserving existing query params', async () => {
    const user = userEvent.setup();
    renderPanel();

    const urlInput = screen.getByTestId('preview-url-input') as HTMLInputElement;
    const loadBtn = screen.getByTestId('preview-load-btn');

    // Enter a URL that already has a query param
    await user.clear(urlInput);
    await user.type(urlInput, 'http://localhost:3001/dashboard?foo=bar');
    await user.click(loadBtn);

    // iframe should now exist
    const iframe = screen.getByTestId('preview-iframe') as HTMLIFrameElement;
    expect(iframe).toBeTruthy();

    const src = iframe.getAttribute('src') ?? '';
    // Must contain the existing param
    expect(src).toContain('foo=bar');
    // Must contain layoutPreview=1
    expect(src).toContain('layoutPreview=1');
    // Must contain parentOrigin
    expect(src).toContain('parentOrigin=');
    // parentOrigin should be the current window origin (jsdom default)
    expect(src).toContain(encodeURIComponent(window.location.origin));
  });

  /**
   * Test 2: On ready, previewResolve is called with the current draft and
   * sendConfig receives the resolved result.
   */
  it('Test 2: on ready, previewResolve called with draft and sendConfig receives its result', async () => {
    vi.mocked(previewResolve).mockResolvedValue(RESOLVED);

    const user = userEvent.setup();
    renderPanel();

    const urlInput = screen.getByTestId('preview-url-input') as HTMLInputElement;
    await user.clear(urlInput);
    await user.type(urlInput, VALID_URL);
    await user.click(screen.getByTestId('preview-load-btn'));

    // Trigger the ready callback from the bridge
    expect(capturedOpts).toBeTruthy();
    await act(async () => {
      capturedOpts!.onReady();
    });

    // After debounce (400ms), previewResolve should be called
    await act(async () => {
      await new Promise((r) => setTimeout(r, 450));
    });

    await waitFor(() => {
      expect(vi.mocked(previewResolve)).toHaveBeenCalledWith('test-token', DRAFT, 'web');
    });

    await waitFor(() => {
      expect(mockSendConfig).toHaveBeenCalledWith(RESOLVED);
    });
  });

  /**
   * Test 3: Draft change triggers debounced re-resolve (real timers, waitFor).
   *
   * Uses real timers + waitFor instead of fake timers to avoid the complexity
   * of fake-timer + React effect interaction in jsdom.
   */
  it('Test 3: draft change triggers debounced re-resolve', async () => {
    vi.mocked(previewResolve).mockResolvedValue(RESOLVED);

    const user = userEvent.setup();
    const { rerender } = renderPanel();

    const urlInput = screen.getByTestId('preview-url-input') as HTMLInputElement;
    await user.clear(urlInput);
    await user.type(urlInput, VALID_URL);
    await user.click(screen.getByTestId('preview-load-btn'));

    // Trigger ready
    expect(capturedOpts).toBeTruthy();
    await act(async () => {
      capturedOpts!.onReady();
    });

    // Wait for the initial debounced resolve to fire
    await waitFor(
      () => {
        expect(vi.mocked(previewResolve)).toHaveBeenCalledTimes(1);
      },
      { timeout: 1000 }
    );

    const callsBefore = vi.mocked(previewResolve).mock.calls.length;

    // Change the draft — this should trigger a new debounced resolve.
    const newDraft: ScreenConfig = {
      screen: 'home/dashboard',
      version: 2,
      sections: [{ type: 'Hero' }, { type: 'Banner' }],
    };

    rerender(
      <PreviewPanel
        screen="home/dashboard"
        platform="web"
        draft={newDraft}
        token="test-token"
        onSectionSelected={vi.fn()}
      />
    );

    // Wait for the re-resolve to fire after the debounce (>400ms)
    await waitFor(
      () => {
        expect(vi.mocked(previewResolve).mock.calls.length).toBeGreaterThan(callsBefore);
      },
      { timeout: 2000 }
    );

    const lastCall =
      vi.mocked(previewResolve).mock.calls[vi.mocked(previewResolve).mock.calls.length - 1];
    expect(lastCall[1]).toEqual(newDraft);
  });

  /**
   * Test 4: Resolve failure shows inline note, no crash.
   */
  it('Test 4: resolve failure shows inline error note, no crash', async () => {
    vi.mocked(previewResolve).mockRejectedValue(new Error('network error'));

    const user = userEvent.setup();
    renderPanel();

    const urlInput = screen.getByTestId('preview-url-input') as HTMLInputElement;
    await user.clear(urlInput);
    await user.type(urlInput, VALID_URL);
    await user.click(screen.getByTestId('preview-load-btn'));

    expect(capturedOpts).toBeTruthy();
    await act(async () => {
      capturedOpts!.onReady();
    });

    // Wait for debounce + rejection
    await act(async () => {
      await new Promise((r) => setTimeout(r, 450));
    });

    await waitFor(() => {
      const errorEl = screen.queryByTestId('preview-resolve-error');
      expect(errorEl).toBeTruthy();
    });

    // sendConfig should NOT have been called
    expect(mockSendConfig).not.toHaveBeenCalled();
  });

  /**
   * Test 5: Section-click invokes onSectionSelected.
   */
  it('Test 5: section-click invokes onSectionSelected', async () => {
    vi.mocked(previewResolve).mockResolvedValue(RESOLVED);

    const user = userEvent.setup();
    const { onSectionSelected } = renderPanel();

    const urlInput = screen.getByTestId('preview-url-input') as HTMLInputElement;
    await user.clear(urlInput);
    await user.type(urlInput, VALID_URL);
    await user.click(screen.getByTestId('preview-load-btn'));

    expect(capturedOpts).toBeTruthy();

    act(() => {
      capturedOpts!.onSectionClick('Hero');
    });

    expect(onSectionSelected).toHaveBeenCalledWith('Hero');
  });

  /**
   * Test 5b: Repeated ready (iframe reload) re-pushes the config.
   */
  it('Test 5b: a repeated ready re-pushes the config to the reloaded frame', async () => {
    vi.mocked(previewResolve).mockResolvedValue(RESOLVED);

    const user = userEvent.setup();
    renderPanel();

    const urlInput = screen.getByTestId('preview-url-input') as HTMLInputElement;
    await user.clear(urlInput);
    await user.type(urlInput, VALID_URL);
    await user.click(screen.getByTestId('preview-load-btn'));

    expect(capturedOpts).toBeTruthy();
    await act(async () => {
      capturedOpts!.onReady();
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 450));
    });
    await waitFor(() => {
      expect(mockSendConfig).toHaveBeenCalledTimes(1);
    });

    // The child reloads and sends ready again — the config must be re-pushed
    // (previously this no-oped because isReady was already true).
    await act(async () => {
      capturedOpts!.onReady();
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 450));
    });
    await waitFor(() => {
      expect(mockSendConfig).toHaveBeenCalledTimes(2);
    });
  });

  /**
   * Test 5c: Same-origin preview URL is rejected with an inline error.
   */
  it('Test 5c: same-origin preview URL shows inline error and does not load', async () => {
    const user = userEvent.setup();
    renderPanel();

    const urlInput = screen.getByTestId('preview-url-input') as HTMLInputElement;
    await user.clear(urlInput);
    await user.type(urlInput, `${window.location.origin}/dashboard`);
    await user.click(screen.getByTestId('preview-load-btn'));

    const errorEl = screen.queryByTestId('preview-url-error');
    expect(errorEl).toBeTruthy();
    expect(errorEl?.textContent).toContain('different origin');
    expect(screen.queryByTestId('preview-iframe')).toBeNull();
  });

  /**
   * Test 5d: Relative preview URL resolves same-origin and is rejected too.
   */
  it('Test 5d: relative preview URL is rejected as same-origin', async () => {
    const user = userEvent.setup();
    renderPanel();

    const urlInput = screen.getByTestId('preview-url-input') as HTMLInputElement;
    await user.clear(urlInput);
    await user.type(urlInput, '/dashboard');
    await user.click(screen.getByTestId('preview-load-btn'));

    expect(screen.queryByTestId('preview-url-error')).toBeTruthy();
    expect(screen.queryByTestId('preview-iframe')).toBeNull();
  });

  /**
   * Test 6: Invalid URL shows inline error, no iframe rendered.
   */
  it('Test 6: invalid URL shows inline error, no iframe', async () => {
    const user = userEvent.setup();
    renderPanel();

    const urlInput = screen.getByTestId('preview-url-input') as HTMLInputElement;
    await user.clear(urlInput);
    await user.type(urlInput, 'not-a-valid-url');
    await user.click(screen.getByTestId('preview-load-btn'));

    const errorEl = screen.queryByTestId('preview-url-error');
    expect(errorEl).toBeTruthy();

    const iframe = screen.queryByTestId('preview-iframe');
    expect(iframe).toBeNull();
  });
});
