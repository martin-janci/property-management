/**
 * Layout Preview Bridge Protocol — @ppt/shared
 *
 * Pure TypeScript, zero runtime dependencies, no React, no DOM globals.
 * All DOM access is injected via the `win` parameter (default: globalThis.window).
 *
 * Security invariants:
 * - Child delivers config callbacks ONLY for events whose origin === parentOrigin.
 * - Parent delivers ready/section-click ONLY for events whose origin === childOrigin.
 * - postMessage is always called with the exact counterpart origin as targetOrigin (never '*').
 * - Malformed/foreign messages are silently ignored.
 * - dispose() removes all listeners.
 * - readPreviewParams returns null unless layoutPreview=1 AND parentOrigin survives
 *   a new URL().origin round-trip (and is not "null").
 */

// ============================================
// Constants
// ============================================

export const LAYOUT_PREVIEW_PROTOCOL = 1 as const;
export const LAYOUT_PREVIEW_KIND = 'ppt-layout-preview' as const;

// ============================================
// Shared types
// ============================================

export interface ResolvedSectionLike {
  type: string;
  mode?: string;
  props?: Record<string, unknown>;
  presentation: 'visible' | 'placeholder';
}

export interface ResolvedScreenLike {
  screen: string;
  version: number;
  sections: ResolvedSectionLike[];
}

// ============================================
// Message types
// ============================================

export type PreviewChildMessage =
  | { kind: typeof LAYOUT_PREVIEW_KIND; protocol: 1; type: 'ready'; screen?: string }
  | { kind: typeof LAYOUT_PREVIEW_KIND; protocol: 1; type: 'section-click'; sectionType: string };

export type PreviewParentMessage = {
  kind: typeof LAYOUT_PREVIEW_KIND;
  protocol: 1;
  type: 'config';
  resolved: ResolvedScreenLike;
};

// ============================================
// Type guards
// ============================================

export function isPreviewChildMessage(data: unknown): data is PreviewChildMessage {
  if (data === null || typeof data !== 'object') return false;
  const d = data as Record<string, unknown>;
  if (d['kind'] !== LAYOUT_PREVIEW_KIND) return false;
  if (d['protocol'] !== LAYOUT_PREVIEW_PROTOCOL) return false;
  if (d['type'] === 'ready') return true;
  if (d['type'] === 'section-click') return typeof d['sectionType'] === 'string';
  return false;
}

export function isPreviewParentMessage(data: unknown): data is PreviewParentMessage {
  if (data === null || typeof data !== 'object') return false;
  const d = data as Record<string, unknown>;
  if (d['kind'] !== LAYOUT_PREVIEW_KIND) return false;
  if (d['protocol'] !== LAYOUT_PREVIEW_PROTOCOL) return false;
  if (d['type'] !== 'config') return false;
  const resolved = d['resolved'];
  if (resolved === null || typeof resolved !== 'object' || Array.isArray(resolved)) return false;
  const r = resolved as Record<string, unknown>;
  return (
    typeof r['screen'] === 'string' &&
    typeof r['version'] === 'number' &&
    Array.isArray(r['sections'])
  );
}

// ============================================
// readPreviewParams
// ============================================

/**
 * Read preview params from a URL search string.
 * Returns null when not in preview mode or when parentOrigin is missing/invalid.
 * Never returns a half-on result.
 */
export function readPreviewParams(search: string): { parentOrigin: string } | null {
  const params = new URLSearchParams(search);
  if (params.get('layoutPreview') !== '1') return null;

  const raw = params.get('parentOrigin');
  if (!raw) return null;

  let origin: string;
  try {
    origin = new URL(raw).origin;
  } catch {
    return null;
  }

  // "null" is the serialized origin for opaque origins (e.g. file://)
  if (origin === 'null') return null;

  return { parentOrigin: origin };
}

// ============================================
// connectPreviewChild
// ============================================

/**
 * Child side of the preview bridge.
 *
 * - Registers a message listener on `win`.
 * - Sends a `ready` message to `parent.postMessage` with parentOrigin as targetOrigin.
 * - Delivers `onConfig` ONLY for messages whose origin === parentOrigin.
 * - Ignores malformed or foreign-origin messages.
 * - Returns { sendSectionClick, dispose }.
 */
export function connectPreviewChild(opts: {
  parentOrigin: string;
  screen?: string;
  onConfig: (resolved: ResolvedScreenLike) => void;
  win?: Window;
  parentWindow?: Pick<Window, 'postMessage'>;
}): { sendSectionClick: (sectionType: string) => void; dispose: () => void } {
  const win = opts.win ?? window;
  const parentWin = opts.parentWindow ?? win.parent;

  function handleMessage(event: MessageEvent): void {
    if (event.origin !== opts.parentOrigin) return;
    if (!isPreviewParentMessage(event.data)) return;
    opts.onConfig(event.data.resolved);
  }

  win.addEventListener('message', handleMessage);

  // Send ready immediately after registering the listener
  const readyMsg: PreviewChildMessage = {
    kind: LAYOUT_PREVIEW_KIND,
    protocol: LAYOUT_PREVIEW_PROTOCOL,
    type: 'ready',
    ...(opts.screen !== undefined ? { screen: opts.screen } : {}),
  };
  parentWin.postMessage(readyMsg, opts.parentOrigin);

  return {
    sendSectionClick(sectionType: string): void {
      const msg: PreviewChildMessage = {
        kind: LAYOUT_PREVIEW_KIND,
        protocol: LAYOUT_PREVIEW_PROTOCOL,
        type: 'section-click',
        sectionType,
      };
      parentWin.postMessage(msg, opts.parentOrigin);
    },
    dispose(): void {
      win.removeEventListener('message', handleMessage);
    },
  };
}

// ============================================
// connectPreviewParent
// ============================================

/**
 * Parent side of the preview bridge.
 *
 * - Registers a message listener on `win`.
 * - Delivers `onReady` / `onSectionClick` ONLY for messages whose origin === childOrigin.
 * - Ignores malformed or foreign-origin messages.
 * - `sendConfig` posts to the child window with childOrigin as targetOrigin (never '*').
 *   No-op if childWindow() returns null.
 * - Returns { sendConfig, dispose }.
 */
export function connectPreviewParent(opts: {
  childWindow: () => Window | null;
  childOrigin: string;
  onReady: () => void;
  onSectionClick: (sectionType: string) => void;
  win?: Window;
}): { sendConfig: (resolved: ResolvedScreenLike) => void; dispose: () => void } {
  const win = opts.win ?? window;

  function handleMessage(event: MessageEvent): void {
    if (event.origin !== opts.childOrigin) return;
    if (!isPreviewChildMessage(event.data)) return;
    const msg = event.data;
    if (msg.type === 'ready') {
      opts.onReady();
    } else if (msg.type === 'section-click') {
      opts.onSectionClick(msg.sectionType);
    }
  }

  win.addEventListener('message', handleMessage);

  return {
    sendConfig(resolved: ResolvedScreenLike): void {
      const child = opts.childWindow();
      if (!child) return;
      const msg: PreviewParentMessage = {
        kind: LAYOUT_PREVIEW_KIND,
        protocol: LAYOUT_PREVIEW_PROTOCOL,
        type: 'config',
        resolved,
      };
      child.postMessage(msg, opts.childOrigin);
    },
    dispose(): void {
      win.removeEventListener('message', handleMessage);
    },
  };
}
