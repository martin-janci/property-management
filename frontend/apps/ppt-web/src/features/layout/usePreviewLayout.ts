/**
 * usePreviewLayout — child side of the layout preview bridge.
 *
 * Reads preview params from the URL ONCE (useState initializer).
 * When in preview mode, connects the bridge in a useEffect and stores pushed
 * configs in state.  When not in preview mode, no listeners are registered.
 */

import type { ResolvedScreenLike } from '@ppt/shared';
import { connectPreviewChild, readPreviewParams } from '@ppt/shared';
import { useEffect, useRef, useState } from 'react';

export interface PreviewLayoutResult {
  /** The layout pushed from the parent via the bridge, or null while waiting. */
  previewLayout: ResolvedScreenLike | null;
  /** True when the page is running inside a preview iframe. */
  inPreview: boolean;
  /** Send a section-click event to the parent. No-op when not in preview mode. */
  sendSectionClick: (type: string) => void;
}

/**
 * @param screen - The screen identifier sent to the parent in the `ready` message
 *   (e.g. `'ppt/dashboard'`).
 */
export function usePreviewLayout(screen: string): PreviewLayoutResult {
  // Read params ONCE at mount — stable across re-renders
  const [params] = useState(() => readPreviewParams(window.location.search));
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
