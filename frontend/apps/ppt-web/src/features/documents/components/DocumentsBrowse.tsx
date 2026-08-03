/**
 * Documents Browse Panel (gap-7a-3 / gap-7a-4).
 *
 * RLS-aware document listing with audience and status filtering.
 * The backend's list endpoint enforces access_scope through PostgreSQL RLS —
 * managers see all documents, non-managers only see documents accessible to
 * their role/unit. This component surfaces those permission boundaries in the
 * UI via the audience filter chips and status segmented control.
 *
 * gap-7a-2-folder-ui: Added "Move to folder" action button per document row,
 * wired to MoveFolderDialog + useMoveDocument hook.
 *
 * gap-7a-4: Added inline preview (eye) and download buttons to each document
 * row. Preview opens DocumentPreviewModal; download fetches a presigned URL
 * via useDocumentDownload and triggers a programmatic anchor click.
 */

import {
  type AccessScope,
  DOCUMENT_CATEGORIES,
  type DocumentListQuery,
  type DocumentStatus,
  type DocumentSummary,
  useDocuments,
} from '@ppt/api-client';
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useDocumentDownload } from '../hooks/useDocumentDownload';
import { useMoveDocumentWithToast } from '../hooks/useMoveDocumentWithToast';
import { DocumentPreviewModal } from './DocumentPreviewModal';
import { MoveFolderDialog } from './MoveFolderDialog';

// ─── helpers ─────────────────────────────────────────────────────────────────

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString('sk-SK', {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
  });
}

/** Map access_scope value to a human-readable audience label. */
function audienceLabel(scope: string | undefined): string {
  switch (scope) {
    case 'organization':
      return 'Všetci rezidenti';
    case 'building':
      return 'Bytový dom';
    case 'unit':
      return 'Iba jednotka';
    case 'user':
      return 'Konkrétni používatelia';
    case 'public':
      return 'Verejné';
    default:
      return 'Všetci rezidenti';
  }
}

/** Returns CSS class modifier for audience badge color-coding. */
function audienceMod(scope: string | undefined): string {
  switch (scope) {
    case 'organization':
      return 'audience--org';
    case 'building':
      return 'audience--building';
    case 'user':
      return 'audience--role';
    case 'public':
      return 'audience--public';
    default:
      return 'audience--org';
  }
}

// ─── sub-components ──────────────────────────────────────────────────────────

interface DocumentRowProps {
  doc: DocumentSummary;
  onSelect: (id: string) => void;
  onPreview: (doc: DocumentSummary) => void;
  onMoveRequest: (doc: DocumentSummary) => void;
  selected: boolean;
}

function RowDownloadButton({ doc }: { doc: DocumentSummary }) {
  const { download, isDownloading } = useDocumentDownload(doc.id, doc.file_name);

  return (
    <button
      type="button"
      className="doc-row__action-btn"
      onClick={(e) => {
        e.stopPropagation();
        download();
      }}
      disabled={isDownloading}
      aria-label={`Stiahnuť ${doc.file_name}`}
      title="Stiahnuť"
    >
      <svg
        width="14"
        height="14"
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
    </button>
  );
}

function DocumentRow({ doc, onSelect, onPreview, onMoveRequest, selected }: DocumentRowProps) {
  const ext = doc.file_name.split('.').pop()?.toUpperCase() ?? 'DOC';

  // Use div+role="button" to avoid invalid interactive-in-interactive nesting (HTML spec §4.10.6).
  return (
    <div
      role="button"
      tabIndex={0}
      aria-label={doc.title}
      className={`doc-row${selected ? ' doc-row--selected' : ''}`}
      onClick={() => onSelect(doc.id)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onSelect(doc.id);
        }
      }}
    >
      <span className="doc-row__icon" aria-hidden="true">
        {ext.slice(0, 3)}
      </span>
      <span className="doc-row__body">
        <span className="doc-row__title">{doc.title}</span>
        <span className="doc-row__meta">
          {doc.category} · {formatFileSize(doc.size_bytes)} · {formatDate(doc.updated_at)}
        </span>
      </span>
      <span className={`doc-row__audience ${audienceMod(doc.access_scope)}`}>
        {audienceLabel(doc.access_scope)}
      </span>
      {doc.status && doc.status !== 'published' && (
        <span className={`doc-row__status doc-row__status--${doc.status}`}>
          {doc.status === 'draft' ? 'Návrh' : 'Archivované'}
        </span>
      )}
      {/* Row action buttons (gap-7a-4: preview + download; gap-7a-2: move) */}
      <span className="doc-row__actions" onClick={(e) => e.stopPropagation()}>
        <button
          type="button"
          className="doc-row__action-btn"
          onClick={(e) => {
            e.stopPropagation();
            onPreview(doc);
          }}
          aria-label={`Náhľad ${doc.file_name}`}
          title="Náhľad"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
            <circle cx="12" cy="12" r="3" />
          </svg>
        </button>
        <RowDownloadButton doc={doc} />
        <button
          type="button"
          className="doc-row__action-btn"
          onClick={(e) => {
            e.stopPropagation();
            onMoveRequest(doc);
          }}
          aria-label={`Presunúť ${doc.file_name} do priečinka`}
          title="Presunúť do priečinka"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
            <line x1="12" y1="11" x2="12" y2="17" />
            <line x1="9" y1="14" x2="15" y2="14" />
          </svg>
        </button>
      </span>
    </div>
  );
}

// ─── main component ───────────────────────────────────────────────────────────

const STATUS_TABS: { label: string; value: DocumentStatus | undefined }[] = [
  { label: 'Všetky', value: undefined },
  { label: 'Publikované', value: 'published' },
  { label: 'Návrhy', value: 'draft' },
  { label: 'Archivované', value: 'archived' },
];

const AUDIENCE_OPTIONS: { label: string; value: AccessScope }[] = [
  { label: 'Všetci rezidenti', value: 'organization' },
  { label: 'Bytový dom', value: 'building' },
  { label: 'Iba jednotka', value: 'unit' },
  { label: 'Konkrétni používatelia', value: 'user' },
  { label: 'Verejné', value: 'public' },
];

export interface DocumentsBrowseProps {
  organizationId: string;
  buildingId?: string;
  onSelectDocument?: (documentId: string) => void;
}

export function DocumentsBrowse({
  organizationId: _organizationId,
  buildingId,
  onSelectDocument,
}: DocumentsBrowseProps) {
  const { t } = useTranslation();
  const [selectedStatus, setSelectedStatus] = useState<DocumentStatus | undefined>(undefined);
  const [selectedAudience, setSelectedAudience] = useState<AccessScope | undefined>(undefined);
  const [selectedCategory, setSelectedCategory] = useState<string | undefined>(undefined);
  const [search, setSearch] = useState('');
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  /** Document currently open in the inline preview modal (gap-7a-4). */
  const [previewDoc, setPreviewDoc] = useState<DocumentSummary | null>(null);
  // Move-to-folder state
  const [moveTarget, setMoveTarget] = useState<DocumentSummary | null>(null);

  // useMoveDocumentWithToast wraps useMoveDocument; query invalidation for
  // documentKeys.lists() and documentKeys.folders() happens inside useMoveDocument.onSuccess.
  const { executeMoveWithToast, isPending: isMovePending } = useMoveDocumentWithToast();

  const PAGE_SIZE = 20;

  const query = useMemo<DocumentListQuery>(
    () => ({
      search: search.length >= 2 ? search : undefined,
      category: selectedCategory,
      status: selectedStatus,
      access_scope: selectedAudience,
      limit: PAGE_SIZE,
      offset: page * PAGE_SIZE,
    }),
    [search, selectedCategory, selectedStatus, selectedAudience, page]
  );

  const { data, isLoading, error, refetch } = useDocuments(query);

  const handleSelect = useCallback(
    (id: string) => {
      setSelectedId(id);
      onSelectDocument?.(id);
    },
    [onSelectDocument]
  );

  const handlePreview = useCallback((doc: DocumentSummary) => {
    setPreviewDoc(doc);
  }, []);

  const handleClosePreview = useCallback(() => {
    setPreviewDoc(null);
  }, []);

  const handleMoveRequest = useCallback((doc: DocumentSummary) => {
    setMoveTarget(doc);
  }, []);

  const handleMoveConfirm = useCallback(
    async (folderId: string | null) => {
      if (!moveTarget) return;
      const success = await executeMoveWithToast(moveTarget, folderId);
      if (success) setMoveTarget(null);
    },
    [moveTarget, executeMoveWithToast]
  );

  const handleClearFilters = useCallback(() => {
    setSelectedStatus(undefined);
    setSelectedAudience(undefined);
    setSelectedCategory(undefined);
    setSearch('');
    setPage(0);
  }, []);

  const hasActiveFilters =
    selectedStatus || selectedAudience || selectedCategory || search.length >= 2;

  return (
    <div className="docs-browse">
      {/* Status tabs */}
      <div className="docs-browse__tabs" role="tablist" aria-label={t('aria.documentsStatus')}>
        {STATUS_TABS.map((tab) => (
          <button
            key={tab.value ?? 'all'}
            type="button"
            role="tab"
            aria-selected={selectedStatus === tab.value}
            className={`docs-browse__tab${selectedStatus === tab.value ? ' docs-browse__tab--active' : ''}`}
            onClick={() => {
              setSelectedStatus(tab.value);
              setPage(0);
            }}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Filter bar */}
      <div className="docs-browse__filters">
        <div className="docs-browse__search-wrap">
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
          <input
            type="search"
            value={search}
            onChange={(e: { target: { value: string } }) => {
              setSearch(e.target.value);
              setPage(0);
            }}
            placeholder="Hľadať podľa názvu…"
            className="docs-browse__search"
            aria-label={t('aria.searchDocuments')}
          />
        </div>

        {/* Audience filter */}
        <fieldset className="docs-browse__fieldset">
          <legend className="docs-browse__legend">Publikum</legend>
          <div className="docs-browse__chips">
            {AUDIENCE_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                type="button"
                aria-pressed={selectedAudience === opt.value}
                className={`docs-browse__chip${selectedAudience === opt.value ? ' docs-browse__chip--active' : ''}`}
                onClick={() => {
                  setSelectedAudience(selectedAudience === opt.value ? undefined : opt.value);
                  setPage(0);
                }}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </fieldset>

        {/* Category filter */}
        <fieldset className="docs-browse__fieldset">
          <legend className="docs-browse__legend">Kategória</legend>
          <div className="docs-browse__chips">
            {DOCUMENT_CATEGORIES.map((cat: string) => (
              <button
                key={cat}
                type="button"
                aria-pressed={selectedCategory === cat}
                className={`docs-browse__chip${selectedCategory === cat ? ' docs-browse__chip--active' : ''}`}
                onClick={() => {
                  setSelectedCategory(selectedCategory === cat ? undefined : cat);
                  setPage(0);
                }}
              >
                {cat}
              </button>
            ))}
          </div>
        </fieldset>

        {hasActiveFilters && (
          <button type="button" className="docs-browse__clear" onClick={handleClearFilters}>
            Vymazať filtre
          </button>
        )}
      </div>

      {/* Results */}
      <div className="docs-browse__results">
        {isLoading && (
          <ul
            className="docs-browse__skeleton"
            aria-label={t('aria.loadingDocuments')}
            aria-busy="true"
          >
            {Array.from({ length: 6 }).map((_, i) => (
              <li key={i} className="docs-browse__skel-row" />
            ))}
          </ul>
        )}

        {error && !isLoading && (
          <div className="docs-browse__error" role="alert">
            <p>Dokumenty sa nepodarilo načítať.</p>
            <button type="button" className="docs-browse__retry" onClick={() => refetch()}>
              Skúsiť znova
            </button>
          </div>
        )}

        {!isLoading && !error && data && data.documents.length === 0 && (
          <div className="docs-browse__empty">
            <p>Žiadne dokumenty nezodpovedajú zvoleným filtrom.</p>
            {hasActiveFilters && (
              <button type="button" className="docs-browse__clear" onClick={handleClearFilters}>
                Vymazať filtre
              </button>
            )}
          </div>
        )}

        {!isLoading && !error && data && data.documents.length > 0 && (
          <>
            <div className="docs-browse__count" aria-live="polite">
              {data.total} dokument{data.total !== 1 ? 'ov' : ''} · zobrazených{' '}
              {data.documents.length}
            </div>
            <ul className="docs-browse__list">
              {data.documents.map((doc: DocumentSummary) => (
                <li key={doc.id}>
                  <DocumentRow
                    doc={doc}
                    selected={selectedId === doc.id}
                    onSelect={handleSelect}
                    onPreview={handlePreview}
                    onMoveRequest={handleMoveRequest}
                  />
                </li>
              ))}
            </ul>

            {/* Pagination */}
            {data.total > PAGE_SIZE && (
              <div className="docs-browse__pagination">
                <button
                  type="button"
                  disabled={page === 0}
                  className="docs-browse__page-btn"
                  onClick={() => setPage((p: number) => Math.max(0, p - 1))}
                  aria-label={t('aria.previousPage')}
                >
                  ‹ Predch.
                </button>
                <span className="docs-browse__page-info">
                  Strana {page + 1} / {Math.ceil(data.total / PAGE_SIZE)}
                </span>
                <button
                  type="button"
                  disabled={(page + 1) * PAGE_SIZE >= data.total}
                  className="docs-browse__page-btn"
                  onClick={() => setPage((p: number) => p + 1)}
                  aria-label={t('aria.nextPage')}
                >
                  Ďalšia ›
                </button>
              </div>
            )}
          </>
        )}
      </div>

      {/* Inline preview modal (gap-7a-4) */}
      {previewDoc && (
        <DocumentPreviewModal
          documentId={previewDoc.id}
          title={previewDoc.title}
          fileName={previewDoc.file_name}
          mimeType={previewDoc.mime_type}
          onClose={handleClosePreview}
        />
      )}

      {/* Move-to-folder dialog */}
      {moveTarget && (
        <MoveFolderDialog
          documentTitles={[moveTarget.title]}
          buildingId={buildingId}
          currentFolderId={moveTarget.folder_id ?? null}
          onConfirm={handleMoveConfirm}
          onCancel={() => setMoveTarget(null)}
          isPending={isMovePending}
        />
      )}

      <style>{`
        .docs-browse {
          display: flex;
          flex-direction: column;
          gap: 1rem;
        }

        /* Status tabs */
        .docs-browse__tabs {
          display: flex;
          gap: 0.25rem;
          border-bottom: 1px solid var(--ppt-border-default);
          padding-bottom: 0.5rem;
        }

        .docs-browse__tab {
          padding: 0.375rem 0.875rem;
          font-size: 0.875rem;
          font-weight: 500;
          background: transparent;
          border: 1px solid transparent;
          border-radius: 0.375rem;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          transition: all 0.15s;
        }

        .docs-browse__tab:hover {
          color: var(--ppt-fg-primary);
          background: var(--ppt-bg-app);
        }

        .docs-browse__tab--active {
          color: var(--ppt-brand-600, var(--ppt-brand-500));
          background: var(--ppt-bg-surface);
          border-color: var(--ppt-border-default);
        }

        /* Filter bar */
        .docs-browse__filters {
          display: flex;
          flex-direction: column;
          gap: 0.75rem;
          padding: 0.75rem;
          background: var(--ppt-bg-app);
          border-radius: 0.5rem;
        }

        .docs-browse__search-wrap {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.5rem 0.75rem;
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.375rem;
          background: var(--ppt-bg-surface);
          color: var(--ppt-fg-subtle);
        }

        .docs-browse__search-wrap:focus-within {
          border-color: var(--ppt-brand-500);
          box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
        }

        .docs-browse__search {
          flex: 1;
          border: none;
          background: transparent;
          font-size: 0.875rem;
          outline: none;
          color: var(--ppt-fg-primary);
        }

        .docs-browse__fieldset {
          border: none;
          padding: 0;
          margin: 0;
          display: flex;
          flex-direction: column;
          gap: 0.375rem;
        }

        .docs-browse__legend {
          font-size: 0.75rem;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          color: var(--ppt-fg-subtle);
          padding: 0;
          margin-bottom: 0.25rem;
        }

        .docs-browse__chips {
          display: flex;
          flex-wrap: wrap;
          gap: 0.25rem;
        }

        .docs-browse__chip {
          padding: 0.25rem 0.75rem;
          font-size: 0.75rem;
          border: 1px solid var(--ppt-border-default);
          border-radius: 9999px;
          background: var(--ppt-bg-surface);
          color: var(--ppt-fg-muted);
          cursor: pointer;
          transition: all 0.15s;
        }

        .docs-browse__chip:hover {
          border-color: var(--ppt-brand-500);
          color: var(--ppt-brand-500);
        }

        .docs-browse__chip--active {
          background: var(--ppt-brand-500);
          border-color: var(--ppt-brand-500);
          color: #fff;
        }

        .docs-browse__clear {
          align-self: flex-start;
          padding: 0.25rem 0.75rem;
          font-size: 0.75rem;
          background: transparent;
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.375rem;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          transition: all 0.15s;
        }

        .docs-browse__clear:hover {
          border-color: var(--ppt-color-danger, #ef4444);
          color: var(--ppt-color-danger, #ef4444);
        }

        /* Results area */
        .docs-browse__results {
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
        }

        .docs-browse__count {
          font-size: 0.8125rem;
          color: var(--ppt-fg-muted);
          padding: 0.25rem 0;
        }

        .docs-browse__list {
          list-style: none;
          padding: 0;
          margin: 0;
          display: flex;
          flex-direction: column;
          gap: 0.25rem;
        }

        /* Document row */
        .doc-row {
          display: flex;
          align-items: center;
          gap: 0.75rem;
          width: 100%;
          padding: 0.625rem 0.75rem;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.5rem;
          text-align: left;
          cursor: pointer;
          transition: all 0.15s;
        }

        .doc-row:hover {
          border-color: var(--ppt-brand-500);
          background: var(--ppt-bg-app);
        }

        .doc-row--selected {
          border-color: var(--ppt-brand-500);
          border-left-width: 3px;
          background: color-mix(in srgb, var(--ppt-brand-500) 6%, var(--ppt-bg-surface));
        }

        .doc-row__icon {
          flex-shrink: 0;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 2.25rem;
          height: 2.25rem;
          font-size: 0.625rem;
          font-weight: 700;
          border-radius: 0.375rem;
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-muted);
          font-family: monospace;
          border: 1px solid var(--ppt-border-default);
        }

        .doc-row__body {
          flex: 1;
          min-width: 0;
          display: flex;
          flex-direction: column;
          gap: 0.125rem;
        }

        .doc-row__title {
          font-size: 0.875rem;
          font-weight: 500;
          color: var(--ppt-fg-primary);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }

        .doc-row__meta {
          font-size: 0.75rem;
          color: var(--ppt-fg-muted);
        }

        /* Audience badge */
        .doc-row__audience {
          flex-shrink: 0;
          padding: 0.125rem 0.5rem;
          font-size: 0.6875rem;
          font-weight: 500;
          border-radius: 9999px;
          border: 1px solid transparent;
        }

        .audience--org {
          background: color-mix(in srgb, var(--ppt-brand-500) 12%, transparent);
          color: var(--ppt-brand-600, var(--ppt-brand-500));
          border-color: color-mix(in srgb, var(--ppt-brand-500) 25%, transparent);
        }

        .audience--building {
          background: color-mix(in srgb, #6366f1 12%, transparent);
          color: #4f46e5;
          border-color: color-mix(in srgb, #6366f1 25%, transparent);
        }

        .audience--role {
          background: color-mix(in srgb, #f59e0b 12%, transparent);
          color: #b45309;
          border-color: color-mix(in srgb, #f59e0b 25%, transparent);
        }

        .audience--public {
          background: color-mix(in srgb, #10b981 12%, transparent);
          color: #047857;
          border-color: color-mix(in srgb, #10b981 25%, transparent);
        }

        /* Status pill */
        .doc-row__status {
          flex-shrink: 0;
          padding: 0.125rem 0.5rem;
          font-size: 0.6875rem;
          font-weight: 600;
          border-radius: 9999px;
        }

        .doc-row__status--draft {
          background: color-mix(in srgb, #f59e0b 15%, transparent);
          color: #b45309;
        }

        .doc-row__status--archived {
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-subtle);
        }

        /* Action buttons */
        .doc-row__actions {
          display: flex;
          align-items: center;
          gap: 0.125rem;
          flex-shrink: 0;
          opacity: 0;
          transition: opacity 0.12s;
        }

        .doc-row:hover .doc-row__actions,
        .doc-row--selected .doc-row__actions,
        .doc-row:focus-within .doc-row__actions {
          opacity: 1;
        }

        .doc-row__action-btn {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 1.625rem;
          height: 1.625rem;
          border: none;
          border-radius: 0.25rem;
          background: transparent;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          transition: all 0.1s;
        }

        .doc-row__action-btn:hover {
          background: var(--ppt-border-default);
          color: var(--ppt-brand-500);
        }

        .doc-row__action-btn:focus-visible {
          outline: 2px solid var(--ppt-brand-500);
          outline-offset: 2px;
        }

        /* Loading skeleton */
        .docs-browse__skeleton {
          list-style: none;
          padding: 0;
          margin: 0;
          display: flex;
          flex-direction: column;
          gap: 0.25rem;
        }

        .docs-browse__skel-row {
          height: 3.5rem;
          border-radius: 0.5rem;
          background: linear-gradient(
            90deg,
            var(--ppt-bg-app) 25%,
            var(--ppt-border-default) 50%,
            var(--ppt-bg-app) 75%
          );
          background-size: 200% 100%;
          animation: shimmer 1.4s ease-in-out infinite;
        }

        @keyframes shimmer {
          0% { background-position: 200% 0; }
          100% { background-position: -200% 0; }
        }

        /* Error state */
        .docs-browse__error {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 0.75rem;
          padding: 2rem;
          background: color-mix(in srgb, #ef4444 8%, var(--ppt-bg-surface));
          border: 1px solid color-mix(in srgb, #ef4444 20%, transparent);
          border-radius: 0.5rem;
          color: #b91c1c;
          text-align: center;
        }

        .docs-browse__retry {
          padding: 0.5rem 1.25rem;
          background: var(--ppt-brand-500);
          color: #fff;
          border: none;
          border-radius: 0.375rem;
          font-size: 0.875rem;
          font-weight: 600;
          cursor: pointer;
        }

        /* Empty state */
        .docs-browse__empty {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 0.75rem;
          padding: 3rem 1rem;
          color: var(--ppt-fg-muted);
          text-align: center;
        }

        /* Pagination */
        .docs-browse__pagination {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 1rem;
          padding: 0.75rem 0;
          font-size: 0.875rem;
          color: var(--ppt-fg-muted);
        }

        .docs-browse__page-btn {
          padding: 0.375rem 0.875rem;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.375rem;
          font-size: 0.875rem;
          cursor: pointer;
          transition: all 0.15s;
        }

        .docs-browse__page-btn:disabled {
          opacity: 0.4;
          cursor: not-allowed;
        }

        .docs-browse__page-btn:not(:disabled):hover {
          border-color: var(--ppt-brand-500);
          color: var(--ppt-brand-500);
        }

        .docs-browse__page-info {
          font-variant-numeric: tabular-nums;
        }
      `}</style>
    </div>
  );
}

export default DocumentsBrowse;
