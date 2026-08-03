/**
 * DocumentPreviewModal (gap-7a-4).
 *
 * Inline preview overlay for PDF and image documents.
 * Renders PDF via PdfPreview (react-pdf), images via <img>, and a
 * fallback download prompt for other mime types.
 *
 * Accessibility:
 *  - role="dialog" + aria-modal + aria-labelledby on the panel, not the backdrop (ARIA 1.2 §3.15)
 *  - useId() for collision-safe aria-labelledby across concurrent instances
 *  - Focus trap + Escape key + focus-restore via useFocusTrap (WCAG 2.4.3)
 *  - Backdrop click closes (click must target the backdrop itself)
 *  - Body scroll locked while open
 *  - z-index: 1050 — above toasts (1000) and FolderTree overlay (1000),
 *    below skip-nav (9999) and focus-visible outlines (10000-10001)
 */

import { usePreviewUrl } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useEffect, useId, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useFocusTrap } from '../../../hooks/useFocusTrap';
import { useDocumentDownload } from '../hooks/useDocumentDownload';
import { PdfPreview } from './PdfPreview';

// ─── types ────────────────────────────────────────────────────────────────────

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

// ─── sub-components ───────────────────────────────────────────────────────────

interface ImagePreviewProps {
  documentId: string;
  title: string;
}

function ImagePreview({ documentId, title }: ImagePreviewProps) {
  const { t } = useTranslation();
  const { data, isLoading, error } = usePreviewUrl(documentId);

  if (isLoading) {
    return (
      <div className="doc-preview__loading" aria-busy="true" aria-label={t('aria.loadingPreview')}>
        <div className="doc-preview__spinner" />
        <p>Načítavanie náhľadu…</p>
      </div>
    );
  }

  if (error || !data?.url) {
    return (
      <div className="doc-preview__fallback">
        <p>Náhľad nie je dostupný.</p>
      </div>
    );
  }

  return (
    <div className="doc-preview__image-wrap">
      <img src={data.url} alt={title} className="doc-preview__image" />
    </div>
  );
}

function FallbackPreview({ fileName }: { fileName: string }) {
  return (
    <div className="doc-preview__fallback">
      <svg
        width="48"
        height="48"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        aria-hidden="true"
        className="doc-preview__fallback-icon"
      >
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
      </svg>
      <p className="doc-preview__fallback-name">{fileName}</p>
      <p className="doc-preview__fallback-msg">
        Náhľad nie je pre tento typ súboru k dispozícii. Stiahnite si súbor pre zobrazenie.
      </p>
    </div>
  );
}

interface DownloadButtonProps {
  documentId: string;
  fileName: string;
  className?: string;
}

function DownloadButton({
  documentId,
  fileName,
  className = 'doc-preview__download-btn',
}: DownloadButtonProps) {
  const { download, isDownloading } = useDocumentDownload(documentId, fileName);

  return (
    <button
      type="button"
      className={className}
      onClick={download}
      disabled={isDownloading}
      aria-label={`Stiahnuť ${fileName}`}
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
        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
        <polyline points="7 10 12 15 17 10" />
        <line x1="12" y1="15" x2="12" y2="3" />
      </svg>
      {isDownloading ? 'Sťahujem…' : 'Stiahnuť'}
    </button>
  );
}

// ─── main component ───────────────────────────────────────────────────────────

const isPdf = (mimeType: string) => mimeType === 'application/pdf';
const isImage = (mimeType: string) => mimeType.startsWith('image/');

export function DocumentPreviewModal({
  documentId,
  title,
  fileName,
  mimeType,
  onClose,
}: DocumentPreviewModalProps) {
  const { t } = useTranslation();
  const titleId = useId();
  const backdropRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  // Focus trap: Tab/Shift+Tab cycle within the panel, Escape calls onClose,
  // and focus is restored to the triggering element on unmount (WCAG 2.4.3).
  useFocusTrap(panelRef, onClose);

  // Lock body scroll while the modal is open.
  useEffect(() => {
    const prev = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = prev;
    };
  }, []);

  const handleBackdropClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (e.target === backdropRef.current) onClose();
    },
    [onClose]
  );

  return (
    <div ref={backdropRef} className="doc-preview-backdrop" onClick={handleBackdropClick}>
      {/* role="dialog" must be on the panel element, not the backdrop (ARIA 1.2 §3.15) */}
      <div
        ref={panelRef}
        className="doc-preview-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
      >
        {/* Header */}
        <div className="doc-preview__header">
          <span id={titleId} className="doc-preview__title" title={title}>
            {title}
          </span>
          <div className="doc-preview__header-actions">
            <DownloadButton documentId={documentId} fileName={fileName} />
            <button
              type="button"
              className="doc-preview__close-btn"
              onClick={onClose}
              aria-label={t('aria.closePreview')}
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

        {/* Content */}
        <div className="doc-preview__body">
          {isPdf(mimeType) ? (
            <PdfPreview documentId={documentId} isPdf className="doc-preview__pdf" />
          ) : isImage(mimeType) ? (
            <ImagePreview documentId={documentId} title={title} />
          ) : (
            <FallbackPreview fileName={fileName} />
          )}
        </div>
      </div>

      <style>{`
        .doc-preview-backdrop {
          position: fixed;
          inset: 0;
          z-index: 1050;
          background: rgba(0, 0, 0, 0.65);
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 1rem;
          animation: doc-preview-fade-in 0.15s ease-out;
        }

        @keyframes doc-preview-fade-in {
          from { opacity: 0; }
          to   { opacity: 1; }
        }

        .doc-preview-modal {
          display: flex;
          flex-direction: column;
          width: min(960px, 100%);
          max-height: calc(100vh - 2rem);
          background: var(--ppt-bg-surface);
          border-radius: 0.75rem;
          box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
          overflow: hidden;
          outline: none;
          animation: doc-preview-slide-up 0.15s ease-out;
        }

        @keyframes doc-preview-slide-up {
          from { transform: translateY(12px); opacity: 0; }
          to   { transform: translateY(0);   opacity: 1; }
        }

        .doc-preview__header {
          display: flex;
          align-items: center;
          gap: 0.75rem;
          padding: 0.875rem 1rem;
          border-bottom: 1px solid var(--ppt-border-default);
          flex-shrink: 0;
          background: var(--ppt-bg-surface);
        }

        .doc-preview__title {
          flex: 1;
          min-width: 0;
          font-size: 0.9375rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }

        .doc-preview__header-actions {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          flex-shrink: 0;
        }

        .doc-preview__download-btn {
          display: inline-flex;
          align-items: center;
          gap: 0.375rem;
          padding: 0.375rem 0.875rem;
          font-size: 0.875rem;
          font-weight: 500;
          background: var(--ppt-brand-500);
          color: #fff;
          border: none;
          border-radius: 0.375rem;
          cursor: pointer;
          transition: background 0.15s;
        }

        .doc-preview__download-btn:hover:not(:disabled) {
          background: var(--ppt-brand-600, var(--ppt-brand-500));
        }

        .doc-preview__download-btn:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .doc-preview__download-btn:focus-visible {
          outline: 2px solid var(--ppt-brand-500);
          outline-offset: 2px;
        }

        .doc-preview__close-btn {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 2rem;
          height: 2rem;
          background: transparent;
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.375rem;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          transition: all 0.15s;
        }

        .doc-preview__close-btn:hover {
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-primary);
        }

        .doc-preview__close-btn:focus-visible {
          outline: 2px solid var(--ppt-brand-500);
          outline-offset: 2px;
        }

        .doc-preview__body {
          flex: 1;
          overflow: auto;
          min-height: 0;
        }

        /* PDF */
        .doc-preview__pdf {
          width: 100%;
          height: 100%;
        }

        /* Image */
        .doc-preview__image-wrap {
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 1rem;
          min-height: 300px;
        }

        .doc-preview__image {
          max-width: 100%;
          max-height: calc(100vh - 12rem);
          object-fit: contain;
          border-radius: 0.375rem;
        }

        /* Fallback / error */
        .doc-preview__fallback {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          gap: 0.75rem;
          padding: 3rem 2rem;
          text-align: center;
          color: var(--ppt-fg-muted);
          min-height: 240px;
        }

        .doc-preview__fallback-icon {
          color: var(--ppt-fg-subtle);
        }

        .doc-preview__fallback-name {
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .doc-preview__fallback-msg {
          font-size: 0.875rem;
          max-width: 36ch;
        }

        /* Loading */
        .doc-preview__loading {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          gap: 0.75rem;
          padding: 3rem 2rem;
          color: var(--ppt-fg-muted);
          min-height: 240px;
          font-size: 0.875rem;
        }

        .doc-preview__spinner {
          width: 2rem;
          height: 2rem;
          border: 3px solid var(--ppt-border-default);
          border-top-color: var(--ppt-brand-500);
          border-radius: 50%;
          animation: doc-preview-spin 0.7s linear infinite;
        }

        @keyframes doc-preview-spin {
          to { transform: rotate(360deg); }
        }

        /* Responsive */
        @media (max-width: 640px) {
          .doc-preview-backdrop {
            padding: 0;
            align-items: flex-end;
          }

          .doc-preview-modal {
            width: 100%;
            max-height: 92vh;
            border-radius: 1rem 1rem 0 0;
          }
        }

        /* Reduced motion */
        @media (prefers-reduced-motion: reduce) {
          .doc-preview-backdrop,
          .doc-preview-modal {
            animation: none;
          }

          .doc-preview__spinner {
            animation: none;
            border-top-color: var(--ppt-brand-500);
          }
        }
      `}</style>
    </div>
  );
}
