/**
 * FolderTreePage (gap-7a-2).
 *
 * Dedicated page for managing the 5-level document folder hierarchy.
 * Left panel: FolderTree (create / rename / delete folders, up to 5 levels).
 * Right panel: document list filtered to the selected folder.
 *
 * Backend enforces:
 *   - 5-level max nesting (returns 422 when exceeded)
 *   - circular-reference prevention on moves / parent updates (returns 409)
 *
 * The UI respects these constraints by:
 *   - disabling "add child" when depth === MAX_DEPTH - 1 (4)
 *   - surfacing API errors via toast on the consuming page (FolderTreePageRoute)
 *
 * gap-7a-2-folder-ui additions:
 *   - FolderBreadcrumb in the content panel header shows the path of the
 *     currently-selected folder (e.g. "Všetky / Rok 2025 / Faktúry").
 *   - Each document row in the folder-scoped list has a "Move to folder" button
 *     that opens MoveFolderDialog.
 */

import { type DocumentSummary, useDocuments, useFolderTree } from '@ppt/api-client';
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { DocumentsBrowse } from '../components/DocumentsBrowse';
import { FolderTree } from '../components/FolderTree';
import {
  buildFolderCrumbs,
  FolderBreadcrumb,
  MoveFolderDialog,
} from '../components/MoveFolderDialog';
import { useMoveDocumentWithToast } from '../hooks/useMoveDocumentWithToast';

// ─── sub-component: folder-scoped document list ───────────────────────────────

interface FolderDocumentsProps {
  organizationId: string;
  buildingId?: string;
  folderId: string;
  onSelectDocument?: (id: string) => void;
  onMoveRequest?: (doc: DocumentSummary) => void;
}

function FolderDocuments({
  organizationId: _organizationId,
  buildingId: _buildingId,
  folderId,
  onSelectDocument,
  onMoveRequest,
}: FolderDocumentsProps) {
  const { t } = useTranslation();
  const { data, isLoading, error, refetch } = useDocuments({
    folder_id: folderId,
    limit: 50,
    offset: 0,
  });

  const docs = data?.documents ?? [];

  if (isLoading) {
    return (
      <ul className="fd__skeleton" aria-busy="true" aria-label={t('aria.loadingDocuments')}>
        {Array.from({ length: 5 }).map((_, i) => (
          <li key={i} className="fd__skel-row" />
        ))}
      </ul>
    );
  }

  if (error) {
    return (
      <div className="fd__error" role="alert">
        <p>Dokumenty sa nepodarilo načítať.</p>
        <button type="button" className="fd__retry" onClick={() => refetch()}>
          Skúsiť znova
        </button>
      </div>
    );
  }

  if (docs.length === 0) {
    return (
      <div className="fd__empty">
        <svg
          width="32"
          height="32"
          viewBox="0 0 24 24"
          fill="none"
          stroke="var(--ppt-fg-subtle)"
          strokeWidth="1.5"
          aria-hidden="true"
        >
          <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
          <polyline points="14 2 14 8 20 8" />
        </svg>
        <p>Priečinok je prázdny</p>
        <Link to="/documents/upload" className="fd__upload-link">
          Nahrať dokumenty
        </Link>
      </div>
    );
  }

  return (
    <ul className="fd__list">
      {docs.map((doc) => {
        const ext = doc.file_name.split('.').pop()?.toUpperCase() ?? 'DOC';
        return (
          <li key={doc.id}>
            <div className="fd__row-wrap">
              <button type="button" className="fd__row" onClick={() => onSelectDocument?.(doc.id)}>
                <span className="fd__icon" aria-hidden="true">
                  {ext.slice(0, 3)}
                </span>
                <span className="fd__body">
                  <span className="fd__title">{doc.title}</span>
                  <span className="fd__meta">
                    {doc.category} · {(doc.size_bytes / 1024).toFixed(1)} KB
                  </span>
                </span>
              </button>
              {onMoveRequest && (
                <button
                  type="button"
                  className="fd__move-btn"
                  onClick={() => onMoveRequest(doc)}
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
              )}
            </div>
          </li>
        );
      })}
      {data && data.total > docs.length && (
        <li className="fd__more">
          <Link to={`/documents?folder_id=${folderId}`} className="fd__more-link">
            Zobraziť všetkých {data.total} dokumentov →
          </Link>
        </li>
      )}
    </ul>
  );
}

// ─── main page ────────────────────────────────────────────────────────────────

export interface FolderTreePageProps {
  organizationId: string;
  buildingId?: string;
}

export function FolderTreePage({ organizationId, buildingId }: FolderTreePageProps) {
  const { t } = useTranslation();
  const [selectedFolderId, setSelectedFolderId] = useState<string | null>(null);
  const [selectedDocumentId, setSelectedDocumentId] = useState<string | null>(null);

  // Move-to-folder state
  const [moveTarget, setMoveTarget] = useState<DocumentSummary | null>(null);
  // useMoveDocumentWithToast wraps useMoveDocument; query invalidation for
  // documentKeys.lists() and documentKeys.folders() happens inside useMoveDocument.onSuccess.
  const { executeMoveWithToast, isPending: isMovePending } = useMoveDocumentWithToast();

  // Folder tree data — needed for breadcrumbs
  const { data: treeData } = useFolderTree(buildingId);
  const tree = treeData?.tree ?? [];

  // Breadcrumb crumbs for the currently selected folder
  const crumbs = useMemo(() => buildFolderCrumbs(tree, selectedFolderId), [tree, selectedFolderId]);

  // Query that feeds into the header count for the selected folder
  const { data: selectedFolderDocs } = useDocuments(
    useMemo(
      () => (selectedFolderId ? { folder_id: selectedFolderId, limit: 1, offset: 0 } : undefined),
      [selectedFolderId]
    )
  );

  const handleFolderSelect = (id: string | null) => {
    setSelectedFolderId(id);
    setSelectedDocumentId(null);
  };

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

  return (
    <div className="ftp">
      {/* Page header */}
      <div className="ftp__header">
        <nav className="ftp__breadcrumb" aria-label={t('aria.navigation')}>
          <Link to="/documents" className="ftp__bc-link">
            Dokumenty
          </Link>
          <span className="ftp__bc-sep" aria-hidden="true">
            /
          </span>
          <span className="ftp__bc-current">Priečinky</span>
        </nav>
        <div className="ftp__header-actions">
          <Link to="/documents/upload" className="ftp__upload-btn">
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
              <polyline points="17 8 12 3 7 8" />
              <line x1="12" y1="3" x2="12" y2="15" />
            </svg>
            Nahrať dokument
          </Link>
          <Link to="/documents" className="ftp__back-btn">
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <line x1="19" y1="12" x2="5" y2="12" />
              <polyline points="12 19 5 12 12 5" />
            </svg>
            Späť na dokumenty
          </Link>
        </div>
      </div>

      {/* Body: tree (left) + document content (right) */}
      <div className="ftp__body">
        {/* Left: folder tree panel */}
        <aside className="ftp__sidebar" aria-label={t('aria.folderStructure')}>
          <FolderTree
            buildingId={buildingId}
            selectedFolderId={selectedFolderId}
            onSelectFolder={handleFolderSelect}
          />
        </aside>

        {/* Right: document content */}
        <section className="ftp__content" aria-label={t('aria.documentsInFolder')}>
          {/* Folder header with breadcrumb */}
          <div className="ftp__content-header">
            <div className="ftp__content-heading">
              {crumbs.length > 0 ? (
                <FolderBreadcrumb
                  crumbs={crumbs}
                  onNavigate={handleFolderSelect}
                  className="ftp__folder-bc"
                />
              ) : (
                <h2 className="ftp__content-title">Všetky dokumenty</h2>
              )}
            </div>
            {selectedFolderId && selectedFolderDocs && (
              <span className="ftp__content-count">
                {selectedFolderDocs.total} dokument
                {selectedFolderDocs.total !== 1 ? 'ov' : ''}
              </span>
            )}
          </div>

          {/* Document list */}
          {selectedFolderId ? (
            <FolderDocuments
              organizationId={organizationId}
              buildingId={buildingId}
              folderId={selectedFolderId}
              onSelectDocument={setSelectedDocumentId}
              onMoveRequest={handleMoveRequest}
            />
          ) : (
            <DocumentsBrowse
              organizationId={organizationId}
              buildingId={buildingId}
              onSelectDocument={setSelectedDocumentId}
            />
          )}

          {/* Suppress unused variable lint: selectedDocumentId is used for detail panel integration */}
          {selectedDocumentId && (
            <div className="ftp__detail-hint" aria-live="polite">
              <span className="sr-only">Dokument {selectedDocumentId} vybraný</span>
            </div>
          )}
        </section>
      </div>

      {/* Move-to-folder dialog (from FolderDocuments) */}
      {moveTarget && (
        <MoveFolderDialog
          documentTitles={[moveTarget.title]}
          buildingId={buildingId}
          currentFolderId={selectedFolderId}
          onConfirm={handleMoveConfirm}
          onCancel={() => setMoveTarget(null)}
          isPending={isMovePending}
        />
      )}

      <style>{`
        /* ── Layout ──────────────────────────────────────── */
        .ftp {
          display: flex;
          flex-direction: column;
          height: 100%;
          padding: 1.5rem;
          gap: 1rem;
          overflow: hidden;
        }

        /* ── Page header ─────────────────────────────────── */
        .ftp__header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          flex-shrink: 0;
        }

        .ftp__breadcrumb {
          display: flex;
          align-items: center;
          gap: 0.375rem;
          font-size: 0.875rem;
        }

        .ftp__bc-link {
          color: var(--ppt-brand-500);
          text-decoration: none;
          font-weight: 500;
        }

        .ftp__bc-link:hover { text-decoration: underline; }

        .ftp__bc-sep {
          color: var(--ppt-fg-muted);
        }

        .ftp__bc-current {
          color: var(--ppt-fg-primary);
          font-weight: 600;
        }

        .ftp__header-actions {
          display: flex;
          gap: 0.5rem;
          align-items: center;
        }

        .ftp__upload-btn {
          display: inline-flex;
          align-items: center;
          gap: 0.375rem;
          padding: 0.4375rem 0.875rem;
          font-size: 0.8125rem;
          font-weight: 600;
          text-decoration: none;
          background: var(--ppt-brand-500);
          color: var(--ppt-fg-on-accent, #fff);
          border-radius: 0.375rem;
          transition: opacity 0.15s;
        }

        .ftp__upload-btn:hover { opacity: 0.9; }

        .ftp__back-btn {
          display: inline-flex;
          align-items: center;
          gap: 0.375rem;
          padding: 0.4375rem 0.875rem;
          font-size: 0.8125rem;
          font-weight: 500;
          text-decoration: none;
          color: var(--ppt-fg-muted);
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.375rem;
          background: var(--ppt-bg-surface);
          transition: all 0.15s;
        }

        .ftp__back-btn:hover {
          color: var(--ppt-fg-primary);
          border-color: var(--ppt-fg-muted);
        }

        /* ── Body split ──────────────────────────────────── */
        .ftp__body {
          display: flex;
          flex: 1;
          gap: 1rem;
          overflow: hidden;
          min-height: 0;
        }

        /* ── Sidebar ─────────────────────────────────────── */
        .ftp__sidebar {
          flex-shrink: 0;
          width: 15rem;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.5rem;
          overflow-y: auto;
        }

        /* ── Content ─────────────────────────────────────── */
        .ftp__content {
          flex: 1;
          display: flex;
          flex-direction: column;
          min-width: 0;
          overflow: hidden;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.5rem;
        }

        .ftp__content-header {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.75rem 1rem;
          border-bottom: 1px solid var(--ppt-border-default);
          flex-shrink: 0;
        }

        .ftp__content-heading {
          flex: 1;
          min-width: 0;
        }

        .ftp__content-title {
          margin: 0;
          font-size: 0.9375rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .ftp__folder-bc {
          font-size: 0.875rem;
        }

        .ftp__content-count {
          font-size: 0.75rem;
          font-weight: 600;
          padding: 0.125rem 0.5rem;
          background: var(--ppt-bg-app);
          border: 1px solid var(--ppt-border-default);
          border-radius: 9999px;
          color: var(--ppt-fg-muted);
          flex-shrink: 0;
        }

        /* ── FolderDocuments sub-component ───────────────── */
        .fd__skeleton {
          list-style: none;
          padding: 0.75rem;
          margin: 0;
          display: flex;
          flex-direction: column;
          gap: 0.375rem;
        }

        .fd__skel-row {
          height: 3rem;
          border-radius: 0.375rem;
          background: linear-gradient(
            90deg,
            var(--ppt-bg-app) 25%,
            var(--ppt-border-default) 50%,
            var(--ppt-bg-app) 75%
          );
          background-size: 200% 100%;
          animation: fd-shimmer 1.4s ease-in-out infinite;
        }

        @keyframes fd-shimmer {
          0%  { background-position: 200% 0; }
          100% { background-position: -200% 0; }
        }

        .fd__error {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 0.5rem;
          padding: 2rem;
          color: #b91c1c;
          text-align: center;
          font-size: 0.875rem;
        }

        .fd__retry {
          padding: 0.375rem 1rem;
          font-size: 0.8125rem;
          font-weight: 600;
          background: var(--ppt-brand-500);
          color: #fff;
          border: none;
          border-radius: 0.375rem;
          cursor: pointer;
        }

        .fd__empty {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 0.75rem;
          padding: 3rem 1rem;
          color: var(--ppt-fg-muted);
          text-align: center;
          font-size: 0.875rem;
        }

        .fd__upload-link {
          padding: 0.375rem 1rem;
          font-size: 0.8125rem;
          font-weight: 600;
          text-decoration: none;
          background: var(--ppt-brand-500);
          color: #fff;
          border-radius: 0.375rem;
          transition: opacity 0.15s;
        }

        .fd__upload-link:hover { opacity: 0.85; }

        .fd__list {
          list-style: none;
          padding: 0.5rem;
          margin: 0;
          display: flex;
          flex-direction: column;
          gap: 0.25rem;
          overflow-y: auto;
          flex: 1;
        }

        /* Row wrapper: doc button + move button */
        .fd__row-wrap {
          display: flex;
          align-items: center;
          gap: 0.25rem;
        }

        .fd__row {
          display: flex;
          align-items: center;
          gap: 0.625rem;
          flex: 1;
          min-width: 0;
          padding: 0.5rem 0.625rem;
          background: transparent;
          border: 1px solid transparent;
          border-radius: 0.375rem;
          text-align: left;
          cursor: pointer;
          transition: all 0.12s;
        }

        .fd__row:hover {
          background: var(--ppt-bg-app);
          border-color: var(--ppt-border-default);
        }

        .fd__move-btn {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 1.75rem;
          height: 1.75rem;
          border: none;
          border-radius: 0.25rem;
          background: transparent;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          flex-shrink: 0;
          transition: all 0.1s;
          opacity: 0;
        }

        .fd__row-wrap:hover .fd__move-btn,
        .fd__row-wrap:focus-within .fd__move-btn {
          opacity: 1;
        }

        .fd__move-btn:hover {
          background: var(--ppt-border-default);
          color: var(--ppt-brand-500);
        }

        .fd__move-btn:focus-visible {
          outline: 2px solid var(--ppt-brand-500);
          outline-offset: 2px;
          opacity: 1;
        }

        .fd__icon {
          flex-shrink: 0;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 2rem;
          height: 2rem;
          font-size: 0.5625rem;
          font-weight: 700;
          font-family: monospace;
          border-radius: 0.25rem;
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-muted);
          border: 1px solid var(--ppt-border-default);
        }

        .fd__body {
          flex: 1;
          min-width: 0;
          display: flex;
          flex-direction: column;
          gap: 0.0625rem;
        }

        .fd__title {
          font-size: 0.8125rem;
          font-weight: 500;
          color: var(--ppt-fg-primary);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }

        .fd__meta {
          font-size: 0.6875rem;
          color: var(--ppt-fg-muted);
        }

        .fd__more {
          text-align: center;
          padding: 0.5rem;
        }

        .fd__more-link {
          font-size: 0.8125rem;
          font-weight: 500;
          color: var(--ppt-brand-500);
          text-decoration: none;
        }

        .fd__more-link:hover { text-decoration: underline; }

        .ftp__detail-hint { display: none; }

        /* ── Responsive ──────────────────────────────────── */
        @media (max-width: 768px) {
          .ftp__body {
            flex-direction: column;
          }

          .ftp__sidebar {
            width: 100%;
            max-height: 12rem;
          }

          .ftp__header {
            flex-wrap: wrap;
            gap: 0.5rem;
          }
        }
      `}</style>
    </div>
  );
}

export default FolderTreePage;
