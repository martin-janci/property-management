/**
 * PdfPreview — client-side PDF rendering via react-pdf (pdfjs-dist).
 *
 * Features:
 * - Lazy-loads only when the component enters the viewport (IntersectionObserver).
 * - Page navigator (prev/next + current/total indicator).
 * - Zoom controls (− / reset / +, clamped 50–200 %).
 * - Fetches presigned preview URL from the backend (usePreviewUrl hook).
 * - Accessible: ARIA labels on all icon-only buttons.
 */

import { usePreviewUrl } from '@ppt/api-client';
import { useCallback, useEffect, useRef, useState } from 'react';

// react-pdf + worker configuration.
// pdfjs.GlobalWorkerOptions.workerSrc must be set before any Document is
// mounted. Using `new URL(..., import.meta.url)` lets Vite/Rollup resolve
// and hash the worker asset at build time.
import type { DocumentProps } from 'react-pdf';
import { Document as PdfDocument, Page as PdfPage, pdfjs } from 'react-pdf';
import 'react-pdf/dist/Page/AnnotationLayer.css';
import 'react-pdf/dist/Page/TextLayer.css';
import { useTranslation } from 'react-i18next';

// LoadedPdf is the type of the `pdf` argument in onLoadSuccess.
// react-pdf re-exports it internally but does not expose it from the package
// root. We infer it from the prop type to stay dependency-safe.
type LoadedPdf = NonNullable<Parameters<NonNullable<DocumentProps['onLoadSuccess']>>[0]>;

pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  'pdfjs-dist/build/pdf.worker.mjs',
  import.meta.url
).toString();

// --- Types ---

interface PdfPreviewProps {
  /** Document UUID — used to fetch the presigned preview URL. */
  documentId: string;
  /** Whether the document is a PDF (other mime types show a non-PDF fallback). */
  isPdf?: boolean;
  /** Optional CSS class applied to the outermost element. */
  className?: string;
}

// --- Public component ---

export function PdfPreview({ documentId, isPdf = true, className }: PdfPreviewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [isVisible, setIsVisible] = useState(false);

  // Lazy-load trigger: only mount the heavy PDF renderer once the pane enters
  // the viewport. This keeps initial LCP fast for users who never scroll down.
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setIsVisible(true);
          observer.disconnect();
        }
      },
      { threshold: 0.1 }
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <div ref={containerRef} className={`pdf-preview-root${className ? ` ${className}` : ''}`}>
      {isVisible ? <PdfRenderer documentId={documentId} isPdf={isPdf} /> : <PdfPlaceholder />}
      <style>{previewStyles}</style>
    </div>
  );
}

// --- Inner renderer (only mounted once in-viewport) ---

function PdfRenderer({ documentId, isPdf }: { documentId: string; isPdf: boolean }) {
  const { t } = useTranslation();
  const { data: urlData, isLoading: urlLoading, error: urlError } = usePreviewUrl(documentId);

  const [numPages, setNumPages] = useState<number>(0);
  const [pageNumber, setPageNumber] = useState<number>(1);
  const [scale, setScale] = useState<number>(1.0);
  const [docError, setDocError] = useState<string | null>(null);

  const handleDocumentLoadSuccess = useCallback((pdf: LoadedPdf) => {
    setNumPages(pdf.numPages);
    setPageNumber(1);
    setDocError(null);
  }, []);

  const handleDocumentLoadError = useCallback((err: Error) => {
    setDocError(err.message);
  }, []);

  const goToPrev = useCallback(() => setPageNumber((p) => Math.max(1, p - 1)), []);
  const goToNext = useCallback(() => setPageNumber((p) => Math.min(numPages, p + 1)), [numPages]);
  const zoomIn = useCallback(
    () => setScale((s) => Math.min(2.0, parseFloat((s + 0.1).toFixed(1)))),
    []
  );
  const zoomOut = useCallback(
    () => setScale((s) => Math.max(0.5, parseFloat((s - 0.1).toFixed(1)))),
    []
  );
  const zoomReset = useCallback(() => setScale(1.0), []);

  if (urlLoading) return <PdfPlaceholder loading />;

  if (urlError || !urlData?.url) {
    return (
      <div className="pdf-state pdf-error" role="alert">
        <svg
          width="24"
          height="24"
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
        <p>Nahled dokumentu nie je dostupny.</p>
      </div>
    );
  }

  if (!isPdf) {
    return (
      <div className="pdf-state pdf-no-preview">
        <svg
          width="32"
          height="32"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          aria-hidden="true"
        >
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
          <polyline points="14 2 14 8 20 8" />
        </svg>
        <p>Nahled nie je dostupny pre tento typ suboru.</p>
        <a
          href={urlData.url}
          target="_blank"
          rel="noopener noreferrer"
          className="pdf-download-link"
        >
          Stiahnut subor
        </a>
      </div>
    );
  }

  return (
    <div className="pdf-renderer">
      <div className="pdf-toolbar" role="toolbar" aria-label={t('aria.pdfNavigation')}>
        <div className="pdf-toolbar-group">
          <button
            type="button"
            onClick={goToPrev}
            disabled={pageNumber <= 1}
            aria-label={t('aria.previousPage')}
            className="pdf-icon-btn"
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
              <polyline points="15 18 9 12 15 6" />
            </svg>
          </button>
          <span className="pdf-page-indicator" aria-live="polite">
            <strong>{pageNumber}</strong>
            {' z '}
            <strong>{numPages || '...'}</strong>
          </span>
          <button
            type="button"
            onClick={goToNext}
            disabled={pageNumber >= numPages}
            aria-label={t('aria.nextPage')}
            className="pdf-icon-btn"
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
              <polyline points="9 18 15 12 9 6" />
            </svg>
          </button>
        </div>

        <div className="pdf-toolbar-group">
          <button
            type="button"
            onClick={zoomOut}
            disabled={scale <= 0.5}
            aria-label={t('aria.zoomOut')}
            className="pdf-icon-btn"
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
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
              <line x1="8" y1="11" x2="14" y2="11" />
            </svg>
          </button>
          <button
            type="button"
            onClick={zoomReset}
            aria-label={t('aria.resetZoom')}
            className="pdf-zoom-label"
          >
            {Math.round(scale * 100)}%
          </button>
          <button
            type="button"
            onClick={zoomIn}
            disabled={scale >= 2.0}
            aria-label={t('aria.zoomIn')}
            className="pdf-icon-btn"
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
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
              <line x1="11" y1="8" x2="11" y2="14" />
              <line x1="8" y1="11" x2="14" y2="11" />
            </svg>
          </button>
        </div>
      </div>

      {docError && (
        <div className="pdf-state pdf-error" role="alert">
          <p>Chyba pri nacitani PDF: {docError}</p>
        </div>
      )}

      <div className="pdf-canvas-area" aria-label={t('aria.pdfPreview')}>
        <PdfDocument
          file={urlData.url}
          onLoadSuccess={handleDocumentLoadSuccess}
          onLoadError={handleDocumentLoadError}
          loading={<PdfPlaceholder loading />}
          error={null}
        >
          <PdfPage
            pageNumber={pageNumber}
            scale={scale}
            renderTextLayer
            renderAnnotationLayer
            loading={<PdfPlaceholder loading small />}
          />
        </PdfDocument>
      </div>
    </div>
  );
}

// --- Skeleton placeholder ---

function PdfPlaceholder({
  loading = false,
  small = false,
}: {
  loading?: boolean;
  small?: boolean;
}) {
  return (
    <div
      className={`pdf-placeholder${small ? ' pdf-placeholder--small' : ''}`}
      aria-busy={loading}
      aria-label={loading ? 'Nacitava sa nahled...' : 'Nahled dokumentu'}
    >
      <div className="pdf-placeholder-page">
        {loading && <div className="pdf-shimmer" />}
        {!loading && (
          <svg
            width="48"
            height="48"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1"
            opacity="0.3"
            aria-hidden="true"
          >
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
          </svg>
        )}
      </div>
    </div>
  );
}

// --- Styles ---

const previewStyles = `
  .pdf-preview-root {
    display: flex;
    flex-direction: column;
    background: var(--ppt-bg-surface, #fff);
    border: 1px solid var(--ppt-border-default, #e5e7eb);
    border-radius: 0.5rem;
    overflow: hidden;
  }

  .pdf-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 0.75rem;
    background: var(--ppt-bg-app, #f9fafb);
    border-bottom: 1px solid var(--ppt-border-default, #e5e7eb);
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .pdf-toolbar-group {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .pdf-icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    padding: 0;
    background: transparent;
    border: 1px solid var(--ppt-border-default, #e5e7eb);
    border-radius: 0.375rem;
    color: var(--ppt-fg-secondary, #374151);
    cursor: pointer;
    transition: background 0.12s, border-color 0.12s;
  }

  .pdf-icon-btn:hover:not(:disabled) {
    background: var(--ppt-bg-surface, #fff);
    border-color: var(--ppt-border-strong, #9ca3af);
  }

  .pdf-icon-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .pdf-page-indicator {
    font-size: 0.8125rem;
    color: var(--ppt-fg-secondary, #374151);
    min-width: 6ch;
    text-align: center;
    user-select: none;
  }

  .pdf-zoom-label {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--ppt-fg-secondary, #374151);
    min-width: 3.5rem;
    text-align: center;
    padding: 0.25rem 0.375rem;
    background: var(--ppt-bg-surface, #fff);
    border: 1px solid var(--ppt-border-default, #e5e7eb);
    border-radius: 0.375rem;
    cursor: pointer;
    transition: background 0.12s;
  }

  .pdf-zoom-label:hover {
    background: var(--ppt-bg-app, #f9fafb);
  }

  .pdf-canvas-area {
    overflow: auto;
    display: flex;
    justify-content: center;
    padding: 1rem;
    min-height: 400px;
    background: var(--ppt-bg-app, #f3f4f6);
  }

  .pdf-canvas-area .react-pdf__Document {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .pdf-canvas-area .react-pdf__Page {
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.12);
  }

  .pdf-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: 2.5rem 1.5rem;
    font-size: 0.875rem;
    color: var(--ppt-fg-muted, #6b7280);
    text-align: center;
    min-height: 200px;
  }

  .pdf-state p {
    margin: 0;
  }

  .pdf-error {
    color: var(--ppt-color-danger, #dc2626);
  }

  .pdf-download-link {
    font-size: 0.875rem;
    color: var(--ppt-brand-500, #2563eb);
    text-decoration: underline;
    cursor: pointer;
  }

  .pdf-placeholder {
    min-height: 400px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    background: var(--ppt-bg-app, #f3f4f6);
  }

  .pdf-placeholder--small {
    min-height: 120px;
  }

  .pdf-placeholder-page {
    width: 100%;
    max-width: 480px;
    aspect-ratio: 1 / 1.414;
    background: var(--ppt-bg-surface, #fff);
    border-radius: 0.25rem;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    position: relative;
  }

  .pdf-shimmer {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      var(--ppt-bg-surface, #fff) 0%,
      var(--ppt-border-default, #e5e7eb) 50%,
      var(--ppt-bg-surface, #fff) 100%
    );
    background-size: 200% 100%;
    animation: pdf-shimmer 1.4s ease-in-out infinite;
  }

  @keyframes pdf-shimmer {
    0%   { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
`;
