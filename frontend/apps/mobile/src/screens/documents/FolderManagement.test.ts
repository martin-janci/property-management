/**
 * Unit tests for the folder-management slice (feat-folder-organization-document-mobile).
 *
 * Covers the two pure-function utilities that drive the rename / move-folder
 * UX in the React Native app:
 *
 *   1. collectDescendantIds (MoveFolderSheet) — walks the tree to collect all
 *      ids in the subtree rooted at a given folder. Used to exclude the folder
 *      being moved from its own destination list (prevents circular refs).
 *
 *   2. flattenTreeExcluding (MoveFolderSheet) — depth-first flatten that skips
 *      any nodes whose ids are in the exclusion set. Drives the flat folder
 *      picker in the move-folder bottom-sheet.
 *
 * RenameFolderSheet and MoveFolderSheet mutate server state via hooks that
 * delegate to fetch; those paths are covered by the useApiMutation contract
 * tested elsewhere. Only the local pure-function utilities are tested here.
 */

import type { ApiFolderTreeNode } from './DocumentsScreen';
import { collectDescendantIds, flattenTreeExcluding } from './MoveFolderSheet';

// ─── Fixtures ─────────────────────────────────────────────────────────────────

/**
 * Sample tree:
 *
 *   Contracts (f1)
 *     └── 2024 (f1a)
 *           └── Signed (f1a1)
 *   Financial (f2)
 *   Maintenance (f3)
 *     └── Reports (f3a)
 */
const SAMPLE_TREE: ApiFolderTreeNode[] = [
  {
    id: 'f1',
    name: 'Contracts',
    parent_id: null,
    document_count: 3,
    children: [
      {
        id: 'f1a',
        name: '2024',
        parent_id: 'f1',
        document_count: 1,
        children: [
          {
            id: 'f1a1',
            name: 'Signed',
            parent_id: 'f1a',
            document_count: 5,
            children: [],
          },
        ],
      },
    ],
  },
  {
    id: 'f2',
    name: 'Financial',
    parent_id: null,
    document_count: 0,
    children: null,
  },
  {
    id: 'f3',
    name: 'Maintenance',
    parent_id: null,
    document_count: 2,
    children: [
      {
        id: 'f3a',
        name: 'Reports',
        parent_id: 'f3',
        document_count: 4,
        children: [],
      },
    ],
  },
];

// ─── collectDescendantIds ──────────────────────────────────────────────────────

describe('collectDescendantIds', () => {
  it('returns only the root-level folder id when it has no children', () => {
    const ids = collectDescendantIds(SAMPLE_TREE, 'f2');
    expect([...ids]).toEqual(['f2']);
  });

  it('collects the target and all its descendants', () => {
    const ids = collectDescendantIds(SAMPLE_TREE, 'f1');
    expect(ids).toEqual(new Set(['f1', 'f1a', 'f1a1']));
  });

  it('collects only a single subtree branch', () => {
    const ids = collectDescendantIds(SAMPLE_TREE, 'f3');
    expect(ids).toEqual(new Set(['f3', 'f3a']));
  });

  it('collects a mid-level node and its descendants', () => {
    const ids = collectDescendantIds(SAMPLE_TREE, 'f1a');
    expect(ids).toEqual(new Set(['f1a', 'f1a1']));
  });

  it('collects a leaf node with an empty children array', () => {
    const ids = collectDescendantIds(SAMPLE_TREE, 'f1a1');
    expect(ids).toEqual(new Set(['f1a1']));
  });

  it('returns an empty set for a non-existent id', () => {
    const ids = collectDescendantIds(SAMPLE_TREE, 'ghost');
    expect(ids.size).toBe(0);
  });

  it('returns an empty set for an empty tree', () => {
    const ids = collectDescendantIds([], 'f1');
    expect(ids.size).toBe(0);
  });

  it('handles nodes with null children gracefully', () => {
    // f2 has children: null
    const ids = collectDescendantIds(SAMPLE_TREE, 'f2');
    expect(ids).toEqual(new Set(['f2']));
  });
});

// ─── flattenTreeExcluding ──────────────────────────────────────────────────────

describe('flattenTreeExcluding', () => {
  it('returns the full flat list when exclude set is empty', () => {
    const result = flattenTreeExcluding(SAMPLE_TREE, new Set());
    expect(result.map((f) => f.id)).toEqual(['f1', 'f1a', 'f1a1', 'f2', 'f3', 'f3a']);
  });

  it('excludes a root-level leaf folder', () => {
    const result = flattenTreeExcluding(SAMPLE_TREE, new Set(['f2']));
    expect(result.map((f) => f.id)).toEqual(['f1', 'f1a', 'f1a1', 'f3', 'f3a']);
  });

  it('excludes a root-level folder and its entire subtree', () => {
    const excludeIds = collectDescendantIds(SAMPLE_TREE, 'f1');
    const result = flattenTreeExcluding(SAMPLE_TREE, excludeIds);
    expect(result.map((f) => f.id)).toEqual(['f2', 'f3', 'f3a']);
  });

  it('excludes a nested folder and its descendants only', () => {
    const excludeIds = collectDescendantIds(SAMPLE_TREE, 'f1a');
    const result = flattenTreeExcluding(SAMPLE_TREE, excludeIds);
    expect(result.map((f) => f.id)).toEqual(['f1', 'f2', 'f3', 'f3a']);
  });

  it('returns an empty array when all folders are excluded', () => {
    const excludeIds = new Set(['f1', 'f1a', 'f1a1', 'f2', 'f3', 'f3a']);
    const result = flattenTreeExcluding(SAMPLE_TREE, excludeIds);
    expect(result).toEqual([]);
  });

  it('assigns correct depth values to the remaining folders', () => {
    // Exclude f1 subtree — remaining: f2 (depth 0), f3 (depth 0), f3a (depth 1)
    const excludeIds = collectDescendantIds(SAMPLE_TREE, 'f1');
    const result = flattenTreeExcluding(SAMPLE_TREE, excludeIds);
    const depthOf = (id: string) => result.find((f) => f.id === id)?.depth;
    expect(depthOf('f2')).toBe(0);
    expect(depthOf('f3')).toBe(0);
    expect(depthOf('f3a')).toBe(1);
  });

  it('preserves folder names verbatim', () => {
    const result = flattenTreeExcluding(SAMPLE_TREE, new Set());
    const f3a = result.find((f) => f.id === 'f3a');
    expect(f3a?.name).toBe('Reports');
  });

  it('returns an empty array for an empty tree', () => {
    expect(flattenTreeExcluding([], new Set(['f1']))).toEqual([]);
  });

  it('starts at a custom depth when depth argument is provided', () => {
    const tree: ApiFolderTreeNode[] = [
      { id: 'f1', name: 'Root', parent_id: null, document_count: 0, children: [] },
    ];
    const result = flattenTreeExcluding(tree, new Set(), 2);
    expect(result).toEqual([{ id: 'f1', name: 'Root', depth: 2 }]);
  });

  it('matches the full-tree flatten output of flattenTree when no exclusions', () => {
    // Ensure consistency with the existing flattenTree utility from MoveDocumentSheet
    const { flattenTree } = jest.requireActual(
      './MoveDocumentSheet'
    ) as typeof import('./MoveDocumentSheet');
    const withoutExclusions = flattenTreeExcluding(SAMPLE_TREE, new Set());
    const full = flattenTree(SAMPLE_TREE);
    expect(withoutExclusions).toEqual(full);
  });
});
