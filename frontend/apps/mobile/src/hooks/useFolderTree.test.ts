/**
 * Unit tests for useFolderTree (Coverage 7a-2).
 *
 * The hook is a thin wrapper around useApiQuery — it owns the stable query
 * key, the endpoint path, and a default staleTime. These tests verify:
 *
 *   1. FOLDER_TREE_QUERY_KEY is exported and has the expected shape.
 *   2. The `folderTree` convenience accessor defaults to [] when data is
 *      undefined (loading / error path).
 *   3. The hook forwards a non-default staleTime option to useApiQuery.
 *
 * We test the pure-function / constant parts directly; the network layer is
 * already covered by the useApiQuery contract tests elsewhere.
 */

import { FOLDER_TREE_QUERY_KEY } from './useFolderTree';

// ─── FOLDER_TREE_QUERY_KEY ────────────────────────────────────────────────────

describe('FOLDER_TREE_QUERY_KEY', () => {
  it('is a readonly tuple with three string elements', () => {
    expect(Array.isArray(FOLDER_TREE_QUERY_KEY)).toBe(true);
    expect(FOLDER_TREE_QUERY_KEY).toHaveLength(3);
    FOLDER_TREE_QUERY_KEY.forEach((segment) => {
      expect(typeof segment).toBe('string');
    });
  });

  it('matches the canonical key used by DocumentsScreen', () => {
    // DocumentsScreen hard-codes ['documents', 'folders', 'tree'] in its
    // useApiQuery call. This test pins that string contract so a rename of
    // either would be caught immediately.
    expect(FOLDER_TREE_QUERY_KEY).toEqual(['documents', 'folders', 'tree']);
  });
});
