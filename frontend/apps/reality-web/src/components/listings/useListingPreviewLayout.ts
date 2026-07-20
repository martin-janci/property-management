'use client';

/**
 * useListingPreviewLayout — child side of the layout preview bridge for reality-web.
 *
 * Reads preview params from the URL ONCE (useState initializer).
 * When in preview mode, connects the bridge in a useEffect and stores pushed
 * configs in state. When not in preview mode, no listeners are registered.
 *
 * ADAPT (duplication note): this is a near-copy of ppt-web's usePreviewLayout.
 * Small enough that duplication beats a premature shared-react package. If a
 * shared-react package is ever created, merge both hooks there and remove these copies.
 *
 * SSR-safe: the useState initializer guards typeof window !== 'undefined' so
 * Next.js server-side rendering does not crash when window is absent.
 */

import type { ResolvedScreenLike } from '@ppt/shared';
import { connectPreviewChild, readPreviewParams } from '@ppt/shared';
import { useEffect, useRef, useState } from 'react';

export interface ListingPreviewLayoutResult {
  /** The layout pushed from the parent via the bridge, or null while waiting. */
  previewLayout: ResolvedScreenLike | null;
  /** True when the page is running inside a preview iframe. */
  inPreview: boolean;
  /** Send a section-click event to the parent. No-op when not in preview mode. */
  sendSectionClick: (type: string) => void;
}

/**
 * @param screen - The screen identifier sent to the parent in the `ready` message
 *   (e.g. `'reality/listing-detail'`).
 */
export function useListingPreviewLayout(screen: string): ListingPreviewLayoutResult {
  // Read params ONCE at mount — stable across re-renders.
  // Guard typeof window for SSR safety (Next.js may render on the server).
  const [params] = useState(() =>
    typeof window !== 'undefined' ? readPreviewParams(window.location.search) : null
  );
  const inPreview = params !== null;

  const [previewLayout, setPreviewLayout] = useState<ResolvedScreenLike | null>(null);

  // Stable ref to sendSectionClick so callers don't need to re-subscribe
  const sendRef = useRef<(type: string) => void>(() => {
    /* no-op when not in preview mode */
  });

  useEffect(() => {
    if (!params) return;

    const { sendSectionClick, dispose } = connectPreviewChild({
      parentOrigin: params.parentOrigin,
      screen,
      onConfig: (resolved) => {
        setPreviewLayout(resolved);
      },
    });

    sendRef.current = sendSectionClick;

    return () => {
      dispose();
      sendRef.current = () => {};
    };
  }, [params, screen]);

  return {
    previewLayout,
    inPreview,
    sendSectionClick: (type: string) => sendRef.current(type),
  };
}
