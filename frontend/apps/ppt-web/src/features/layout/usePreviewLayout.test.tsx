/**
 * Tests for usePreviewLayout hook.
 *
 * Mocks @ppt/shared's connectPreviewChild to keep tests pure and fast.
 * Uses window.history.replaceState to set up URL search params without
 * a full navigation.
 */

import type { ResolvedScreenLike } from '@ppt/shared';
import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { usePreviewLayout } from './usePreviewLayout';

// ---------------------------------------------------------------------------
// Mock @ppt/shared — only the functions we depend on
// ---------------------------------------------------------------------------
const mockSendSectionClick = vi.fn();
const mockDispose = vi.fn();
const mockConnectPreviewChild = vi.fn();

vi.mock('@ppt/shared', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@ppt/shared')>();
  return {
    ...actual,
    connectPreviewChild: (...args: Parameters<typeof mockConnectPreviewChild>) => {
      mockConnectPreviewChild(...args);
      return { sendSectionClick: mockSendSectionClick, dispose: mockDispose };
    },
  };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const PARENT_ORIGIN = 'https://parent.example.com';

function setSearchParams(search: string) {
  window.history.replaceState(null, '', `/${search}`);
}

function clearSearchParams() {
  window.history.replaceState(null, '', '/');
}

const SAMPLE_RESOLVED: ResolvedScreenLike = {
  screen: 'ppt/dashboard',
  version: 1,
  sections: [{ type: 'summary.v1', presentation: 'visible' }],
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('usePreviewLayout', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearSearchParams();
    // Allowlist gate: preview only activates when parentOrigin matches this env
    vi.stubEnv('VITE_LAYOUT_PREVIEW_PARENT_ORIGINS', PARENT_ORIGIN);
  });

  afterEach(() => {
    clearSearchParams();
    vi.unstubAllEnvs();
  });

  describe('when NOT in preview mode (no params)', () => {
    it('returns inPreview=false and null previewLayout', () => {
      const { result } = renderHook(() => usePreviewLayout('ppt/dashboard'));
      expect(result.current.inPreview).toBe(false);
      expect(result.current.previewLayout).toBeNull();
    });

    it('does NOT call connectPreviewChild', () => {
      renderHook(() => usePreviewLayout('ppt/dashboard'));
      expect(mockConnectPreviewChild).not.toHaveBeenCalled();
    });

    it('sendSectionClick is a no-op (does not throw)', () => {
      const { result } = renderHook(() => usePreviewLayout('ppt/dashboard'));
      expect(() => result.current.sendSectionClick('whatever')).not.toThrow();
    });
  });

  describe('when in preview mode (valid params)', () => {
    beforeEach(() => {
      setSearchParams(`?layoutPreview=1&parentOrigin=${encodeURIComponent(PARENT_ORIGIN)}`);
    });

    it('returns inPreview=true and null previewLayout initially', () => {
      const { result } = renderHook(() => usePreviewLayout('ppt/dashboard'));
      expect(result.current.inPreview).toBe(true);
      expect(result.current.previewLayout).toBeNull();
    });

    it('calls connectPreviewChild with correct parentOrigin and screen', () => {
      renderHook(() => usePreviewLayout('ppt/dashboard'));
      expect(mockConnectPreviewChild).toHaveBeenCalledOnce();
      const opts = mockConnectPreviewChild.mock.calls[0][0];
      expect(opts.parentOrigin).toBe(PARENT_ORIGIN);
      expect(opts.screen).toBe('ppt/dashboard');
    });

    it('updates previewLayout when onConfig fires', async () => {
      const { result } = renderHook(() => usePreviewLayout('ppt/dashboard'));

      // Grab the onConfig callback that was passed to connectPreviewChild
      const opts = mockConnectPreviewChild.mock.calls[0][0];
      const onConfig: (r: ResolvedScreenLike) => void = opts.onConfig;

      act(() => {
        onConfig(SAMPLE_RESOLVED);
      });

      expect(result.current.previewLayout).toEqual(SAMPLE_RESOLVED);
    });

    it('sendSectionClick delegates to the bridge sendSectionClick', () => {
      const { result } = renderHook(() => usePreviewLayout('ppt/dashboard'));
      result.current.sendSectionClick('summary.v1');
      expect(mockSendSectionClick).toHaveBeenCalledWith('summary.v1');
    });

    it('calls dispose on unmount', () => {
      const { unmount } = renderHook(() => usePreviewLayout('ppt/dashboard'));
      unmount();
      expect(mockDispose).toHaveBeenCalledOnce();
    });
  });

  describe('when params are malformed (layoutPreview=0)', () => {
    it('does not enter preview mode', () => {
      setSearchParams('?layoutPreview=0&parentOrigin=https://parent.example.com');
      const { result } = renderHook(() => usePreviewLayout('ppt/dashboard'));
      expect(result.current.inPreview).toBe(false);
      expect(mockConnectPreviewChild).not.toHaveBeenCalled();
    });
  });

  describe('when parentOrigin is invalid', () => {
    it('does not enter preview mode', () => {
      setSearchParams('?layoutPreview=1&parentOrigin=not-a-url');
      const { result } = renderHook(() => usePreviewLayout('ppt/dashboard'));
      expect(result.current.inPreview).toBe(false);
      expect(mockConnectPreviewChild).not.toHaveBeenCalled();
    });
  });

  describe('allowlist gate (VITE_LAYOUT_PREVIEW_PARENT_ORIGINS)', () => {
    it('never activates preview when the env var is unset', () => {
      vi.stubEnv('VITE_LAYOUT_PREVIEW_PARENT_ORIGINS', '');
      setSearchParams(`?layoutPreview=1&parentOrigin=${encodeURIComponent(PARENT_ORIGIN)}`);
      const { result } = renderHook(() => usePreviewLayout('ppt/dashboard'));
      expect(result.current.inPreview).toBe(false);
      expect(mockConnectPreviewChild).not.toHaveBeenCalled();
    });

    it('does not activate when parentOrigin is not in the allowlist', () => {
      vi.stubEnv('VITE_LAYOUT_PREVIEW_PARENT_ORIGINS', 'https://allowed.example.com');
      setSearchParams(`?layoutPreview=1&parentOrigin=${encodeURIComponent(PARENT_ORIGIN)}`);
      const { result } = renderHook(() => usePreviewLayout('ppt/dashboard'));
      expect(result.current.inPreview).toBe(false);
      expect(mockConnectPreviewChild).not.toHaveBeenCalled();
    });

    it('activates when parentOrigin matches one of several comma-separated entries', () => {
      vi.stubEnv(
        'VITE_LAYOUT_PREVIEW_PARENT_ORIGINS',
        `https://other.example.com, ${PARENT_ORIGIN}`
      );
      setSearchParams(`?layoutPreview=1&parentOrigin=${encodeURIComponent(PARENT_ORIGIN)}`);
      const { result } = renderHook(() => usePreviewLayout('ppt/dashboard'));
      expect(result.current.inPreview).toBe(true);
      expect(mockConnectPreviewChild).toHaveBeenCalledOnce();
    });
  });
});
