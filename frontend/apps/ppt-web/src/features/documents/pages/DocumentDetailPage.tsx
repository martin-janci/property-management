/**
 * Document Detail Page (Epic 39).
 *
 * Standalone page wrapper for document detail view.
 */

import { Link } from 'react-router-dom';
import { DocumentDetail } from './DocumentDetail';

interface DocumentDetailPageProps {
  documentId: string;
}

export function DocumentDetailPage({ documentId }: DocumentDetailPageProps) {
  return (
    <div className="document-detail-page">
      <div className="page-header">
        <Link to="/documents" className="back-link">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <path d="M19 12H5" />
            <polyline points="12 19 5 12 12 5" />
          </svg>
          Back to Documents
        </Link>
      </div>

      <div className="page-content">
        <DocumentDetail documentId={documentId} />
      </div>

      <style>{`
        .document-detail-page {
          min-height: 100%;
          padding: 1.5rem;
          background: var(--ppt-bg-app);
        }

        .page-header {
          margin-bottom: 1rem;
        }

        .back-link {
          display: inline-flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.5rem 1rem;
          font-size: 0.875rem;
          font-weight: 500;
          text-decoration: none;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.375rem;
          color: var(--ppt-fg-secondary);
          transition: all 0.15s;
        }

        .back-link:hover {
          background: var(--ppt-bg-app);
          border-color: var(--ppt-border-strong);
        }

        .page-content {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.75rem;
          overflow: hidden;
        }
      `}</style>
    </div>
  );
}

export default DocumentDetailPage;
