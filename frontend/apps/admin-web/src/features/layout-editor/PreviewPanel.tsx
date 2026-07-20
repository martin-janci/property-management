/**
 * PreviewPanel — live preview bridge panel (Task 5, Layout Editor)
 *
 * Allows the operator to point at a running app URL and see the layout
 * resolve in real-time as they edit the draft.
 *
 * House rules (admin-web):
 * - No @ppt/ui-kit imports
 * - Native elements + inline --ppt-* token styles
 * - All strings via t('admin.layout.preview.…', { defaultValue })
 */

import { connectPreviewParent, type ResolvedScreenLike } from '@ppt/shared';
import type React from 'react';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { previewResolve, type ScreenConfig } from './api';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const LS_KEY_PREFIX = 'ppt.layoutPreview.url.';
const DEBOUNCE_MS = 400;

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface Props {
  screen: string;
  platform: 'web' | 'mobile';
  draft: ScreenConfig;
  token: string | null;
  onSectionSelected: (type: string) => void;
}

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const PANEL_STYLE: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: 12,
};

const ROW_STYLE: React.CSSProperties = {
  display: 'flex',
  gap: 8,
  alignItems: 'center',
  flexWrap: 'wrap',
};

const INPUT_STYLE: React.CSSProperties = {
  flex: 1,
  minWidth: 240,
  padding: '6px 10px',
  fontSize: 14,
  borderRadius: 6,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
};

const BTN_STYLE: React.CSSProperties = {
  padding: '6px 14px',
  fontSize: 14,
  borderRadius: 6,
  cursor: 'pointer',
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  fontWeight: 500,
  background: 'var(--ppt-bg-subtle, #f3f4f6)',
  color: 'var(--ppt-fg-primary, #111827)',
};

const ERROR_STYLE: React.CSSProperties = {
  fontSize: 12,
  color: 'var(--ppt-danger-600, #dc2626)',
};

const NOTE_STYLE: React.CSSProperties = {
  fontSize: 12,
  color: 'var(--ppt-warning-700, #b45309)',
};

const IFRAME_STYLE: React.CSSProperties = {
  width: '100%',
  height: 600,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 8,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function buildSrc(rawUrl: string): string {
  const url = new URL(rawUrl);
  url.searchParams.set('layoutPreview', '1');
  url.searchParams.set('parentOrigin', window.location.origin);
  return url.toString();
}

function lsKey(screen: string): string {
  return `${LS_KEY_PREFIX}${screen}`;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function PreviewPanel({ screen, platform, draft, token, onSectionSelected }: Props) {
  const { t } = useTranslation();

  // URL input — persisted to localStorage per screen
  const [urlInput, setUrlInput] = useState<string>(() => {
    try {
      return localStorage.getItem(lsKey(screen)) ?? '';
    } catch {
      return '';
    }
  });

  // Committed (loaded) preview URL — set when Load is clicked
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);

  // Inline errors
  const [urlError, setUrlError] = useState<string | null>(null);
  const [resolveError, setResolveError] = useState<string | null>(null);

  // Whether we have received a "ready" from the child
  const [isReady, setIsReady] = useState(false);

  // Refs
  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const bridgeRef = useRef<{
    sendConfig: (r: ResolvedScreenLike) => void;
    dispose: () => void;
  } | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Stable ref for onSectionSelected so the bridge effect doesn't re-run on every parent render
  const onSectionSelectedRef = useRef(onSectionSelected);
  onSectionSelectedRef.current = onSectionSelected;

  // Persist URL input to localStorage when screen changes
  useEffect(() => {
    try {
      const stored = localStorage.getItem(lsKey(screen));
      setUrlInput(stored ?? '');
    } catch {
      setUrlInput('');
    }
    // When screen changes, drop the loaded URL and bridge
    setPreviewUrl(null);
    setIsReady(false);
    setUrlError(null);
    setResolveError(null);
  }, [screen]);

  // -------------------------------------------------------------------------
  // Bridge lifecycle: mount when previewUrl is set, dispose on change/unmount
  // -------------------------------------------------------------------------

  useEffect(() => {
    if (!previewUrl) return;

    let childOrigin: string;
    try {
      childOrigin = new URL(previewUrl).origin;
    } catch {
      return;
    }

    // Dispose any existing bridge
    bridgeRef.current?.dispose();
    bridgeRef.current = null;
    setIsReady(false);
    setResolveError(null);

    const bridge = connectPreviewParent({
      childWindow: () => iframeRef.current?.contentWindow ?? null,
      childOrigin,
      onReady: () => {
        setIsReady(true);
      },
      onSectionClick: (sectionType: string) => {
        onSectionSelectedRef.current(sectionType);
      },
    });

    bridgeRef.current = bridge;

    return () => {
      bridge.dispose();
      bridgeRef.current = null;
    };
  }, [previewUrl]);

  // -------------------------------------------------------------------------
  // Debounced resolve: fires on ready + draft/platform changes
  // -------------------------------------------------------------------------

  useEffect(() => {
    if (!isReady || !previewUrl) return;

    if (debounceRef.current !== null) {
      clearTimeout(debounceRef.current);
    }

    debounceRef.current = setTimeout(() => {
      debounceRef.current = null;
      previewResolve(token, draft, platform)
        .then((resolved) => {
          setResolveError(null);
          bridgeRef.current?.sendConfig(resolved);
        })
        .catch(() => {
          setResolveError(
            t('admin.layout.preview.resolveError', {
              defaultValue: 'Could not resolve preview config — keeping last pushed.',
            })
          );
        });
    }, DEBOUNCE_MS);

    return () => {
      if (debounceRef.current !== null) {
        clearTimeout(debounceRef.current);
        debounceRef.current = null;
      }
    };
  }, [isReady, draft, platform, token, previewUrl, t]);

  // -------------------------------------------------------------------------
  // Handlers
  // -------------------------------------------------------------------------

  function handleUrlChange(e: React.ChangeEvent<HTMLInputElement>) {
    const val = e.target.value;
    setUrlInput(val);
    try {
      localStorage.setItem(lsKey(screen), val);
    } catch {
      // ignore
    }
    setUrlError(null);
  }

  function handleLoad() {
    // Validate URL
    let parsed: URL;
    try {
      parsed = new URL(urlInput.trim());
      void parsed; // ensure it doesn't throw
    } catch {
      setUrlError(
        t('admin.layout.preview.invalidUrl', {
          defaultValue: 'Please enter a valid URL (e.g. http://localhost:3001)',
        })
      );
      setPreviewUrl(null);
      return;
    }

    setUrlError(null);
    setPreviewUrl(urlInput.trim());
  }

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  const iframeSrc = previewUrl ? buildSrc(previewUrl) : null;

  return (
    <div style={PANEL_STYLE}>
      {/* URL input row */}
      <div style={ROW_STYLE}>
        <input
          type="text"
          data-testid="preview-url-input"
          value={urlInput}
          onChange={handleUrlChange}
          placeholder={t('admin.layout.preview.urlPlaceholder', {
            defaultValue: 'Preview URL (e.g. http://localhost:3001)',
          })}
          style={{
            ...INPUT_STYLE,
            borderColor: urlError
              ? 'var(--ppt-danger-600, #dc2626)'
              : 'var(--ppt-border-default, #e5e7eb)',
          }}
          aria-invalid={urlError !== null}
        />
        <button type="button" data-testid="preview-load-btn" style={BTN_STYLE} onClick={handleLoad}>
          {t('admin.layout.preview.loadBtn', { defaultValue: 'Load' })}
        </button>
      </div>

      {/* Framing hint — always visible below URL input */}
      <div data-testid="preview-framing-hint" style={NOTE_STYLE}>
        {t('admin.layout.preview.framingHint', {
          defaultValue:
            'The framed site must allow embedding from this origin (reality-web: set LAYOUT_PREVIEW_FRAME_ANCESTORS).',
        })}
      </div>

      {/* URL validation error */}
      {urlError && (
        <div data-testid="preview-url-error" style={ERROR_STYLE} role="alert">
          {urlError}
        </div>
      )}

      {/* Resolve error note */}
      {resolveError && (
        <div data-testid="preview-resolve-error" style={NOTE_STYLE} role="status">
          {resolveError}
        </div>
      )}

      {/* iframe */}
      {iframeSrc && (
        <iframe
          ref={iframeRef}
          data-testid="preview-iframe"
          src={iframeSrc}
          style={IFRAME_STYLE}
          sandbox="allow-scripts allow-same-origin allow-forms"
          referrerPolicy="no-referrer"
          title={t('admin.layout.preview.iframeTitle', { defaultValue: 'Live preview' })}
        />
      )}
    </div>
  );
}
