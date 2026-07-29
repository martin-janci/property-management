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

/**
 * Allowlist gate for preview activation (unset = deny, mirroring reality-web's
 * LAYOUT_PREVIEW_FRAME_ANCESTORS posture): the parentOrigin from the URL must
 * exactly match one of the comma-separated origins in
 * VITE_LAYOUT_PREVIEW_PARENT_ORIGINS. When the env var is unset/empty, preview
 * NEVER activates.
 */
function readAllowedPreviewParams(search: string): { parentOrigin: string } | null {
  const params = readPreviewParams(search);
  if (!params) return null;

  const raw = import.meta.env.VITE_LAYOUT_PREVIEW_PARENT_ORIGINS ?? '';
  const allowed = raw
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
  if (allowed.length === 0) return null;
  if (!allowed.includes(params.parentOrigin)) return null;

  return params;
}

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
  const [params] = useState(() => readAllowedPreviewParams(window.location.search));
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
