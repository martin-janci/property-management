/**
 * useFolderTree — live folder hierarchy for the document module (gap-7a-2).
 *
 * Wraps the `GET /api/v1/documents/folders/tree` query so that any screen that
 * needs the folder hierarchy (DocumentsScreen, DocumentUploadScreen, future
 * move/copy flows) can share the same TanStack Query key and staleTime without
 * duplicating the endpoint path and cache options.
 *
 * The hook is deliberately thin: it owns the query key and endpoint address,
 * but forwards all other TanStack Query options to the caller so components
 * retain full control over enabled/staleTime overrides.
 */

import type { UseQueryOptions } from '@tanstack/react-query';
import { useApiQuery } from './useApi';
import type { ApiFolderTreeNode } from '../screens/documents/DocumentsScreen';

// ─── API shape ────────────────────────────────────────────────────────────────

export interface FolderTreeResponse {
  tree: ApiFolderTreeNode[];
}

/** Stable query key used for the folder tree. Re-exported so callers can
 *  invalidate it without hard-coding the string. */
export const FOLDER_TREE_QUERY_KEY = ['documents', 'folders', 'tree'] as const;

// ─── Hook ─────────────────────────────────────────────────────────────────────

export type UseFolderTreeOptions = Omit<
  UseQueryOptions<FolderTreeResponse, Error, FolderTreeResponse, readonly unknown[]>,
  'queryKey' | 'queryFn'
>;

/**
 * Fetch the live folder tree from the API.
 *
 * ```tsx
 * const { folderTree, isLoading, refetch } = useFolderTree();
 * ```
 */
export function useFolderTree(options?: UseFolderTreeOptions) {
  const result = useApiQuery<FolderTreeResponse>(
    FOLDER_TREE_QUERY_KEY,
    '/api/v1/documents/folders/tree',
    {
      staleTime: 60_000,
      ...options,
    }
  );

  return {
    ...result,
    /** Convenience accessor: the raw tree array (empty while loading). */
    folderTree: result.data?.tree ?? [],
  };
}
