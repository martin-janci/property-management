/**
 * Tests for useListingPreviewLayout hook.
 *
 * Near-mirror of ppt-web's usePreviewLayout.test.tsx.
 * Mocks @ppt/shared to keep tests pure and fast.
 * Uses window.history.replaceState to set up URL search params.
 *
 * ADAPT (duplication note): this hook is a near-copy of ppt-web's
 * usePreviewLayout — small enough that duplication beats a premature
 * shared-react package. If a shared-react package is ever created, merge.
 */

import type { ResolvedScreenLike } from '@ppt/shared';
import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useListingPreviewLayout } from './useListingPreviewLayout';

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
  screen: 'reality/listing-detail',
  version: 1,
  sections: [{ type: 'gallery.v1', presentation: 'visible' }],
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('useListingPreviewLayout', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearSearchParams();
  });

  afterEach(() => {
    clearSearchParams();
  });

  describe('when NOT in preview mode (no params)', () => {
    it('returns inPreview=false and null previewLayout', () => {
      const { result } = renderHook(() => useListingPreviewLayout('reality/listing-detail'));
      expect(result.current.inPreview).toBe(false);
      expect(result.current.previewLayout).toBeNull();
    });

    it('does NOT call connectPreviewChild', () => {
      renderHook(() => useListingPreviewLayout('reality/listing-detail'));
      expect(mockConnectPreviewChild).not.toHaveBeenCalled();
    });

    it('sendSectionClick is a no-op (does not throw)', () => {
      const { result } = renderHook(() => useListingPreviewLayout('reality/listing-detail'));
      expect(() => result.current.sendSectionClick('gallery.v1')).not.toThrow();
    });
  });

  describe('when in preview mode (valid params)', () => {
    beforeEach(() => {
      setSearchParams(`?layoutPreview=1&parentOrigin=${encodeURIComponent(PARENT_ORIGIN)}`);
    });

    it('returns inPreview=true and null previewLayout initially', () => {
      const { result } = renderHook(() => useListingPreviewLayout('reality/listing-detail'));
      expect(result.current.inPreview).toBe(true);
      expect(result.current.previewLayout).toBeNull();
    });

    it('calls connectPreviewChild with correct parentOrigin and screen', () => {
      renderHook(() => useListingPreviewLayout('reality/listing-detail'));
      expect(mockConnectPreviewChild).toHaveBeenCalledOnce();
      const opts = mockConnectPreviewChild.mock.calls[0][0];
      expect(opts.parentOrigin).toBe(PARENT_ORIGIN);
      expect(opts.screen).toBe('reality/listing-detail');
    });

    it('updates previewLayout when onConfig fires', async () => {
      const { result } = renderHook(() => useListingPreviewLayout('reality/listing-detail'));

      const opts = mockConnectPreviewChild.mock.calls[0][0];
      const onConfig: (r: ResolvedScreenLike) => void = opts.onConfig;

      act(() => {
        onConfig(SAMPLE_RESOLVED);
      });

      expect(result.current.previewLayout).toEqual(SAMPLE_RESOLVED);
    });

    it('sendSectionClick delegates to the bridge sendSectionClick', () => {
      const { result } = renderHook(() => useListingPreviewLayout('reality/listing-detail'));
      result.current.sendSectionClick('gallery.v1');
      expect(mockSendSectionClick).toHaveBeenCalledWith('gallery.v1');
    });

    it('calls dispose on unmount', () => {
      const { unmount } = renderHook(() => useListingPreviewLayout('reality/listing-detail'));
      unmount();
      expect(mockDispose).toHaveBeenCalledOnce();
    });
  });

  describe('when params are malformed (layoutPreview=0)', () => {
    it('does not enter preview mode', () => {
      setSearchParams('?layoutPreview=0&parentOrigin=https://parent.example.com');
      const { result } = renderHook(() => useListingPreviewLayout('reality/listing-detail'));
      expect(result.current.inPreview).toBe(false);
      expect(mockConnectPreviewChild).not.toHaveBeenCalled();
    });
  });

  describe('when parentOrigin is invalid', () => {
    it('does not enter preview mode', () => {
      setSearchParams('?layoutPreview=1&parentOrigin=not-a-url');
      const { result } = renderHook(() => useListingPreviewLayout('reality/listing-detail'));
      expect(result.current.inPreview).toBe(false);
      expect(mockConnectPreviewChild).not.toHaveBeenCalled();
    });
  });
});
