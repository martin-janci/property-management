/**
 * Documents Page (Epic 39, gap-7a-3).
 *
 * Main page for document management with intelligence features.
 * Browse tab uses the RLS-aware list endpoint; audience and status filters
 * are surfaced in the UI so the server's row-level security is visible to the
 * manager (not bypassed in the UI).
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { DocumentSearch } from '../components/DocumentSearch';
import { DocumentsBrowse } from '../components/DocumentsBrowse';
import { DocumentDetail } from './DocumentDetail';

interface DocumentsPageProps {
  organizationId: string;
  buildingId?: string;
}

export function DocumentsPage({ organizationId, buildingId }: DocumentsPageProps) {
  const { t } = useTranslation();
  const [selectedDocumentId, setSelectedDocumentId] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<'search' | 'browse'>('search');

  return (
    <div className="documents-page">
      <div className="page-header">
        <h1 className="page-title">Documents</h1>
        <div className="header-actions">
          <Link to="/documents/folders" className="folders-link">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
            </svg>
            Priečinky
          </Link>
          <Link to="/documents/upload" className="upload-link">
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
              <polyline points="17 8 12 3 7 8" />
              <line x1="12" y1="3" x2="12" y2="15" />
            </svg>
            Upload
          </Link>
          <Link to="/documents/templates" className="folders-link">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
            Šablóny
          </Link>
          <div className="view-toggle">
            <button
              type="button"
              onClick={() => setViewMode('search')}
              className={`toggle-btn ${viewMode === 'search' ? 'active' : ''}`}
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
                <path d="M21 21l-4.35-4.35" />
              </svg>
              Search
            </button>
            <button
              type="button"
              onClick={() => setViewMode('browse')}
              className={`toggle-btn ${viewMode === 'browse' ? 'active' : ''}`}
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
                <line x1="8" y1="6" x2="21" y2="6" />
                <line x1="8" y1="12" x2="21" y2="12" />
                <line x1="8" y1="18" x2="21" y2="18" />
                <line x1="3" y1="6" x2="3.01" y2="6" />
                <line x1="3" y1="12" x2="3.01" y2="12" />
                <line x1="3" y1="18" x2="3.01" y2="18" />
              </svg>
              Zoznam
            </button>
          </div>
        </div>
      </div>

      <div className="page-content">
        <div className={`documents-panel ${selectedDocumentId ? 'with-detail' : ''}`}>
          {viewMode === 'search' ? (
            <DocumentSearch
              organizationId={organizationId}
              buildingId={buildingId}
              onSelectDocument={setSelectedDocumentId}
            />
          ) : (
            <DocumentsBrowse
              organizationId={organizationId}
              buildingId={buildingId}
              onSelectDocument={setSelectedDocumentId}
            />
          )}
        </div>

        {selectedDocumentId && (
          <div className="detail-panel">
            <div className="detail-header">
              <button
                type="button"
                onClick={() => setSelectedDocumentId(null)}
                className="close-btn"
                aria-label={t('aria.closeDocumentDetail')}
              >
                ×
              </button>
            </div>
            <DocumentDetail documentId={selectedDocumentId} />
          </div>
        )}
      </div>

      <style>{`
        .documents-page {
          display: flex;
          flex-direction: column;
          height: 100%;
          padding: 1.5rem;
        }

        .page-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 1.5rem;
        }

        .page-title {
          margin: 0;
          font-size: 1.5rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .header-actions {
          display: flex;
          align-items: center;
          gap: 1rem;
        }

        .folders-link {
          display: inline-flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.5rem 1rem;
          font-size: 0.875rem;
          font-weight: 500;
          text-decoration: none;
          background: var(--ppt-bg-surface);
          color: var(--ppt-fg-muted);
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.375rem;
          transition: all 0.15s;
        }

        .folders-link:hover {
          color: var(--ppt-fg-primary);
          border-color: var(--ppt-fg-muted);
        }

        .upload-link {
          display: inline-flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.5rem 1rem;
          font-size: 0.875rem;
          font-weight: 600;
          text-decoration: none;
          background: var(--ppt-brand-500);
          color: var(--ppt-fg-on-accent);
          border-radius: 0.375rem;
          transition: background 0.15s;
        }

        .upload-link:hover {
          background: var(--ppt-color-primary);
        }

        .view-toggle {
          display: flex;
          background: var(--ppt-bg-app);
          border-radius: 0.5rem;
          padding: 0.25rem;
        }

        .toggle-btn {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.5rem 1rem;
          font-size: 0.875rem;
          font-weight: 500;
          background: transparent;
          border: none;
          border-radius: 0.375rem;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          transition: all 0.15s;
        }

        .toggle-btn:hover {
          color: var(--ppt-fg-primary);
        }

        .toggle-btn.active {
          background: var(--ppt-bg-surface);
          color: var(--ppt-fg-primary);
          box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
        }

        .page-content {
          display: flex;
          flex: 1;
          gap: 1.5rem;
          overflow: hidden;
        }

        .documents-panel {
          flex: 1;
          overflow-y: auto;
          padding-right: 0.5rem;
        }

        .documents-panel.with-detail {
          max-width: 50%;
        }

        .detail-panel {
          flex: 1;
          max-width: 50%;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.5rem;
          overflow-y: auto;
        }

        .detail-header {
          display: flex;
          justify-content: flex-end;
          padding: 0.5rem;
          border-bottom: 1px solid var(--ppt-border-default);
        }

        .close-btn {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 2rem;
          height: 2rem;
          font-size: 1.5rem;
          background: transparent;
          border: none;
          border-radius: 0.25rem;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          transition: all 0.15s;
        }

        .close-btn:hover {
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-primary);
        }

        .browse-placeholder {
          display: flex;
          align-items: center;
          justify-content: center;
          height: 200px;
          background: var(--ppt-bg-app);
          border-radius: 0.5rem;
          color: var(--ppt-fg-muted);
        }

        @media (max-width: 1024px) {
          .page-content {
            flex-direction: column;
          }

          .documents-panel,
          .documents-panel.with-detail {
            max-width: 100%;
          }

          .detail-panel {
            max-width: 100%;
          }
        }
      `}</style>
    </div>
  );
}

export default DocumentsPage;
