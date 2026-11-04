/**
 * Document Search Result Card (Story 39.1).
 *
 * Displays a search result with highlighted snippets.
 */

import type {
  AccessScope,
  SearchHighlight,
  DocumentSearchResult as SearchResult,
} from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

/** Maps backend access_scope values to i18n keys for audience labels (7a-3). */
const SCOPE_LABEL_KEYS: Record<AccessScope, string> = {
  organization: 'documents.search.audience.organization',
  building: 'documents.search.audience.building',
  unit: 'documents.search.audience.unit',
  user: 'documents.search.audience.user',
  public: 'documents.search.audience.public',
};

/** Shows a pill indicating who can see the document (RLS-resolved audience, 7a-3). */
function AudienceBadge({ scope }: { scope: AccessScope }) {
  const { t } = useTranslation();
  const label = SCOPE_LABEL_KEYS[scope] ? t(SCOPE_LABEL_KEYS[scope]) : scope;
  return (
    <span
      className="badge badge-audience"
      title={t('documents.detail.visibleTo', { audience: label })}
    >
      <svg
        width="10"
        height="10"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        aria-hidden="true"
        style={{ marginRight: '3px', verticalAlign: 'middle' }}
      >
        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
        <circle cx="12" cy="12" r="3" />
      </svg>
      {label}
    </span>
  );
}

import DOMPurify from 'dompurify';
import { ClassificationBadge } from './ClassificationBadge';
import { OcrStatusBadge } from './OcrStatusBadge';

interface DocumentSearchResultProps {
  result: SearchResult;
  onClick?: () => void;
}

export function DocumentSearchResult({ result, onClick }: DocumentSearchResultProps) {
  const { t, i18n } = useTranslation();
  const { document, score, highlights } = result;

  // Format file size
  const formatSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  // Format date
  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    return date.toLocaleDateString(i18n.language, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  };

  // Render highlighted snippet
  const renderHighlight = (highlight: SearchHighlight) => {
    // The snippet contains <mark> tags for highlighting
    // Sanitize with DOMPurify allowing only mark tags for defense-in-depth
    const sanitizedHtml = DOMPurify.sanitize(highlight.snippet, {
      ALLOWED_TAGS: ['mark'],
      ALLOWED_ATTR: [],
    });
    return (
      <div
        key={highlight.field}
        className="highlight-snippet"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: Content is sanitized with DOMPurify
        dangerouslySetInnerHTML={{ __html: sanitizedHtml }}
      />
    );
  };

  // Get field label
  const getFieldLabel = (field: SearchHighlight['field']): string => {
    switch (field) {
      case 'title':
        return t('documents.searchResult.field.title');
      case 'description':
        return t('documents.searchResult.field.description');
      case 'ocr_text':
        return t('documents.searchResult.field.ocrText');
      case 'summary':
        return t('documents.searchResult.field.summary');
      default:
        return field;
    }
  };

  return (
    <button type="button" className="search-result-card" onClick={onClick}>
      {/* Header */}
      <div className="result-header">
        <div className="result-title-row">
          <h3 className="result-title">{document.title}</h3>
          <span
            className="result-score"
            title={t('documents.searchResult.relevance', { score: score.toFixed(2) })}
          >
            {t('documents.searchResult.match', { percent: Math.round(score * 100) })}
          </span>
        </div>
        <div className="result-meta">
          <span className="meta-item">
            {t(`documents.category.${document.category}`, { defaultValue: document.category })}
          </span>
          <span className="meta-separator">|</span>
          <span className="meta-item">{document.file_name}</span>
          <span className="meta-separator">|</span>
          <span className="meta-item">{formatSize(document.size_bytes)}</span>
          <span className="meta-separator">|</span>
          <span className="meta-item">{formatDate(document.created_at)}</span>
        </div>
      </div>

      {/* Status Badges */}
      <div className="result-badges">
        {document.ocr_status && <OcrStatusBadge status={document.ocr_status} compact />}
        {document.predicted_category && document.classification_confidence && (
          <ClassificationBadge
            category={document.predicted_category}
            confidence={document.classification_confidence}
            compact
          />
        )}
        {document.summary && (
          <span className="badge badge-summary">{t('documents.searchResult.hasSummary')}</span>
        )}
        {document.access_scope && <AudienceBadge scope={document.access_scope} />}
      </div>

      {/* Highlighted Snippets */}
      {highlights.length > 0 && (
        <div className="result-highlights">
          {highlights.map((highlight) => (
            <div key={highlight.field} className="highlight-group">
              <span className="highlight-field">{getFieldLabel(highlight.field)}:</span>
              {renderHighlight(highlight)}
            </div>
          ))}
        </div>
      )}

      {/* Description fallback if no highlights */}
      {highlights.length === 0 && document.description && (
        <p className="result-description">{document.description}</p>
      )}

      <style>{`
        .search-result-card {
          display: block;
          width: 100%;
          text-align: left;
          font-family: inherit;
          padding: 1rem;
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.5rem;
          background: var(--ppt-bg-surface);
          cursor: pointer;
          transition: all 0.15s;
        }

        .search-result-card:hover {
          border-color: var(--ppt-brand-500);
          box-shadow: 0 2px 8px rgba(59, 130, 246, 0.1);
        }

        .search-result-card:focus-visible {
          outline: var(--ppt-focus-ring-width) solid var(--ppt-focus-ring-color);
          outline-offset: var(--ppt-focus-ring-offset);
          border-color: var(--ppt-brand-500);
          box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.2);
        }

        .result-header {
          margin-bottom: 0.5rem;
        }

        .result-title-row {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          gap: 1rem;
        }

        .result-title {
          margin: 0;
          font-size: 1rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .result-score {
          flex-shrink: 0;
          padding: 0.125rem 0.5rem;
          font-size: 0.75rem;
          font-weight: 500;
          background: var(--ppt-color-success-light);
          color: var(--ppt-color-success-hover);
          border-radius: 9999px;
        }

        .result-meta {
          display: flex;
          flex-wrap: wrap;
          align-items: center;
          gap: 0.25rem;
          margin-top: 0.25rem;
          font-size: 0.75rem;
          color: var(--ppt-fg-muted);
        }

        .meta-separator {
          color: var(--ppt-border-strong);
        }

        .result-badges {
          display: flex;
          flex-wrap: wrap;
          gap: 0.5rem;
          margin-bottom: 0.5rem;
        }

        .badge {
          display: inline-flex;
          align-items: center;
          padding: 0.125rem 0.5rem;
          font-size: 0.75rem;
          border-radius: 9999px;
        }

        .badge-summary {
          background: #ede9fe;
          color: #7c3aed;
        }

        .badge-audience {
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-muted);
          border: 1px solid var(--ppt-border-default);
          display: inline-flex;
          align-items: center;
        }

        .result-highlights {
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
        }

        .highlight-group {
          display: flex;
          flex-direction: column;
          gap: 0.25rem;
        }

        .highlight-field {
          font-size: 0.75rem;
          font-weight: 500;
          color: var(--ppt-fg-muted);
          text-transform: uppercase;
          letter-spacing: 0.025em;
        }

        .highlight-snippet {
          font-size: 0.875rem;
          color: var(--ppt-fg-secondary);
          line-height: 1.5;
        }

        .highlight-snippet mark {
          background: #fef08a;
          color: inherit;
          padding: 0 0.125rem;
          border-radius: 0.125rem;
        }

        .result-description {
          margin: 0;
          font-size: 0.875rem;
          color: var(--ppt-fg-muted);
          line-height: 1.5;
          display: -webkit-box;
          -webkit-line-clamp: 2;
          -webkit-box-orient: vertical;
          overflow: hidden;
        }
      `}</style>
    </button>
  );
}
