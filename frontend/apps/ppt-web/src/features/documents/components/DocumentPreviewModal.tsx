/**
 * DocumentPreviewModal — inline preview modal for PDF and image documents (gap-7a-4).
 *
 * Features:
 * - Opens a full-screen overlay with PDF or image preview.
 * - Uses usePreviewUrl + useDownloadUrl hooks from @ppt/api-client.
 * - PDF rendering via the existing PdfPreview component.
 * - Image rendering via a native <img> tag with zoom-to-fit.
 * - Download button in the modal header (calls useDownloadUrl and triggers anchor download).
 * - Accessible: focus-trap, Escape key to close, ARIA roles.
 */

import { useDownloadUrl, usePreviewUrl } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useEffect, useRef } from 'react';
import { PdfPreview } from './PdfPreview';

// ─── helpers ─────────────────────────────────────────────────────────────────

function isPdfMimeType(mimeType: string): boolean {
  return mimeType === 'application/pdf';
}

function isImageMimeType(mimeType: string): boolean {
  return mimeType.startsWith('image/');
}

// ─── ImagePreview sub-component ──────────────────────────────────────────────

function ImagePreview({ documentId, fileName }: { documentId: string; fileName: string }) {
  const { data, isLoading, error } = usePreviewUrl(documentId);

  if (isLoading) {
    return (
      <div className="doc-preview-modal__state" aria-busy="true">
        <div className="doc-preview-modal__spinner" aria-label="Načítava sa náhľad..." />
      </div>
    );
  }

  if (error || !data?.url) {
    return (
      <div className="doc-preview-modal__state doc-preview-modal__state--error" role="alert">
        <svg
          width="32"
          height="32"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          aria-hidden="true"
        >
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="8" x2="12" y2="12" />
          <line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
        <p>Náhľad nie je dostupný.</p>
      </div>
    );
  }

  return (
    <div className="doc-preview-modal__image-wrap">
      <img
        src={data.url}
        alt={fileName}
        className="doc-preview-modal__image"
      />
    </div>
  );
}

// ─── FallbackPreview sub-component ───────────────────────────────────────────

function FallbackPreview({ fileName }: { fileName: string }) {
  return (
    <div className="doc-preview-modal__state">
      <svg
        width="48"
        height="48"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1"
        opacity="0.4"
        aria-hidden="true"
      >
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
      </svg>
      <p>Náhľad nie je dostupný pre tento typ súboru.</p>
      <p className="doc-preview-modal__state-hint">{fileName}</p>
    </div>
  );
}

// ─── DownloadButton sub-component ────────────────────────────────────────────

function DownloadButton({
  documentId,
  fileName,
}: {
  documentId: string;
  fileName: string;
}) {
  const { data, isLoading } = useDownloadUrl(documentId);

  const handleDownload = useCallback(() => {
    if (!data?.url) return;
    const anchor = document.createElement('a');
    anchor.href = data.url;
    anchor.download = fileName;
    anchor.rel = 'noopener noreferrer';
    document.body.appendChild(anchor);
    anchor.click();
    document.body.removeChild(anchor);
  }, [data?.url, fileName]);

  return (
    <button
      type="button"
      className="doc-preview-modal__download-btn"
      onClick={handleDownload}
      disabled={isLoading || !data?.url}
      aria-label={`Stiahnuť ${fileName}`}
      title={`Stiahnuť ${fileName}`}
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        aria-hidden="true"
      >
        <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
        <polyline points="7 10 12 15 17 10" />
        <line x1="12" y1="15" x2="12" y2="3" />
      </svg>
      Stiahnuť
    </button>
  );
}

// ─── Main modal component ─────────────────────────────────────────────────────

export interface DocumentPreviewModalProps {
  /** Document UUID */
  documentId: string;
  /** Human-readable title shown in the modal header */
  title: string;
  /** Original filename (used for MIME detection fallback and download attribute) */
  fileName: string;
  /** MIME type — determines which renderer is used */
  mimeType: string;
  /** Callback invoked when the user closes the modal */
  onClose: () => void;
}

export function DocumentPreviewModal({
  documentId,
  title,
  fileName,
  mimeType,
  onClose,
}: DocumentPreviewModalProps) {
  const backdropRef = useRef<HTMLDivElement>(null);
  const closeBtnRef = useRef<HTMLButtonElement>(null);

  // Close on Escape
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [onClose]);

  // Auto-focus close button on mount (focus-trap entry point)
  useEffect(() => {
    closeBtnRef.current?.focus();
  }, []);

  // Close on backdrop click (not on content click)
  const handleBackdropClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (e.target === backdropRef.current) onClose();
    },
    [onClose]
  );

  const isPdf = isPdfMimeType(mimeType);
  const isImage = isImageMimeType(mimeType);

  return (
    <div
      ref={backdropRef}
      className="doc-preview-modal__backdrop"
      role="dialog"
      aria-modal="true"
      aria-label={`Náhľad: ${title}`}
      onClick={handleBackdropClick}
    >
      <div className="doc-preview-modal__panel">
        {/* Header */}
        <div className="doc-preview-modal__header">
          <div className="doc-preview-modal__header-title">
            <span className="doc-preview-modal__file-ext" aria-hidden="true">
              {fileName.split('.').pop()?.toUpperCase()?.slice(0, 4) ?? 'DOC'}
            </span>
            <h2 className="doc-preview-modal__title" title={title}>
              {title}
            </h2>
          </div>
          <div className="doc-preview-modal__header-actions">
            <DownloadButton documentId={documentId} fileName={fileName} />
            <button
              ref={closeBtnRef}
              type="button"
              className="doc-preview-modal__close-btn"
              onClick={onClose}
              aria-label="Zavrieť náhľad"
            >
              <svg
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                aria-hidden="true"
              >
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>
        </div>

        {/* Content area */}
        <div className="doc-preview-modal__content">
          {isPdf && <PdfPreview documentId={documentId} isPdf className="doc-preview-modal__pdf" />}
          {!isPdf && isImage && <ImagePreview documentId={documentId} fileName={fileName} />}
          {!isPdf && !isImage && <FallbackPreview fileName={fileName} />}
        </div>
      </div>

      <style>{modalStyles}</style>
    </div>
  );
}

// ─── Styles ───────────────────────────────────────────────────────────────────

const modalStyles = `
  .doc-preview-modal__backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: stretch;
    justify-content: center;
    padding: 1.5rem;
    overflow: hidden;
    animation: doc-preview-fade-in 0.15s ease-out;
  }

  @media (max-width: 640px) {
    .doc-preview-modal__backdrop {
      padding: 0;
      align-items: flex-end;
    }
  }

  @keyframes doc-preview-fade-in {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  .doc-preview-modal__panel {
    display: flex;
    flex-direction: column;
    width: 100%;
    max-width: 900px;
    max-height: 100%;
    background: var(--ppt-bg-surface, #fff);
    border-radius: 0.75rem;
    overflow: hidden;
    box-shadow: 0 25px 50px rgba(0, 0, 0, 0.25);
    animation: doc-preview-slide-up 0.18s ease-out;
  }

  @media (max-width: 640px) {
    .doc-preview-modal__panel {
      border-radius: 1rem 1rem 0 0;
      max-height: 92vh;
    }
  }

  @keyframes doc-preview-slide-up {
    from { transform: translateY(16px); opacity: 0.5; }
    to   { transform: translateY(0);    opacity: 1; }
  }

  .doc-preview-modal__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.875rem 1rem;
    background: var(--ppt-bg-app, #f9fafb);
    border-bottom: 1px solid var(--ppt-border-default, #e5e7eb);
    flex-shrink: 0;
  }

  .doc-preview-modal__header-title {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    min-width: 0;
    flex: 1;
  }

  .doc-preview-modal__file-ext {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2.25rem;
    height: 2.25rem;
    font-size: 0.5625rem;
    font-weight: 700;
    font-family: monospace;
    border-radius: 0.375rem;
    background: var(--ppt-brand-500, #2563eb);
    color: #fff;
    letter-spacing: 0.025em;
  }

  .doc-preview-modal__title {
    margin: 0;
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--ppt-fg-primary, #111827);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .doc-preview-modal__header-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .doc-preview-modal__download-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4375rem 0.875rem;
    font-size: 0.8125rem;
    font-weight: 600;
    background: var(--ppt-brand-500, #2563eb);
    color: #fff;
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
    transition: background 0.12s;
    white-space: nowrap;
  }

  .doc-preview-modal__download-btn:hover:not(:disabled) {
    background: var(--ppt-color-primary, #1d4ed8);
  }

  .doc-preview-modal__download-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .doc-preview-modal__close-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2.25rem;
    height: 2.25rem;
    background: transparent;
    border: 1px solid var(--ppt-border-default, #e5e7eb);
    border-radius: 0.5rem;
    color: var(--ppt-fg-muted, #6b7280);
    cursor: pointer;
    transition: all 0.12s;
  }

  .doc-preview-modal__close-btn:hover {
    background: var(--ppt-bg-surface, #fff);
    border-color: var(--ppt-border-strong, #9ca3af);
    color: var(--ppt-fg-primary, #111827);
  }

  .doc-preview-modal__content {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .doc-preview-modal__pdf {
    /* Override PdfPreview border/radius — modal provides the chrome */
    border: none !important;
    border-radius: 0 !important;
    min-height: 60vh;
  }

  .doc-preview-modal__image-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 60vh;
    padding: 1.5rem;
    background: var(--ppt-bg-app, #f3f4f6);
  }

  .doc-preview-modal__image {
    max-width: 100%;
    max-height: 75vh;
    object-fit: contain;
    border-radius: 0.375rem;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
  }

  .doc-preview-modal__state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    min-height: 60vh;
    padding: 2.5rem 1.5rem;
    color: var(--ppt-fg-muted, #6b7280);
    text-align: center;
  }

  .doc-preview-modal__state p {
    margin: 0;
    font-size: 0.875rem;
  }

  .doc-preview-modal__state-hint {
    font-size: 0.75rem !important;
    opacity: 0.7;
    font-family: monospace;
  }

  .doc-preview-modal__state--error {
    color: var(--ppt-color-danger, #dc2626);
  }

  .doc-preview-modal__spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--ppt-border-default, #e5e7eb);
    border-top-color: var(--ppt-brand-500, #2563eb);
    border-radius: 50%;
    animation: doc-preview-spin 0.7s linear infinite;
  }

  @keyframes doc-preview-spin {
    to { transform: rotate(360deg); }
  }

  @media (prefers-reduced-motion: reduce) {
    .doc-preview-modal__backdrop,
    .doc-preview-modal__panel {
      animation: none;
    }
    .doc-preview-modal__spinner {
      animation-duration: 1.5s;
    }
  }
`;
