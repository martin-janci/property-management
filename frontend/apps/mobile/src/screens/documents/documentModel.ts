import type { ApiFolderTreeNode } from '../../hooks/useFolderTree';
import type { AccessScope } from './DocumentPermissionsScreen';

// `ApiFolderTreeNode` is owned by the folder-tree hook (the data layer it
// describes). Re-exported here so importers of the documents data layer keep
// working without a screen→hook coupling.
export type { ApiFolderTreeNode };

export type DocumentType = 'folder' | 'pdf' | 'image' | 'document' | 'spreadsheet';
export type DocumentStatus = 'published' | 'draft' | 'archived';

export interface Document {
  id: string;
  name: string;
  type: DocumentType;
  size?: number;
  createdAt: string;
  updatedAt: string;
  parentId: string | null;
  downloadUrl?: string;
  children?: Document[];
  /** Number of documents directly in a folder (folder rows only, gap-7a-2). */
  documentCount?: number;
  /** RLS-enforced audience scope returned from the server (gap-7a-3). */
  accessScope?: AccessScope;
  status?: DocumentStatus;
}

/**
 * Item shape of `GET /api/v1/documents` (`DocumentSummary` on the server).
 *
 * Fields mirror the backend exactly: `title` (not `name`), `file_name`
 * (not `file_path`), `mime_type` (not `content_type`), and `created_at`
 * (the summary has no separate upload timestamp). The summary carries no
 * `access_scope` or `status` — those live on the document detail/permissions
 * endpoints, not the list.
 */
export interface ApiDocument {
  id: string;
  title: string;
  category: string;
  file_name: string;
  mime_type: string;
  size_bytes: number;
  folder_id?: string | null;
  created_at: string;
}

export interface ApiDocumentListResponse {
  documents: ApiDocument[];
  count?: number;
  total?: number;
}

/** Lightweight breadcrumb entry: just enough to render + look up children. */
export interface FolderCrumb {
  id: string;
  name: string;
}

/**
 * Walk the nested folder tree to the folder identified by `path` (a list of
 * folder ids from root downward) and return its direct subfolders. An empty
 * path returns the root-level folders.
 *
 * Exported for unit-testing (feat-mobile-document-folder-organization).
 */
export function subfoldersAtPath(
  tree: ApiFolderTreeNode[],
  path: ReadonlyArray<FolderCrumb>
): ApiFolderTreeNode[] {
  let level = tree;
  for (const crumb of path) {
    const match = level.find((n) => n.id === crumb.id);
    if (!match) return [];
    level = match.children ?? [];
  }
  return level;
}

/**
 * Map a folder tree node to the screen's unified `Document` row shape.
 *
 * Exported for unit-testing (feat-mobile-document-folder-organization).
 */
export function folderNodeToDocument(node: ApiFolderTreeNode): Document {
  return {
    id: node.id,
    name: node.name,
    type: 'folder',
    createdAt: '',
    updatedAt: '',
    parentId: node.parent_id ?? null,
    // Carry the live document count so the row meta ("N items") is real
    // rather than always 0. `children` here is the on-tree subfolder list,
    // which the row's "items" count intentionally does not use.
    documentCount: node.document_count,
  };
}

export function pickDocumentType(d: ApiDocument): DocumentType {
  const mime = (d.mime_type ?? '').toLowerCase();
  const fileName = (d.file_name ?? d.title).toLowerCase();
  if (mime.includes('pdf') || fileName.endsWith('.pdf')) return 'pdf';
  if (mime.startsWith('image/') || /\.(png|jpe?g|gif|webp)$/.test(fileName)) return 'image';
  if (mime.includes('spreadsheet') || mime.includes('excel') || /\.(xlsx?|csv)$/.test(fileName))
    return 'spreadsheet';
  return 'document';
}

export function toUiDocument(d: ApiDocument): Document {
  return {
    id: d.id,
    name: d.title,
    type: pickDocumentType(d),
    size: d.size_bytes ?? undefined,
    createdAt: d.created_at,
    updatedAt: d.created_at,
    parentId: d.folder_id ?? null,
    downloadUrl: undefined,
    children: undefined,
  };
}
