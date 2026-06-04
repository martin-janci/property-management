/**
 * Document Detail Page (Epic 39).
 *
 * Shows full document details with intelligence features.
 */

import {
  type AccessScope,
  useDocument,
  useDocumentClassification,
  useDocumentVersions,
  useReprocessOcr,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ClassificationUI } from '../components/ClassificationBadge';
import { DocumentSharePanel } from '../components/DocumentSharePanel';
import { DocumentSummary } from '../components/DocumentSummary';
import { OcrProcessingStatus } from '../components/OcrStatusBadge';

/** Human-readable labels for the access_scope values returned by the backend (7a-3). */
const AUDIENCE_LABELS: Record<AccessScope, string> = {
  organization: 'All members',
  building: 'Building members',
  unit: 'Unit members',
  user: 'Specific users',
  public: 'Public',
};

interface DocumentDetailProps {
  documentId: string;
}

/**
 * Raw shape of a version row as returned by the list-versions endpoint.
 * The generated client returns camelCase (`version`, `createdAt`, `createdBy`,
 * `file.sizeBytes`), but we read snake_case aliases too for resilience.
 */
interface VersionRecord {
  id?: string;
  version?: number;
  version_number?: number;
  versionNumber?: number;
  uploader?: string;
  uploaderName?: string;
  createdBy?: string;
  created_by?: string;
  createdAt?: string;
  created_at?: string;
  sizeBytes?: number;
  size_bytes?: number;
  file?: { sizeBytes?: number; size_bytes?: number };
}

/** Normalised version row used by the timeline UI. */
interface VersionRow {
  id: string;
  versionNumber: number;
  uploader?: string;
  createdAt: string;
  sizeBytes?: number;
  isCurrent: boolean;
}

/** Extract a version number from a record regardless of field naming. */
function versionNumberOf(v: VersionRecord): number {
  return v.versionNumber ?? v.version_number ?? v.version ?? 0;
}

export function DocumentDetail({ documentId }: DocumentDetailProps) {
  const { data, isLoading, error, refetch } = useDocument(documentId);
  const classification = useDocumentClassification(documentId);
  const versions = useDocumentVersions(documentId);
  const reprocessOcr = useReprocessOcr();
  const [showSharePanel, setShowSharePanel] = useState(false);
  const { t } = useTranslation();

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
    // 403 = audience access denied — surface the permission-denied state
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
          <p className="permission-message">Tento dokument nie je prístupný pre váš účet.</p>
          <p className="permission-hint">
            Dokument mohol byť nastavený ako viditeľný iba pre správcov alebo konkrétnych
            používateľov.
          </p>

          <style>{detailStyles}</style>
        </div>
      );
    }

    return (
      <div className="document-detail error">
        <p className="error-message">Detail dokumentu sa nepodarilo načítať.</p>
        <button type="button" onClick={() => refetch()} className="retry-btn">
          Skúsiť znova
        </button>

        <style>{detailStyles}</style>
      </div>
    );
  }

  if (!data?.document) {
    return (
      <div className="document-detail empty">
        <p>Dokument nebol nájdený.</p>

        <style>{detailStyles}</style>
      </div>
    );
  }

  const doc = data.document;

  // Format file size
  const formatSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  // Format date
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

  // Estimate word count from OCR text
  const wordCount = doc.ocr_text ? doc.ocr_text.split(/\s+/).filter((w) => w.length > 0).length : 0;

  // Normalise version rows from the (generated, camelCase) list-versions
  // response into a small UI shape, tolerating both camelCase and snake_case
  // field names. Newest version first; the highest version number is current.
  const rawVersions = (versions.data ?? []) as unknown as VersionRecord[];
  const maxVersion = rawVersions.reduce((max, v) => {
    const n = versionNumberOf(v);
    return n > max ? n : max;
  }, 0);
  const versionRows: VersionRow[] = rawVersions
    .map((v) => {
      const versionNumber = versionNumberOf(v);
      return {
        id: v.id ?? `version-${versionNumber}`,
        versionNumber,
        uploader: v.uploaderName ?? v.uploader ?? v.createdBy ?? v.created_by,
        createdAt: v.createdAt ?? v.created_at ?? '',
        sizeBytes: v.file?.sizeBytes ?? v.file?.size_bytes ?? v.sizeBytes ?? v.size_bytes,
        isCurrent: versionNumber === maxVersion && maxVersion > 0,
      };
    })
    .sort((a, b) => b.versionNumber - a.versionNumber);

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
          {doc.access_scope && (
            <>
              <span className="meta-separator">|</span>
              <span
                className="meta-item audience-badge"
                title={`Visible to: ${AUDIENCE_LABELS[doc.access_scope] ?? doc.access_scope}`}
              >
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  aria-hidden="true"
                  style={{ marginRight: '4px', verticalAlign: 'middle' }}
                >
                  <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                  <circle cx="12" cy="12" r="3" />
                </svg>
                {AUDIENCE_LABELS[doc.access_scope] ?? doc.access_scope}
              </span>
            </>
          )}
        </div>

        {doc.description && <p className="document-description">{doc.description}</p>}

        <div className="document-dates">
          <p>Created: {formatDate(doc.created_at)}</p>
          <p>Updated: {formatDate(doc.updated_at)}</p>
        </div>

        {/* Document Actions */}
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
          <a
            href={`/api/v1/documents/${doc.id}/preview`}
            target="_blank"
            rel="noopener noreferrer"
            className="action-btn"
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
              <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
              <circle cx="12" cy="12" r="3" />
            </svg>
            Preview
          </a>
          <button
            type="button"
            className={`action-btn${showSharePanel ? ' active' : ''}`}
            onClick={() => setShowSharePanel((v) => !v)}
            aria-expanded={showSharePanel}
            aria-controls="doc-share-panel"
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
              <circle cx="18" cy="5" r="3" />
              <circle cx="6" cy="12" r="3" />
              <circle cx="18" cy="19" r="3" />
              <line x1="8.59" y1="13.51" x2="15.42" y2="17.49" />
              <line x1="15.41" y1="6.51" x2="8.59" y2="10.49" />
            </svg>
            Share
          </button>
        </div>

        {showSharePanel && (
          <div id="doc-share-panel" className="share-panel-wrapper">
            <DocumentSharePanel documentId={doc.id} documentTitle={doc.title} />
          </div>
        )}
      </div>

      {/* Intelligence Section */}
      <div className="intelligence-section">
        <h3 className="section-title">
          <span className="ai-icon">AI</span>
          Document Intelligence
        </h3>

        {/* Summary (Story 39.4) */}
        <div className="intelligence-card">
          <DocumentSummary
            documentId={doc.id}
            summary={doc.summary}
            summaryGeneratedAt={doc.summary_generated_at}
            wordCount={wordCount}
            onSummaryGenerated={() => refetch()}
          />
        </div>

        {/* OCR Status (Story 39.2) */}
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

        {/* Classification (Story 39.3) */}
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

        {/* OCR Text Preview */}
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

      {/* Version History (Story 7B.1) — list-only timeline */}
      <div className="version-history-section">
        <h3 className="section-title">{t('documents.versionHistory.title', 'Version history')}</h3>

        {versions.isLoading && (
          <p className="version-empty">
            {t('documents.versionHistory.loading', 'Loading versions…')}
          </p>
        )}

        {versions.error && (
          <p className="version-empty">
            {t('documents.versionHistory.loadError', 'Failed to load version history.')}
          </p>
        )}

        {!versions.isLoading && !versions.error && versionRows.length === 0 && (
          <p className="version-empty">
            {t('documents.versionHistory.empty', 'No versions are available for this document.')}
          </p>
        )}

        {versionRows.length > 0 && (
          <ol className="version-timeline">
            {versionRows.map((v) => (
              <li key={v.id} className={`version-row${v.isCurrent ? ' version-row--current' : ''}`}>
                <div className="version-marker" aria-hidden="true" />
                <div className="version-body">
                  <div className="version-head">
                    <span className="version-number">
                      {t('documents.versionHistory.version', {
                        defaultValue: 'Version {{n}}',
                        n: v.versionNumber,
                      })}
                    </span>
                    {v.isCurrent && (
                      <span className="version-current-badge">
                        {t('documents.versionHistory.current', 'Current')}
                      </span>
                    )}
                  </div>
                  <div className="version-meta">
                    {v.uploader && <span className="version-meta-item">{v.uploader}</span>}
                    {v.uploader && <span className="version-meta-sep">|</span>}
                    <span className="version-meta-item">{formatDate(v.createdAt)}</span>
                    {v.sizeBytes != null && (
                      <>
                        <span className="version-meta-sep">|</span>
                        <span className="version-meta-item">{formatSize(v.sizeBytes)}</span>
                      </>
                    )}
                  </div>
                </div>
              </li>
            ))}
          </ol>
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

  .meta-item.audience-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.125rem 0.5rem;
    background: var(--ppt-bg-app);
    border: 1px solid var(--ppt-border-default);
    border-radius: 9999px;
    font-size: 0.75rem;
    color: var(--ppt-fg-muted);
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

  .action-btn.active {
    background: var(--ppt-brand-500);
    border-color: var(--ppt-brand-500);
    color: var(--ppt-fg-on-accent);
  }

  .share-panel-wrapper {
    margin-top: 1rem;
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

  .version-history-section {
    margin-top: 2rem;
    padding-top: 1.5rem;
    border-top: 1px solid var(--ppt-border-default);
  }

  .version-empty {
    font-size: 0.8125rem;
    color: var(--ppt-fg-muted);
    margin: 0;
  }

  .version-timeline {
    list-style: none;
    margin: 0.75rem 0 0;
    padding: 0;
  }

  .version-row {
    position: relative;
    display: flex;
    gap: 0.75rem;
    padding: 0 0 1rem 0.25rem;
  }

  .version-row:not(:last-child)::before {
    content: '';
    position: absolute;
    left: 0.5rem;
    top: 1rem;
    bottom: 0;
    width: 1px;
    background: var(--ppt-border-default);
  }

  .version-marker {
    flex: 0 0 auto;
    width: 0.625rem;
    height: 0.625rem;
    margin-top: 0.25rem;
    border-radius: 9999px;
    background: var(--ppt-border-strong);
    z-index: 1;
  }

  .version-row--current .version-marker {
    background: var(--ppt-brand-500);
  }

  .version-body {
    flex: 1 1 auto;
  }

  .version-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .version-number {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--ppt-fg-primary);
  }

  .version-current-badge {
    padding: 0.0625rem 0.375rem;
    font-size: 0.6875rem;
    font-weight: 600;
    background: var(--ppt-brand-500);
    color: var(--ppt-fg-on-accent);
    border-radius: 9999px;
  }

  .version-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.375rem;
    margin-top: 0.125rem;
    font-size: 0.75rem;
    color: var(--ppt-fg-muted);
  }

  .version-meta-sep {
    color: var(--ppt-border-strong);
  }
`;

export default DocumentDetail;
