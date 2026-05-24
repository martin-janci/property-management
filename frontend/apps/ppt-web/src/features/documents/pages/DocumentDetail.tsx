/**
 * Document Detail Page (Epic 39).
 *
 * Shows full document details with PDF preview (UC-08 gap-7a-4) and AI intelligence.
 */

import { useDocument, useDocumentClassification, useReprocessOcr } from '@ppt/api-client';
import { ClassificationUI } from '../components/ClassificationBadge';
import { DocumentSummary } from '../components/DocumentSummary';
import { OcrProcessingStatus } from '../components/OcrStatusBadge';
import { PdfPreview } from '../components/PdfPreview';

/** Returns true for mime types that can be rendered client-side by PDF.js. */
function isSupportedPdfMime(mimeType: string): boolean {
  return mimeType === 'application/pdf';
}

interface DocumentDetailProps {
  documentId: string;
}

export function DocumentDetail({ documentId }: DocumentDetailProps) {
  const { data, isLoading, error, refetch } = useDocument(documentId);
  const classification = useDocumentClassification(documentId);
  const reprocessOcr = useReprocessOcr();

  if (isLoading) {
    return (
      <div className="document-detail loading">
        <div className="loading-spinner" />
        <p>Loading document...</p>

        <style>{detailStyles}</style>
      </div>
    );
  }

  if (error) {
    const isForbidden =
      error.message.includes('403') || error.message.toLowerCase().includes('forbidden');
    if (isForbidden) {
      return (
        <div className="document-detail permission-denied">
          <svg
            width="40"
            height="40"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            aria-hidden="true"
            className="permission-icon"
          >
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
            <path d="M7 11V7a5 5 0 0110 0v4" />
          </svg>
          <p className="permission-message">Tento dokument nie je pristupny pre vas ucet.</p>
          <p className="permission-hint">
            Dokument mohol byt nastaveny ako viditelny iba pre spravnikov alebo konkretnych
            pouzivatelov.
          </p>

          <style>{detailStyles}</style>
        </div>
      );
    }

    return (
      <div className="document-detail error">
        <p className="error-message">Detail dokumentu sa nepodarilo nacitat.</p>
        <button type="button" onClick={() => refetch()} className="retry-btn">
          Skusit znova
        </button>

        <style>{detailStyles}</style>
      </div>
    );
  }

  if (!data?.document) {
    return (
      <div className="document-detail empty">
        <p>Dokument nebol najdeny.</p>

        <style>{detailStyles}</style>
      </div>
    );
  }

  const doc = data.document;

  const formatSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    return date.toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const wordCount = doc.ocr_text ? doc.ocr_text.split(/\s+/).filter((w) => w.length > 0).length : 0;

  return (
    <div className="document-detail">
      {/* Header */}
      <div className="detail-content">
        <h2 className="document-title">{doc.title}</h2>

        <div className="document-meta">
          <span className="meta-item category">{doc.category}</span>
          <span className="meta-separator">|</span>
          <span className="meta-item">{doc.file_name}</span>
          <span className="meta-separator">|</span>
          <span className="meta-item">{formatSize(doc.size_bytes)}</span>
        </div>

        {doc.description && <p className="document-description">{doc.description}</p>}

        <div className="document-dates">
          <p>Created: {formatDate(doc.created_at)}</p>
          <p>Updated: {formatDate(doc.updated_at)}</p>
        </div>

        <div className="document-actions">
          <a
            href={`/api/v1/documents/${doc.id}/download`}
            target="_blank"
            rel="noopener noreferrer"
            className="action-btn primary"
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
            Download
          </a>
        </div>
      </div>

      {/* PDF Preview (UC-08 gap-7a-4) */}
      <div className="preview-section">
        <h3 className="section-title preview-section-title">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
            <circle cx="12" cy="12" r="3" />
          </svg>
          Nahled dokumentu
        </h3>
        <PdfPreview
          documentId={doc.id}
          isPdf={isSupportedPdfMime(doc.mime_type)}
          className="document-pdf-preview"
        />
      </div>

      {/* Intelligence Section */}
      <div className="intelligence-section">
        <h3 className="section-title">
          <span className="ai-icon">AI</span>
          Document Intelligence
        </h3>

        <div className="intelligence-card">
          <DocumentSummary
            documentId={doc.id}
            summary={doc.summary}
            summaryGeneratedAt={doc.summary_generated_at}
            wordCount={wordCount}
            onSummaryGenerated={() => refetch()}
          />
        </div>

        {doc.ocr_status && doc.ocr_status !== 'not_applicable' && (
          <div className="intelligence-card">
            <OcrProcessingStatus
              status={doc.ocr_status}
              processedAt={doc.ocr_processed_at}
              onReprocess={() => reprocessOcr.mutate(doc.id)}
              isReprocessing={reprocessOcr.isPending}
            />
          </div>
        )}

        {classification.data && (
          <div className="intelligence-card">
            <ClassificationUI
              documentId={doc.id}
              classification={classification.data}
              onFeedbackSubmitted={() => {
                classification.refetch();
                refetch();
              }}
            />
          </div>
        )}

        {doc.ocr_text && (
          <div className="ocr-preview">
            <h4 className="preview-title">Extracted Text</h4>
            <p className="word-count">{wordCount} words extracted</p>
            <div className="ocr-text-container">
              <pre className="ocr-text">{doc.ocr_text}</pre>
            </div>
          </div>
        )}
      </div>

      <style>{detailStyles}</style>
    </div>
  );
}

const detailStyles = `
  .document-detail {
    padding: 1.5rem;
  }

  .document-detail.loading,
  .document-detail.error,
  .document-detail.empty,
  .document-detail.permission-denied {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 200px;
    color: var(--ppt-fg-muted);
    text-align: center;
    padding: 2rem;
  }

  .permission-icon {
    color: var(--ppt-fg-subtle);
    margin-bottom: 1rem;
  }

  .permission-message {
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--ppt-fg-primary);
    margin: 0 0 0.5rem;
  }

  .permission-hint {
    font-size: 0.8125rem;
    color: var(--ppt-fg-muted);
    max-width: 28rem;
    line-height: 1.6;
    margin: 0;
  }

  .loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--ppt-border-default);
    border-top-color: var(--ppt-brand-500);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error-message {
    color: var(--ppt-color-danger);
    margin-bottom: 1rem;
  }

  .retry-btn {
    padding: 0.5rem 1rem;
    background: var(--ppt-brand-500);
    color: var(--ppt-fg-on-accent);
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
  }

  .detail-content {
    margin-bottom: 2rem;
    padding-bottom: 1.5rem;
    border-bottom: 1px solid var(--ppt-border-default);
  }

  .document-title {
    margin: 0 0 0.5rem;
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--ppt-fg-primary);
  }

  .document-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: var(--ppt-fg-muted);
    margin-bottom: 1rem;
  }

  .meta-separator {
    color: var(--ppt-border-strong);
  }

  .meta-item.category {
    padding: 0.125rem 0.5rem;
    background: var(--ppt-bg-app);
    border-radius: 0.25rem;
    font-weight: 500;
  }

  .document-description {
    margin: 0 0 1rem;
    font-size: 0.875rem;
    color: var(--ppt-fg-secondary);
    line-height: 1.6;
  }

  .document-dates {
    font-size: 0.75rem;
    color: var(--ppt-fg-subtle);
  }

  .document-dates p {
    margin: 0.25rem 0;
  }

  .document-actions {
    display: flex;
    gap: 0.75rem;
    margin-top: 1rem;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    font-weight: 500;
    text-decoration: none;
    background: var(--ppt-bg-app);
    border: 1px solid var(--ppt-border-default);
    border-radius: 0.375rem;
    color: var(--ppt-fg-secondary);
    transition: all 0.15s;
  }

  .action-btn:hover {
    background: var(--ppt-border-default);
  }

  .action-btn.primary {
    background: var(--ppt-brand-500);
    border-color: var(--ppt-brand-500);
    color: var(--ppt-fg-on-accent);
  }

  .action-btn.primary:hover {
    background: var(--ppt-color-primary);
  }

  .preview-section {
    margin-bottom: 2rem;
    padding-bottom: 1.5rem;
    border-bottom: 1px solid var(--ppt-border-default);
  }

  .preview-section-title {
    margin-bottom: 0.75rem;
  }

  .document-pdf-preview {
    width: 100%;
  }

  .intelligence-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .section-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0 0 0.5rem;
    font-size: 1rem;
    font-weight: 600;
    color: var(--ppt-fg-primary);
  }

  .ai-icon {
    padding: 0.125rem 0.375rem;
    font-size: 0.625rem;
    font-weight: 700;
    background: #7c3aed;
    color: var(--ppt-fg-on-accent);
    border-radius: 0.25rem;
  }

  .intelligence-card {
    margin-bottom: 0.5rem;
  }

  .ocr-preview {
    margin-top: 1rem;
    padding: 1rem;
    background: var(--ppt-bg-app);
    border-radius: 0.5rem;
  }

  .preview-title {
    margin: 0 0 0.25rem;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--ppt-fg-primary);
  }

  .word-count {
    margin: 0 0 0.75rem;
    font-size: 0.75rem;
    color: var(--ppt-fg-muted);
  }

  .ocr-text-container {
    max-height: 200px;
    overflow-y: auto;
    padding: 0.75rem;
    background: var(--ppt-bg-surface);
    border: 1px solid var(--ppt-border-default);
    border-radius: 0.375rem;
  }

  .ocr-text {
    margin: 0;
    font-size: 0.75rem;
    font-family: inherit;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--ppt-fg-secondary);
  }
`;

export default DocumentDetail;
