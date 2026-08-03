/**
 * MoveFolderDialog (gap-7a-2-folder-ui).
 *
 * Modal dialog that lets the user pick a destination folder to move
 * one or more documents into. It renders the folder tree inline, lets
 * the user click a folder to select it, and calls `onConfirm(folderId)`
 * when the user commits (null means "move to root / no folder").
 *
 * Usage:
 *   <MoveFolderDialog
 *     documentTitles={['Report Q1.pdf']}
 *     buildingId={buildingId}
 *     onConfirm={(folderId) => moveDocument({ documentId, folderId })}
 *     onCancel={() => setMoveTarget(null)}
 *   />
 */

import { type FolderTreeNode, useFolderTree } from '@ppt/api-client';
import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useFocusTrap } from '../../../hooks/useFocusTrap';

// ─── FolderOption ─────────────────────────────────────────────────────────────

interface FolderOptionProps {
  node: FolderTreeNode;
  depth: number;
  selectedId: string | null;
  onSelect: (id: string | null) => void;
}

function FolderOption({ node, depth, selectedId, onSelect }: FolderOptionProps) {
  const { folder, children } = node;
  const [expanded, setExpanded] = useState(depth < 1);
  const isSelected = selectedId === folder.id;
  const hasChildren = children.length > 0;

  return (
    <li className="mfd-option">
      <div
        className={`mfd-option__row${isSelected ? ' mfd-option__row--selected' : ''}`}
        style={{ paddingLeft: `${depth * 1.25 + 0.5}rem` }}
      >
        <button
          type="button"
          className={`mfd-option__toggle${!hasChildren ? ' mfd-option__toggle--leaf' : ''}`}
          onClick={() => setExpanded((p) => !p)}
          aria-expanded={hasChildren ? expanded : undefined}
          tabIndex={hasChildren ? 0 : -1}
          aria-label={expanded ? 'Zbaliť' : 'Rozbaliť'}
        >
          {hasChildren ? (
            <svg
              width="11"
              height="11"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.5"
              style={{
                transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)',
                transition: 'transform 0.12s',
              }}
              aria-hidden="true"
            >
              <polyline points="9 18 15 12 9 6" />
            </svg>
          ) : (
            <span className="mfd-option__dot" aria-hidden="true" />
          )}
        </button>

        <svg
          width="15"
          height="15"
          viewBox="0 0 24 24"
          fill={isSelected ? 'var(--ppt-brand-500)' : 'var(--ppt-fg-subtle)'}
          aria-hidden="true"
          className="mfd-option__icon"
        >
          <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
        </svg>

        <button
          type="button"
          className="mfd-option__name"
          onClick={() => onSelect(isSelected ? null : folder.id)}
          aria-pressed={isSelected}
        >
          {folder.name}
        </button>

        {isSelected && (
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="var(--ppt-brand-500)"
            strokeWidth="2.5"
            aria-hidden="true"
            className="mfd-option__check"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
        )}
      </div>

      {hasChildren && expanded && (
        <ul className="mfd-option__children">
          {children.map((child) => (
            <FolderOption
              key={child.folder.id}
              node={child}
              depth={depth + 1}
              selectedId={selectedId}
              onSelect={onSelect}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

// ─── FolderBreadcrumb ─────────────────────────────────────────────────────────

/**
 * Renders a breadcrumb trail for the currently-selected folder by walking
 * the flat/tree representation. Exported for standalone use in FolderTreePage.
 */

export interface FolderBreadcrumbProps {
  /** Ordered list of ancestors + current: [ { id, name }, … ] */
  crumbs: Array<{ id: string; name: string }>;
  /** Called when the user clicks a crumb — null means "root". */
  onNavigate: (folderId: string | null) => void;
  /** Extra CSS class. */
  className?: string;
}

export function FolderBreadcrumb({ crumbs, onNavigate, className = '' }: FolderBreadcrumbProps) {
  const { t } = useTranslation();
  return (
    <>
      <nav className={`folder-bc ${className}`} aria-label={t('aria.folderLocation')}>
        <button
          type="button"
          className="folder-bc__crumb folder-bc__crumb--link"
          onClick={() => onNavigate(null)}
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
            <line x1="8" y1="21" x2="16" y2="21" />
            <line x1="12" y1="17" x2="12" y2="21" />
          </svg>
          Všetky
        </button>

        {crumbs.map((crumb, idx) => {
          const isLast = idx === crumbs.length - 1;
          return (
            <span key={crumb.id} className="folder-bc__segment">
              <span className="folder-bc__sep" aria-hidden="true">
                /
              </span>
              {isLast ? (
                <span className="folder-bc__crumb folder-bc__crumb--current" aria-current="page">
                  {crumb.name}
                </span>
              ) : (
                <button
                  type="button"
                  className="folder-bc__crumb folder-bc__crumb--link"
                  onClick={() => onNavigate(crumb.id)}
                >
                  {crumb.name}
                </button>
              )}
            </span>
          );
        })}
      </nav>

      <style>{`
        .folder-bc {
          display: flex;
          align-items: center;
          flex-wrap: wrap;
          gap: 0;
          font-size: 0.8125rem;
        }
        .folder-bc__segment {
          display: flex;
          align-items: center;
        }
        .folder-bc__sep {
          padding: 0 0.25rem;
          color: var(--ppt-fg-muted);
        }
        .folder-bc__crumb {
          display: inline-flex;
          align-items: center;
          gap: 0.25rem;
          padding: 0;
          border: none;
          background: transparent;
          font-size: 0.8125rem;
          line-height: 1.5;
        }
        .folder-bc__crumb--link {
          color: var(--ppt-brand-500);
          cursor: pointer;
          text-decoration: none;
          font-weight: 500;
        }
        .folder-bc__crumb--link:hover {
          text-decoration: underline;
        }
        .folder-bc__crumb--link:focus-visible {
          outline: 2px solid var(--ppt-brand-500);
          outline-offset: 2px;
          border-radius: 0.125rem;
        }
        .folder-bc__crumb--current {
          color: var(--ppt-fg-primary);
          font-weight: 600;
          cursor: default;
        }
      `}</style>
    </>
  );
}

/**
 * Builds an ordered breadcrumb list from a folder tree for a given folderId.
 * Returns [] when folderId is null (root selected).
 */
export function buildFolderCrumbs(
  tree: FolderTreeNode[],
  folderId: string | null
): Array<{ id: string; name: string }> {
  if (!folderId) return [];

  function findPath(
    nodes: FolderTreeNode[],
    targetId: string,
    path: Array<{ id: string; name: string }>
  ): Array<{ id: string; name: string }> | null {
    for (const node of nodes) {
      const next = [...path, { id: node.folder.id, name: node.folder.name }];
      if (node.folder.id === targetId) return next;
      const found = findPath(node.children, targetId, next);
      if (found) return found;
    }
    return null;
  }

  return findPath(tree, folderId, []) ?? [];
}

// ─── MoveFolderDialog (main export) ──────────────────────────────────────────

export interface MoveFolderDialogProps {
  /** Human-readable titles of the documents being moved (shown in header). */
  documentTitles: string[];
  /** Optional building scope for the folder tree. */
  buildingId?: string;
  /** Currently assigned folder id (if any) — highlighted in tree. */
  currentFolderId?: string | null;
  /** Called with the chosen folder id (null = move to root). */
  onConfirm: (folderId: string | null) => void;
  /** Called when the user dismisses without confirming. */
  onCancel: () => void;
  /** Whether the move mutation is in flight. */
  isPending?: boolean;
}

export function MoveFolderDialog({
  documentTitles,
  buildingId,
  currentFolderId = null,
  onConfirm,
  onCancel,
  isPending = false,
}: MoveFolderDialogProps) {
  const { t } = useTranslation();
  const { data, isLoading, error } = useFolderTree(buildingId);
  const [selectedId, setSelectedId] = useState<string | null>(currentFolderId);
  const dialogRef = useRef<HTMLDivElement>(null);

  // Focus trap: Tab/Shift+Tab cycle within the dialog, Escape calls onCancel,
  // and focus is restored to the triggering element on unmount (WCAG 2.4.3).
  useFocusTrap(dialogRef, onCancel);

  const tree = data?.tree ?? [];
  const crumbs = selectedId ? buildFolderCrumbs(tree, selectedId) : [];
  const selectedName = selectedId ? (crumbs[crumbs.length - 1]?.name ?? selectedId) : null;

  const hasChanged = selectedId !== currentFolderId;

  return (
    <div
      className="mfd-backdrop"
      role="presentation"
      onClick={(e) => {
        if (e.target === e.currentTarget) onCancel();
      }}
    >
      <div
        ref={dialogRef}
        className="mfd"
        role="dialog"
        aria-modal="true"
        aria-label={t('aria.moveDocuments')}
        tabIndex={-1}
      >
        {/* Header */}
        <div className="mfd__header">
          <h3 className="mfd__title">Presunúť do priečinka</h3>
          <button
            type="button"
            className="mfd__close"
            onClick={onCancel}
            aria-label={t('aria.close')}
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.5"
              aria-hidden="true"
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        {/* Document summary */}
        <div className="mfd__docs">
          {documentTitles.length === 1 ? (
            <span className="mfd__doc-name">{documentTitles[0]}</span>
          ) : (
            <span className="mfd__doc-name">{documentTitles.length} dokumentov</span>
          )}
        </div>

        {/* Destination label */}
        <div className="mfd__dest-label">
          <span className="mfd__dest-prefix">Cieľ:</span>
          {selectedName ? (
            <span className="mfd__dest-value mfd__dest-value--folder">
              <svg
                width="13"
                height="13"
                viewBox="0 0 24 24"
                fill="var(--ppt-brand-500)"
                aria-hidden="true"
              >
                <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
              </svg>
              {selectedName}
            </span>
          ) : (
            <span className="mfd__dest-value mfd__dest-value--root">Koreň (bez priečinka)</span>
          )}
        </div>

        {/* Tree body */}
        <div className="mfd__body">
          {/* "No folder" / root option */}
          <button
            type="button"
            className={`mfd__root-option${selectedId === null ? ' mfd__root-option--selected' : ''}`}
            onClick={() => setSelectedId(null)}
            aria-pressed={selectedId === null}
          >
            <svg
              width="15"
              height="15"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
              <line x1="8" y1="21" x2="16" y2="21" />
              <line x1="12" y1="17" x2="12" y2="21" />
            </svg>
            Všetky dokumenty (bez priečinka)
            {selectedId === null && (
              <svg
                width="13"
                height="13"
                viewBox="0 0 24 24"
                fill="none"
                stroke="var(--ppt-brand-500)"
                strokeWidth="2.5"
                aria-hidden="true"
                className="mfd__root-check"
              >
                <polyline points="20 6 9 17 4 12" />
              </svg>
            )}
          </button>

          {isLoading && (
            <ul className="mfd__skeleton" aria-busy="true">
              {Array.from({ length: 4 }).map((_, i) => (
                <li key={i} className="mfd__skel-row" style={{ width: `${60 + i * 10}%` }} />
              ))}
            </ul>
          )}

          {error && !isLoading && (
            <p className="mfd__error" role="alert">
              Priečinky sa nepodarilo načítať.
            </p>
          )}

          {!isLoading && !error && tree.length === 0 && (
            <p className="mfd__empty">Žiadne priečinky</p>
          )}

          {!isLoading && !error && tree.length > 0 && (
            <ul className="mfd__tree">
              {tree.map((node) => (
                <FolderOption
                  key={node.folder.id}
                  node={node}
                  depth={0}
                  selectedId={selectedId}
                  onSelect={setSelectedId}
                />
              ))}
            </ul>
          )}
        </div>

        {/* Footer */}
        <div className="mfd__footer">
          <button
            type="button"
            className="mfd__btn mfd__btn--secondary"
            onClick={onCancel}
            disabled={isPending}
          >
            Zrušiť
          </button>
          <button
            type="button"
            className="mfd__btn mfd__btn--primary"
            onClick={() => onConfirm(selectedId)}
            disabled={isPending || !hasChanged}
            aria-busy={isPending}
          >
            {isPending ? 'Presúvam…' : 'Presunúť sem'}
          </button>
        </div>
      </div>

      <style>{`
        /* ── Backdrop ─────────────────────────────────────── */
        .mfd-backdrop {
          position: fixed;
          inset: 0;
          background: rgba(0,0,0,0.45);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 1100;
        }

        /* ── Dialog box ───────────────────────────────────── */
        .mfd {
          display: flex;
          flex-direction: column;
          width: min(28rem, calc(100vw - 2rem));
          max-height: min(36rem, calc(100dvh - 2rem));
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.75rem;
          box-shadow: 0 20px 60px rgba(0,0,0,0.2);
          outline: none;
          overflow: hidden;
        }

        /* ── Header ───────────────────────────────────────── */
        .mfd__header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 1rem 1.25rem 0.75rem;
          flex-shrink: 0;
        }

        .mfd__title {
          margin: 0;
          font-size: 1rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .mfd__close {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 1.75rem;
          height: 1.75rem;
          border: none;
          border-radius: 0.375rem;
          background: transparent;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          transition: all 0.12s;
        }

        .mfd__close:hover {
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-primary);
        }

        .mfd__close:focus-visible {
          outline: 2px solid var(--ppt-brand-500);
          outline-offset: 2px;
        }

        /* ── Document summary ─────────────────────────────── */
        .mfd__docs {
          padding: 0 1.25rem 0.5rem;
          flex-shrink: 0;
        }

        .mfd__doc-name {
          font-size: 0.8125rem;
          color: var(--ppt-fg-muted);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
          display: block;
        }

        /* ── Destination label ────────────────────────────── */
        .mfd__dest-label {
          display: flex;
          align-items: center;
          gap: 0.375rem;
          padding: 0.375rem 1.25rem;
          font-size: 0.8125rem;
          border-top: 1px solid var(--ppt-border-default);
          border-bottom: 1px solid var(--ppt-border-default);
          background: var(--ppt-bg-app);
          flex-shrink: 0;
        }

        .mfd__dest-prefix {
          color: var(--ppt-fg-subtle);
          font-weight: 600;
          font-size: 0.75rem;
          text-transform: uppercase;
          letter-spacing: 0.04em;
        }

        .mfd__dest-value {
          display: inline-flex;
          align-items: center;
          gap: 0.25rem;
          font-weight: 500;
        }

        .mfd__dest-value--folder { color: var(--ppt-brand-600, var(--ppt-brand-500)); }
        .mfd__dest-value--root   { color: var(--ppt-fg-muted); font-style: italic; }

        /* ── Tree body ────────────────────────────────────── */
        .mfd__body {
          flex: 1;
          overflow-y: auto;
          padding: 0.375rem 0;
          min-height: 0;
        }

        /* Root option */
        .mfd__root-option {
          display: flex;
          align-items: center;
          gap: 0.375rem;
          width: 100%;
          padding: 0.5rem 1rem;
          font-size: 0.8125rem;
          font-weight: 500;
          color: var(--ppt-fg-muted);
          background: transparent;
          border: none;
          cursor: pointer;
          text-align: left;
          transition: all 0.12s;
          border-bottom: 1px solid var(--ppt-border-default);
          margin-bottom: 0.25rem;
        }

        .mfd__root-option:hover {
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-primary);
        }

        .mfd__root-option:focus-visible {
          outline: 2px solid var(--ppt-brand-500);
          outline-offset: -2px;
        }

        .mfd__root-option--selected {
          background: color-mix(in srgb, var(--ppt-brand-500) 8%, var(--ppt-bg-surface));
          color: var(--ppt-brand-600, var(--ppt-brand-500));
        }

        .mfd__root-check {
          margin-left: auto;
        }

        /* Tree list */
        .mfd__tree {
          list-style: none;
          padding: 0;
          margin: 0;
        }

        /* Folder option */
        .mfd-option { list-style: none; }

        .mfd-option__children {
          list-style: none;
          padding: 0;
          margin: 0;
        }

        .mfd-option__row {
          display: flex;
          align-items: center;
          gap: 0.25rem;
          padding-right: 0.75rem;
          height: 2rem;
          transition: background 0.1s;
          cursor: default;
        }

        .mfd-option__row:hover { background: var(--ppt-bg-app); }

        .mfd-option__row--selected {
          background: color-mix(in srgb, var(--ppt-brand-500) 10%, var(--ppt-bg-surface));
        }

        .mfd-option__toggle {
          flex-shrink: 0;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 1.125rem;
          height: 1.125rem;
          border: none;
          background: transparent;
          color: var(--ppt-fg-subtle);
          cursor: pointer;
          border-radius: 0.2rem;
          transition: all 0.1s;
        }

        .mfd-option__toggle--leaf { cursor: default; pointer-events: none; }

        .mfd-option__toggle:hover { background: var(--ppt-border-default); color: var(--ppt-fg-primary); }

        .mfd-option__dot {
          display: block;
          width: 3px;
          height: 3px;
          border-radius: 50%;
          background: var(--ppt-border-default);
        }

        .mfd-option__icon { flex-shrink: 0; }

        .mfd-option__name {
          flex: 1;
          text-align: left;
          font-size: 0.8125rem;
          color: var(--ppt-fg-primary);
          background: transparent;
          border: none;
          cursor: pointer;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
          padding: 0;
          line-height: 1.4;
        }

        .mfd-option__name:focus-visible {
          outline: 2px solid var(--ppt-brand-500);
          outline-offset: 2px;
          border-radius: 0.125rem;
        }

        .mfd-option__row--selected .mfd-option__name {
          font-weight: 600;
          color: var(--ppt-brand-600, var(--ppt-brand-500));
        }

        .mfd-option__check { flex-shrink: 0; }

        /* Loading skeleton */
        .mfd__skeleton {
          list-style: none;
          padding: 0.75rem 1rem;
          margin: 0;
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
        }

        .mfd__skel-row {
          height: 1.25rem;
          border-radius: 0.25rem;
          background: linear-gradient(
            90deg,
            var(--ppt-bg-app) 25%,
            var(--ppt-border-default) 50%,
            var(--ppt-bg-app) 75%
          );
          background-size: 200% 100%;
          animation: mfd-shimmer 1.4s ease-in-out infinite;
        }

        @keyframes mfd-shimmer {
          0%  { background-position: 200% 0; }
          100% { background-position: -200% 0; }
        }

        .mfd__error {
          padding: 1rem;
          text-align: center;
          color: #b91c1c;
          font-size: 0.875rem;
        }

        .mfd__empty {
          padding: 2rem 1rem;
          text-align: center;
          color: var(--ppt-fg-muted);
          font-size: 0.875rem;
        }

        /* ── Footer ───────────────────────────────────────── */
        .mfd__footer {
          display: flex;
          justify-content: flex-end;
          gap: 0.5rem;
          padding: 0.875rem 1.25rem;
          border-top: 1px solid var(--ppt-border-default);
          flex-shrink: 0;
        }

        .mfd__btn {
          padding: 0.4375rem 1rem;
          font-size: 0.875rem;
          font-weight: 600;
          border-radius: 0.375rem;
          border: none;
          cursor: pointer;
          transition: opacity 0.15s;
        }

        .mfd__btn:disabled {
          opacity: 0.45;
          cursor: not-allowed;
        }

        .mfd__btn:not(:disabled):hover { opacity: 0.85; }

        .mfd__btn:focus-visible {
          outline: 2px solid var(--ppt-brand-500);
          outline-offset: 2px;
        }

        .mfd__btn--secondary {
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-muted);
          border: 1px solid var(--ppt-border-default);
        }

        .mfd__btn--primary {
          background: var(--ppt-brand-500);
          color: #fff;
        }
      `}</style>
    </div>
  );
}

export default MoveFolderDialog;
