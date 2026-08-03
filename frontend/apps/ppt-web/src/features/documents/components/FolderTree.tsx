/**
 * FolderTree component (gap-7a-2).
 *
 * Renders the 5-level folder hierarchy returned by GET /api/v1/documents/folders/tree.
 * Supports creating, renaming, and deleting folders inline.
 * Selecting a folder emits `onSelectFolder` so a parent page can filter documents.
 * The backend enforces circular-reference prevention on create/move.
 */

import {
  type FolderTreeNode,
  useCreateFolder,
  useDeleteFolder,
  useFolderTree,
  useUpdateFolder,
} from '@ppt/api-client';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../../../components/Toast';

// ─── helpers ─────────────────────────────────────────────────────────────────

const MAX_DEPTH = 5;

function countNodes(nodes: FolderTreeNode[]): number {
  return nodes.reduce((acc, n) => acc + 1 + countNodes(n.children), 0);
}

// ─── sub-components ───────────────────────────────────────────────────────────

interface FolderNodeProps {
  node: FolderTreeNode;
  depth: number;
  selectedId: string | null;
  onSelect: (id: string) => void;
  onRename: (id: string, name: string) => void;
  onDelete: (id: string, name: string) => void;
  onAddChild: (parentId: string, depth: number) => void;
  editingId: string | null;
  setEditingId: (id: string | null) => void;
}

function FolderNode({
  node,
  depth,
  selectedId,
  onSelect,
  onRename,
  onDelete,
  onAddChild,
  editingId,
  setEditingId,
}: FolderNodeProps) {
  const { t } = useTranslation();
  const { folder, children, document_count } = node;
  const [expanded, setExpanded] = useState(depth === 0);
  const [editValue, setEditValue] = useState(folder.name);
  const [showActions, setShowActions] = useState(false);
  const editRef = useRef<HTMLInputElement>(null);
  const isEditing = editingId === folder.id;
  const isSelected = selectedId === folder.id;
  const hasChildren = children.length > 0;
  const canAddChild = depth < MAX_DEPTH - 1;

  const handleEditStart = () => {
    setEditValue(folder.name);
    setEditingId(folder.id);
    setTimeout(() => editRef.current?.select(), 0);
  };

  const handleEditCommit = () => {
    const trimmed = editValue.trim();
    if (trimmed && trimmed !== folder.name) {
      onRename(folder.id, trimmed);
    }
    setEditingId(null);
  };

  const handleEditKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleEditCommit();
    if (e.key === 'Escape') {
      setEditValue(folder.name);
      setEditingId(null);
    }
  };

  return (
    <li className="folder-node" data-depth={depth}>
      <div
        className={`folder-row${isSelected ? ' folder-row--selected' : ''}`}
        style={{ paddingLeft: `${depth * 1.25 + 0.5}rem` }}
        onMouseEnter={() => setShowActions(true)}
        onMouseLeave={() => setShowActions(false)}
      >
        {/* Expand / collapse toggle */}
        <button
          type="button"
          className={`folder-row__toggle${hasChildren ? '' : ' folder-row__toggle--leaf'}`}
          onClick={() => setExpanded((p) => !p)}
          aria-expanded={expanded}
          aria-label={expanded ? 'Zbaliť priečinok' : 'Rozbaliť priečinok'}
          tabIndex={hasChildren ? 0 : -1}
        >
          {hasChildren ? (
            <svg
              width="12"
              height="12"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.5"
              style={{
                transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)',
                transition: 'transform 0.15s',
              }}
              aria-hidden="true"
            >
              <polyline points="9 18 15 12 9 6" />
            </svg>
          ) : (
            <span className="folder-row__dot" aria-hidden="true" />
          )}
        </button>

        {/* Folder icon */}
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill={isSelected ? 'var(--ppt-brand-500)' : 'var(--ppt-fg-subtle)'}
          aria-hidden="true"
          className="folder-row__icon"
        >
          <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
        </svg>

        {/* Name (inline edit mode) */}
        {isEditing ? (
          <input
            ref={editRef}
            type="text"
            className="folder-row__edit-input"
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            onBlur={handleEditCommit}
            onKeyDown={handleEditKeyDown}
            aria-label={t('aria.renameFolder')}
          />
        ) : (
          <button
            type="button"
            className="folder-row__name"
            onClick={() => onSelect(folder.id)}
            aria-pressed={isSelected}
          >
            {folder.name}
          </button>
        )}

        {/* Document count badge */}
        {document_count > 0 && (
          <span className="folder-row__count" aria-label={`${document_count} dokumentov`}>
            {document_count}
          </span>
        )}

        {/* Action buttons (hover) */}
        {showActions && !isEditing && (
          <div className="folder-row__actions" role="group" aria-label={t('aria.folderActions')}>
            {canAddChild && (
              <button
                type="button"
                className="folder-row__action-btn"
                onClick={() => {
                  setExpanded(true);
                  onAddChild(folder.id, depth);
                }}
                title="Nový podpriečinok"
                aria-label={t('aria.newSubfolder')}
              >
                <svg
                  width="13"
                  height="13"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  aria-hidden="true"
                >
                  <line x1="12" y1="5" x2="12" y2="19" />
                  <line x1="5" y1="12" x2="19" y2="12" />
                </svg>
              </button>
            )}
            <button
              type="button"
              className="folder-row__action-btn"
              onClick={handleEditStart}
              title="Premenovať"
              aria-label={t('aria.renameFolder')}
            >
              <svg
                width="13"
                height="13"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                aria-hidden="true"
              >
                <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7" />
                <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z" />
              </svg>
            </button>
            <button
              type="button"
              className="folder-row__action-btn folder-row__action-btn--danger"
              onClick={() => onDelete(folder.id, folder.name)}
              title="Odstrániť"
              aria-label={t('aria.deleteFolder')}
            >
              <svg
                width="13"
                height="13"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                aria-hidden="true"
              >
                <polyline points="3 6 5 6 21 6" />
                <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a1 1 0 011-1h4a1 1 0 011 1v2" />
              </svg>
            </button>
          </div>
        )}
      </div>

      {/* Children */}
      {hasChildren && expanded && (
        <ul className="folder-node__children" role="group">
          {children.map((child) => (
            <FolderNode
              key={child.folder.id}
              node={child}
              depth={depth + 1}
              selectedId={selectedId}
              onSelect={onSelect}
              onRename={onRename}
              onDelete={onDelete}
              onAddChild={onAddChild}
              editingId={editingId}
              setEditingId={setEditingId}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

// ─── Inline create form ───────────────────────────────────────────────────────

interface NewFolderRowProps {
  parentId: string | null;
  depth: number;
  onCommit: (name: string) => void;
  onCancel: () => void;
}

function NewFolderRow({ parentId: _parentId, depth, onCommit, onCancel }: NewFolderRowProps) {
  const { t } = useTranslation();
  const [name, setName] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  // Focus input on mount (avoid autoFocus lint rule)
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      const trimmed = name.trim();
      if (trimmed) onCommit(trimmed);
    }
    if (e.key === 'Escape') onCancel();
  };

  return (
    <li className="folder-node" data-depth={depth}>
      <div
        className="folder-row folder-row--new"
        style={{ paddingLeft: `${depth * 1.25 + 0.5}rem` }}
      >
        <span className="folder-row__toggle folder-row__toggle--leaf" aria-hidden="true" />
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="var(--ppt-brand-500)"
          aria-hidden="true"
          className="folder-row__icon"
        >
          <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
        </svg>
        <input
          ref={inputRef}
          type="text"
          className="folder-row__edit-input"
          placeholder="Názov priečinka…"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onBlur={() => {
            const trimmed = name.trim();
            if (trimmed) onCommit(trimmed);
            else onCancel();
          }}
          onKeyDown={handleKeyDown}
          aria-label={t('aria.newFolderName')}
        />
        <button
          type="button"
          className="folder-row__action-btn"
          onClick={() => {
            const trimmed = name.trim();
            if (trimmed) onCommit(trimmed);
          }}
          aria-label={t('aria.createFolder')}
        >
          <svg
            width="13"
            height="13"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            aria-hidden="true"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
        </button>
        <button
          type="button"
          className="folder-row__action-btn folder-row__action-btn--danger"
          onClick={onCancel}
          aria-label={t('aria.cancel')}
        >
          <svg
            width="13"
            height="13"
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
    </li>
  );
}

// ─── Delete confirm dialog ────────────────────────────────────────────────────

interface DeleteDialogProps {
  folderName: string;
  onConfirm: (cascade: boolean) => void;
  onCancel: () => void;
}

function DeleteDialog({ folderName, onConfirm, onCancel }: DeleteDialogProps) {
  const { t } = useTranslation();
  return (
    <div
      className="folder-delete-dialog"
      role="dialog"
      aria-modal="true"
      aria-label={t('aria.deleteFolder')}
    >
      <div className="folder-delete-dialog__box">
        <h3 className="folder-delete-dialog__title">Odstrániť priečinok</h3>
        <p className="folder-delete-dialog__body">
          Naozaj chcete odstrániť priečinok <strong>{folderName}</strong>?
        </p>
        <div className="folder-delete-dialog__actions">
          <button
            type="button"
            className="folder-delete-dialog__btn folder-delete-dialog__btn--secondary"
            onClick={onCancel}
          >
            Zrušiť
          </button>
          <button
            type="button"
            className="folder-delete-dialog__btn folder-delete-dialog__btn--warning"
            onClick={() => onConfirm(false)}
          >
            Odstrániť (presunúť dokumenty do koreňa)
          </button>
          <button
            type="button"
            className="folder-delete-dialog__btn folder-delete-dialog__btn--danger"
            onClick={() => onConfirm(true)}
          >
            Odstrániť vrátane dokumentov
          </button>
        </div>
      </div>
    </div>
  );
}

// ─── main component ───────────────────────────────────────────────────────────

export interface FolderTreeProps {
  buildingId?: string;
  selectedFolderId: string | null;
  onSelectFolder: (folderId: string | null) => void;
}

export function FolderTree({ buildingId, selectedFolderId, onSelectFolder }: FolderTreeProps) {
  const { data, isLoading, error, refetch } = useFolderTree(buildingId);
  const createFolder = useCreateFolder();
  const updateFolder = useUpdateFolder();
  const deleteFolder = useDeleteFolder();
  const { showToast } = useToast();
  const { t } = useTranslation();

  // inline edit state
  const [editingId, setEditingId] = useState<string | null>(null);

  // new-folder creation: { parentId, depth }
  const [newFolder, setNewFolder] = useState<{ parentId: string | null; depth: number } | null>(
    null
  );

  // delete confirm state: { id, name }
  const [deleteTarget, setDeleteTarget] = useState<{ id: string; name: string } | null>(null);

  // ── handlers ─────────────────────────────────────────────────────────────

  const handleSelectFolder = useCallback(
    (id: string) => {
      onSelectFolder(selectedFolderId === id ? null : id);
    },
    [selectedFolderId, onSelectFolder]
  );

  const handleRename = useCallback(
    async (id: string, name: string) => {
      try {
        await updateFolder.mutateAsync({ id, data: { name } });
      } catch (err) {
        showToast({
          type: 'error',
          title: t('documents.folder.renameError', 'Rename failed'),
          message:
            err instanceof Error
              ? err.message
              : t('documents.folder.renameErrorMessage', 'Failed to rename folder'),
        });
      }
    },
    [updateFolder, showToast, t]
  );

  const handleDeleteRequest = useCallback((id: string, name: string) => {
    setDeleteTarget({ id, name });
  }, []);

  const handleDeleteConfirm = useCallback(
    async (cascade: boolean) => {
      if (!deleteTarget) return;
      try {
        await deleteFolder.mutateAsync({ id: deleteTarget.id, cascade });
        if (selectedFolderId === deleteTarget.id) onSelectFolder(null);
      } catch (err) {
        showToast({
          type: 'error',
          title: t('documents.folder.deleteError', 'Delete failed'),
          message:
            err instanceof Error
              ? err.message
              : t('documents.folder.deleteErrorMessage', 'Failed to delete folder'),
        });
      } finally {
        setDeleteTarget(null);
      }
    },
    [deleteTarget, deleteFolder, selectedFolderId, onSelectFolder, showToast, t]
  );

  const handleAddChild = useCallback((parentId: string, depth: number) => {
    setNewFolder({ parentId, depth: depth + 1 });
  }, []);

  const handleCreateCommit = useCallback(
    async (name: string) => {
      if (!newFolder) return;
      try {
        await createFolder.mutateAsync({
          name,
          parent_id: newFolder.parentId ?? undefined,
          building_id: buildingId,
        });
      } catch (err) {
        showToast({
          type: 'error',
          title: t('documents.folder.createError', 'Create failed'),
          message:
            err instanceof Error
              ? err.message
              : t('documents.folder.createErrorMessage', 'Failed to create folder'),
        });
      } finally {
        setNewFolder(null);
      }
    },
    [newFolder, createFolder, buildingId, showToast, t]
  );

  const tree = data?.tree ?? [];
  const totalFolders = countNodes(tree);

  // ── render ────────────────────────────────────────────────────────────────

  return (
    <div className="folder-tree">
      {/* Header */}
      <div className="folder-tree__header">
        <span className="folder-tree__title">Priečinky</span>
        {totalFolders > 0 && (
          <span className="folder-tree__count" aria-label={`${totalFolders} priečinkov`}>
            {totalFolders}
          </span>
        )}
        <div className="folder-tree__toolbar">
          <button
            type="button"
            className="folder-tree__add-root"
            onClick={() => setNewFolder({ parentId: null, depth: 0 })}
            title="Nový priečinok"
            aria-label={t('aria.newRootFolder')}
            disabled={createFolder.isPending}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.5"
              aria-hidden="true"
            >
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          </button>
        </div>
      </div>

      {/* "All documents" root item */}
      <button
        type="button"
        className={`folder-tree__all${selectedFolderId === null ? ' folder-tree__all--active' : ''}`}
        onClick={() => onSelectFolder(null)}
        aria-pressed={selectedFolderId === null}
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
          <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
          <line x1="8" y1="21" x2="16" y2="21" />
          <line x1="12" y1="17" x2="12" y2="21" />
        </svg>
        Všetky dokumenty
      </button>

      {/* Loading */}
      {isLoading && (
        <ul
          className="folder-tree__skeleton"
          aria-busy="true"
          aria-label={t('aria.loadingFolders')}
        >
          {Array.from({ length: 4 }).map((_, i) => (
            <li
              key={i}
              className="folder-tree__skel-row"
              style={{ width: `${70 + (i % 3) * 10}%` }}
            />
          ))}
        </ul>
      )}

      {/* Error */}
      {error && !isLoading && (
        <div className="folder-tree__error" role="alert">
          <p>Priečinky sa nepodarilo načítať.</p>
          <button type="button" className="folder-tree__retry" onClick={() => refetch()}>
            Skúsiť znova
          </button>
        </div>
      )}

      {/* Empty */}
      {!isLoading && !error && tree.length === 0 && !newFolder && (
        <div className="folder-tree__empty">
          <p>Žiadne priečinky</p>
          <button
            type="button"
            className="folder-tree__retry"
            onClick={() => setNewFolder({ parentId: null, depth: 0 })}
          >
            Vytvoriť prvý priečinok
          </button>
        </div>
      )}

      {/* Tree */}
      {!isLoading && !error && (tree.length > 0 || newFolder) && (
        <ul className="folder-tree__list" aria-label={t('aria.documentFolders')}>
          {tree.map((node) => (
            <FolderNode
              key={node.folder.id}
              node={node}
              depth={0}
              selectedId={selectedFolderId}
              onSelect={handleSelectFolder}
              onRename={handleRename}
              onDelete={handleDeleteRequest}
              onAddChild={handleAddChild}
              editingId={editingId}
              setEditingId={setEditingId}
            />
          ))}
          {/* Root-level new folder input */}
          {newFolder && newFolder.parentId === null && (
            <NewFolderRow
              parentId={null}
              depth={0}
              onCommit={handleCreateCommit}
              onCancel={() => setNewFolder(null)}
            />
          )}
        </ul>
      )}

      {/* Delete confirm dialog */}
      {deleteTarget && (
        <DeleteDialog
          folderName={deleteTarget.name}
          onConfirm={handleDeleteConfirm}
          onCancel={() => setDeleteTarget(null)}
        />
      )}

      <style>{`
        /* ── Container ───────────────────────────────────── */
        .folder-tree {
          display: flex;
          flex-direction: column;
          gap: 0;
          user-select: none;
          position: relative;
        }

        /* ── Header ──────────────────────────────────────── */
        .folder-tree__header {
          display: flex;
          align-items: center;
          gap: 0.375rem;
          padding: 0.5rem 0.75rem;
          border-bottom: 1px solid var(--ppt-border-default);
        }

        .folder-tree__title {
          font-size: 0.6875rem;
          font-weight: 700;
          text-transform: uppercase;
          letter-spacing: 0.06em;
          color: var(--ppt-fg-subtle);
          flex: 1;
        }

        .folder-tree__count {
          font-size: 0.6875rem;
          font-weight: 600;
          padding: 0.0625rem 0.375rem;
          background: var(--ppt-bg-app);
          border: 1px solid var(--ppt-border-default);
          border-radius: 9999px;
          color: var(--ppt-fg-muted);
        }

        .folder-tree__toolbar {
          display: flex;
          gap: 0.25rem;
        }

        .folder-tree__add-root {
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
          transition: all 0.12s;
        }

        .folder-tree__add-root:hover {
          background: color-mix(in srgb, var(--ppt-brand-500) 12%, transparent);
          color: var(--ppt-brand-500);
        }

        .folder-tree__add-root:disabled {
          opacity: 0.4;
          cursor: not-allowed;
        }

        /* ── "All documents" root ────────────────────────── */
        .folder-tree__all {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          width: 100%;
          padding: 0.5rem 0.75rem;
          font-size: 0.8125rem;
          font-weight: 500;
          color: var(--ppt-fg-muted);
          background: transparent;
          border: none;
          border-bottom: 1px solid var(--ppt-border-default);
          cursor: pointer;
          text-align: left;
          transition: all 0.12s;
        }

        .folder-tree__all:hover {
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-primary);
        }

        .folder-tree__all--active {
          background: color-mix(in srgb, var(--ppt-brand-500) 8%, var(--ppt-bg-surface));
          color: var(--ppt-brand-600, var(--ppt-brand-500));
          font-weight: 600;
        }

        /* ── Tree list ───────────────────────────────────── */
        .folder-tree__list {
          list-style: none;
          padding: 0.25rem 0;
          margin: 0;
        }

        /* ── Folder node ─────────────────────────────────── */
        .folder-node {
          list-style: none;
        }

        .folder-node__children {
          list-style: none;
          padding: 0;
          margin: 0;
        }

        .folder-row {
          display: flex;
          align-items: center;
          gap: 0.25rem;
          padding-right: 0.5rem;
          height: 2rem;
          border-radius: 0;
          cursor: default;
          transition: background 0.1s;
          position: relative;
        }

        .folder-row:hover {
          background: var(--ppt-bg-app);
        }

        .folder-row--selected {
          background: color-mix(in srgb, var(--ppt-brand-500) 10%, var(--ppt-bg-surface));
        }

        .folder-row--selected:hover {
          background: color-mix(in srgb, var(--ppt-brand-500) 14%, var(--ppt-bg-surface));
        }

        .folder-row--new {
          background: color-mix(in srgb, var(--ppt-brand-500) 5%, var(--ppt-bg-surface));
        }

        /* ── Toggle button ───────────────────────────────── */
        .folder-row__toggle {
          flex-shrink: 0;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 1.25rem;
          height: 1.25rem;
          border: none;
          background: transparent;
          color: var(--ppt-fg-subtle);
          cursor: pointer;
          border-radius: 0.25rem;
          transition: all 0.1s;
        }

        .folder-row__toggle:hover {
          background: var(--ppt-border-default);
          color: var(--ppt-fg-primary);
        }

        .folder-row__toggle--leaf {
          cursor: default;
          pointer-events: none;
        }

        .folder-row__dot {
          display: block;
          width: 4px;
          height: 4px;
          border-radius: 50%;
          background: var(--ppt-border-default);
        }

        /* ── Folder icon ─────────────────────────────────── */
        .folder-row__icon {
          flex-shrink: 0;
        }

        /* ── Name button ─────────────────────────────────── */
        .folder-row__name {
          flex: 1;
          min-width: 0;
          text-align: left;
          font-size: 0.8125rem;
          font-weight: 400;
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

        .folder-row--selected .folder-row__name {
          font-weight: 600;
          color: var(--ppt-brand-600, var(--ppt-brand-500));
        }

        /* ── Edit input ──────────────────────────────────── */
        .folder-row__edit-input {
          flex: 1;
          min-width: 0;
          font-size: 0.8125rem;
          font-weight: 400;
          color: var(--ppt-fg-primary);
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-brand-500);
          border-radius: 0.25rem;
          padding: 0.125rem 0.375rem;
          outline: none;
          box-shadow: 0 0 0 2px color-mix(in srgb, var(--ppt-brand-500) 20%, transparent);
        }

        /* ── Count badge ─────────────────────────────────── */
        .folder-row__count {
          flex-shrink: 0;
          font-size: 0.625rem;
          font-weight: 600;
          padding: 0.0625rem 0.3125rem;
          border-radius: 9999px;
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-muted);
          border: 1px solid var(--ppt-border-default);
        }

        /* ── Action buttons ──────────────────────────────── */
        .folder-row__actions {
          display: flex;
          gap: 0.125rem;
          align-items: center;
          flex-shrink: 0;
        }

        .folder-row__action-btn {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 1.5rem;
          height: 1.5rem;
          border: none;
          border-radius: 0.25rem;
          background: transparent;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          transition: all 0.1s;
        }

        .folder-row__action-btn:hover {
          background: var(--ppt-border-default);
          color: var(--ppt-fg-primary);
        }

        .folder-row__action-btn--danger:hover {
          background: color-mix(in srgb, #ef4444 15%, transparent);
          color: #b91c1c;
        }

        /* ── Loading skeleton ────────────────────────────── */
        .folder-tree__skeleton {
          list-style: none;
          padding: 0.5rem 0.75rem;
          margin: 0;
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
        }

        .folder-tree__skel-row {
          height: 1.25rem;
          border-radius: 0.25rem;
          background: linear-gradient(
            90deg,
            var(--ppt-bg-app) 25%,
            var(--ppt-border-default) 50%,
            var(--ppt-bg-app) 75%
          );
          background-size: 200% 100%;
          animation: folder-shimmer 1.4s ease-in-out infinite;
        }

        @keyframes folder-shimmer {
          0%  { background-position: 200% 0; }
          100% { background-position: -200% 0; }
        }

        /* ── Error ───────────────────────────────────────── */
        .folder-tree__error {
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
          padding: 0.75rem;
          font-size: 0.8125rem;
          color: #b91c1c;
          text-align: center;
        }

        .folder-tree__retry {
          align-self: center;
          padding: 0.25rem 0.75rem;
          font-size: 0.75rem;
          font-weight: 600;
          background: var(--ppt-brand-500);
          color: #fff;
          border: none;
          border-radius: 0.375rem;
          cursor: pointer;
          transition: opacity 0.15s;
        }

        .folder-tree__retry:hover { opacity: 0.85; }

        /* ── Empty ───────────────────────────────────────── */
        .folder-tree__empty {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 0.5rem;
          padding: 1.5rem 0.75rem;
          font-size: 0.8125rem;
          color: var(--ppt-fg-muted);
          text-align: center;
        }

        /* ── Delete dialog ───────────────────────────────── */
        .folder-delete-dialog {
          position: fixed;
          inset: 0;
          background: rgba(0,0,0,0.45);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 1000;
        }

        .folder-delete-dialog__box {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.75rem;
          padding: 1.5rem;
          max-width: 26rem;
          width: calc(100vw - 2rem);
          box-shadow: 0 20px 60px rgba(0,0,0,0.2);
        }

        .folder-delete-dialog__title {
          margin: 0 0 0.5rem;
          font-size: 1rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .folder-delete-dialog__body {
          margin: 0 0 1.25rem;
          font-size: 0.875rem;
          color: var(--ppt-fg-muted);
          line-height: 1.5;
        }

        .folder-delete-dialog__actions {
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
        }

        .folder-delete-dialog__btn {
          padding: 0.5rem 1rem;
          font-size: 0.875rem;
          font-weight: 600;
          border: none;
          border-radius: 0.375rem;
          cursor: pointer;
          transition: opacity 0.15s;
        }

        .folder-delete-dialog__btn:hover { opacity: 0.85; }

        .folder-delete-dialog__btn--secondary {
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-muted);
          border: 1px solid var(--ppt-border-default);
        }

        .folder-delete-dialog__btn--warning {
          background: #f59e0b;
          color: #fff;
        }

        .folder-delete-dialog__btn--danger {
          background: #ef4444;
          color: #fff;
        }
      `}</style>
    </div>
  );
}

export default FolderTree;
