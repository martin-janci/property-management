/**
 * Unit tests for useFolderTree (Coverage 7a-2).
 *
 * The hook is a thin wrapper around useApiQuery — it owns the stable query
 * key, the endpoint path, and a default staleTime. These tests verify:
 *
 *   1. FOLDER_TREE_QUERY_KEY is exported and has the expected shape.
 *   2. The hook actually *queries* under FOLDER_TREE_QUERY_KEY with the tree
 *      endpoint and default staleTime — i.e. the constant is load-bearing, not
 *      just a value that happens to match a literal (#1590).
 *   3. Caller options are forwarded to useApiQuery (staleTime override etc.).
 *   4. The `folderTree` convenience accessor defaults to [] when data is
 *      undefined (loading / error path) and unwraps `data.tree` otherwise.
 *
 * useApiQuery is mocked, so `useFolderTree()` invokes no real React hooks and
 * can be called directly — we deliberately avoid `@testing-library/react-native`
 * / `renderHook`, which currently breaks every importing test via the
 * `react-test-renderer` 19.2.0-vs-19.2.6 peer-dep mismatch (PR #918 finding #7).
 */

import { useApiQuery } from './useApi';
import { FOLDER_TREE_QUERY_KEY, useFolderTree } from './useFolderTree';

jest.mock('./useApi', () => ({
  useApiQuery: jest.fn(() => ({ data: undefined, isLoading: true })),
}));

const mockedUseApiQuery = useApiQuery as jest.Mock;
const TREE_ENDPOINT = '/api/v1/documents/folders/tree';

beforeEach(() => {
  mockedUseApiQuery.mockClear();
  mockedUseApiQuery.mockReturnValue({ data: undefined, isLoading: true });
});

// ─── FOLDER_TREE_QUERY_KEY ────────────────────────────────────────────────────

describe('FOLDER_TREE_QUERY_KEY', () => {
  it('is a readonly tuple with three string elements', () => {
    expect(Array.isArray(FOLDER_TREE_QUERY_KEY)).toBe(true);
    expect(FOLDER_TREE_QUERY_KEY).toHaveLength(3);
    FOLDER_TREE_QUERY_KEY.forEach((segment) => {
      expect(typeof segment).toBe('string');
    });
  });

  it('has the canonical value documents/folders/tree', () => {
    // FOLDER_TREE_QUERY_KEY is the single source of truth: useFolderTree()
    // queries under it (asserted below) and the invalidation sites
    // (MoveDocumentSheet, NewFolderSheet) invalidate it, so a rename of the
    // constant propagates to every consumer automatically.
    expect(FOLDER_TREE_QUERY_KEY).toEqual(['documents', 'folders', 'tree']);
  });
});

// ─── Wiring: the hook queries under the constant ──────────────────────────────

describe('useFolderTree wiring', () => {
  it('queries useApiQuery under FOLDER_TREE_QUERY_KEY with the tree endpoint + default staleTime', () => {
    // This is the load-bearing assertion the constant pin-test alone cannot
    // make: it would still pass if someone changed the hook's *actual* query
    // key while leaving the exported constant alone.
    useFolderTree();

    expect(mockedUseApiQuery).toHaveBeenCalledTimes(1);
    expect(mockedUseApiQuery).toHaveBeenCalledWith(
      FOLDER_TREE_QUERY_KEY,
      TREE_ENDPOINT,
      expect.objectContaining({ staleTime: 60_000 })
    );
  });

  it('forwards caller options (e.g. enabled) to useApiQuery without dropping the default staleTime', () => {
    useFolderTree({ enabled: false });

    expect(mockedUseApiQuery).toHaveBeenCalledWith(
      FOLDER_TREE_QUERY_KEY,
      TREE_ENDPOINT,
      expect.objectContaining({ staleTime: 60_000, enabled: false })
    );
  });

  it('lets a caller override the default staleTime', () => {
    useFolderTree({ staleTime: 0 });

    expect(mockedUseApiQuery).toHaveBeenCalledWith(
      FOLDER_TREE_QUERY_KEY,
      TREE_ENDPOINT,
      expect.objectContaining({ staleTime: 0 })
    );
  });
});

// ─── folderTree accessor ──────────────────────────────────────────────────────

describe('useFolderTree folderTree accessor', () => {
  it('defaults to [] while data is undefined (loading / error)', () => {
    const { folderTree } = useFolderTree();
    expect(folderTree).toEqual([]);
  });

  it('unwraps data.tree when the query has resolved', () => {
    mockedUseApiQuery.mockReturnValueOnce({
      data: { tree: [{ id: '1', name: 'Root', document_count: 0 }] },
      isLoading: false,
    });

    const { folderTree } = useFolderTree();
    expect(folderTree).toHaveLength(1);
    expect(folderTree[0]).toMatchObject({ id: '1', name: 'Root' });
  });
});
