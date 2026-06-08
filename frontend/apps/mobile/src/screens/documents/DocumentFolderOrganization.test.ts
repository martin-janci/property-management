/**
 * Unit tests for the document folder-organisation slice (Coverage 7a-2).
 *
 * Covers the three pure-function slices that drive list/create/move folder
 * behaviour in the React Native app, all of which are testable without a
 * render tree:
 *
 *   1. subfoldersAtPath  — navigates the folder tree to return the children
 *      at a given breadcrumb path (drives the drill-down list view).
 *
 *   2. folderNodeToDocument — maps a server-side ApiFolderTreeNode to the
 *      unified Document shape rendered in the list (drives folder rows).
 *
 *   3. flattenTree (MoveDocumentSheet) — depth-first flatten used to render
 *      the indented folder picker in the move-document sheet.
 *
 * NewFolderSheet and MoveDocumentSheet mutate server state via hooks that
 * delegate to fetch; those paths are covered by the useApiMutation contract
 * already tested in other suites. What is load-bearing here — and not tested
 * elsewhere — are the local utility functions above.
 */

import type { ApiFolderTreeNode } from './DocumentsScreen';
import { folderNodeToDocument, subfoldersAtPath } from './DocumentsScreen';
import { flattenTree } from './MoveDocumentSheet';

// ─── Fixtures ────────────────────────────────────────────────────────────────

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

// ─── subfoldersAtPath ─────────────────────────────────────────────────────────

describe('subfoldersAtPath', () => {
  it('returns all root-level folders when the path is empty', () => {
    const result = subfoldersAtPath(SAMPLE_TREE, []);
    expect(result.map((n) => n.id)).toEqual(['f1', 'f2', 'f3']);
  });

  it('returns the direct children of a single-level path', () => {
    const result = subfoldersAtPath(SAMPLE_TREE, [{ id: 'f1', name: 'Contracts' }]);
    expect(result.map((n) => n.id)).toEqual(['f1a']);
  });

  it('drills two levels deep correctly', () => {
    const result = subfoldersAtPath(SAMPLE_TREE, [
      { id: 'f1', name: 'Contracts' },
      { id: 'f1a', name: '2024' },
    ]);
    expect(result.map((n) => n.id)).toEqual(['f1a1']);
  });

  it('returns an empty array for a leaf folder with no children', () => {
    const result = subfoldersAtPath(SAMPLE_TREE, [
      { id: 'f1', name: 'Contracts' },
      { id: 'f1a', name: '2024' },
      { id: 'f1a1', name: 'Signed' },
    ]);
    expect(result).toEqual([]);
  });

  it('returns an empty array for a folder whose children is null', () => {
    // f2 has children: null
    const result = subfoldersAtPath(SAMPLE_TREE, [{ id: 'f2', name: 'Financial' }]);
    expect(result).toEqual([]);
  });

  it('returns an empty array when the first crumb does not match any root node', () => {
    const result = subfoldersAtPath(SAMPLE_TREE, [{ id: 'nonexistent', name: 'Ghost' }]);
    expect(result).toEqual([]);
  });

  it('returns an empty array when an intermediate crumb does not match', () => {
    const result = subfoldersAtPath(SAMPLE_TREE, [
      { id: 'f1', name: 'Contracts' },
      { id: 'ghost', name: 'Ghost' },
    ]);
    expect(result).toEqual([]);
  });

  it('handles an empty tree', () => {
    expect(subfoldersAtPath([], [])).toEqual([]);
    expect(subfoldersAtPath([], [{ id: 'f1', name: 'Contracts' }])).toEqual([]);
  });
});

// ─── folderNodeToDocument ─────────────────────────────────────────────────────

describe('folderNodeToDocument', () => {
  const node: ApiFolderTreeNode = {
    id: 'f1',
    name: 'Contracts',
    parent_id: 'root-folder',
    document_count: 7,
    children: [],
  };

  it('maps id and name from the tree node', () => {
    const doc = folderNodeToDocument(node);
    expect(doc.id).toBe('f1');
    expect(doc.name).toBe('Contracts');
  });

  it('sets type to "folder"', () => {
    expect(folderNodeToDocument(node).type).toBe('folder');
  });

  it('carries the live documentCount', () => {
    expect(folderNodeToDocument(node).documentCount).toBe(7);
  });

  it('maps parent_id to parentId (string)', () => {
    expect(folderNodeToDocument(node).parentId).toBe('root-folder');
  });

  it('maps null parent_id to null parentId (root-level folder)', () => {
    const rootNode: ApiFolderTreeNode = { ...node, parent_id: null };
    expect(folderNodeToDocument(rootNode).parentId).toBeNull();
  });

  it('maps undefined parent_id to null parentId', () => {
    const { parent_id, ...withoutParent } = node;
    expect(folderNodeToDocument(withoutParent as ApiFolderTreeNode).parentId).toBeNull();
  });

  it('produces a zero documentCount when the node has none', () => {
    const zeroNode: ApiFolderTreeNode = { ...node, document_count: 0 };
    expect(folderNodeToDocument(zeroNode).documentCount).toBe(0);
  });

  it('does not carry children — the row shape has no sub-folder list', () => {
    // children in the Document shape is for nested documents, not sub-folders.
    // folderNodeToDocument intentionally leaves it undefined.
    expect(folderNodeToDocument(node).children).toBeUndefined();
  });
});

// ─── flattenTree (MoveDocumentSheet) ─────────────────────────────────────────

describe('flattenTree', () => {
  it('returns an empty array for an empty tree', () => {
    expect(flattenTree([])).toEqual([]);
  });

  it('returns a single entry with depth 0 for a leaf root node', () => {
    const tree: ApiFolderTreeNode[] = [
      { id: 'f1', name: 'Solo', parent_id: null, document_count: 0, children: [] },
    ];
    expect(flattenTree(tree)).toEqual([{ id: 'f1', name: 'Solo', depth: 0 }]);
  });

  it('flattens a two-level tree depth-first (parent before child)', () => {
    const tree: ApiFolderTreeNode[] = [
      {
        id: 'f1',
        name: 'Parent',
        parent_id: null,
        document_count: 0,
        children: [
          { id: 'f1a', name: 'Child', parent_id: 'f1', document_count: 0, children: [] },
        ],
      },
    ];
    expect(flattenTree(tree)).toEqual([
      { id: 'f1', name: 'Parent', depth: 0 },
      { id: 'f1a', name: 'Child', depth: 1 },
    ]);
  });

  it('flattens the full SAMPLE_TREE depth-first in correct order', () => {
    const result = flattenTree(SAMPLE_TREE);
    expect(result.map((f) => f.id)).toEqual(['f1', 'f1a', 'f1a1', 'f2', 'f3', 'f3a']);
  });

  it('assigns correct depth for three-level nesting', () => {
    const result = flattenTree(SAMPLE_TREE);
    const byId = Object.fromEntries(result.map((f) => [f.id, f.depth]));
    expect(byId['f1']).toBe(0); // root
    expect(byId['f1a']).toBe(1); // child of f1
    expect(byId['f1a1']).toBe(2); // grandchild of f1
    expect(byId['f3a']).toBe(1); // child of f3
  });

  it('handles nodes with null children gracefully', () => {
    const tree: ApiFolderTreeNode[] = [
      { id: 'f1', name: 'Root', parent_id: null, document_count: 0, children: null },
    ];
    expect(flattenTree(tree)).toEqual([{ id: 'f1', name: 'Root', depth: 0 }]);
  });

  it('handles nodes with undefined children gracefully', () => {
    const tree: ApiFolderTreeNode[] = [
      { id: 'f1', name: 'Root', parent_id: null, document_count: 0 },
    ];
    expect(flattenTree(tree)).toEqual([{ id: 'f1', name: 'Root', depth: 0 }]);
  });

  it('preserves folder names verbatim', () => {
    const tree: ApiFolderTreeNode[] = [
      { id: 'f1', name: 'Q1/2024 Budget & Forecast', parent_id: null, document_count: 0, children: [] },
    ];
    expect(flattenTree(tree)[0].name).toBe('Q1/2024 Budget & Forecast');
  });

  it('handles multiple root-level folders with no children', () => {
    const tree: ApiFolderTreeNode[] = [
      { id: 'a', name: 'Alpha', parent_id: null, document_count: 0, children: [] },
      { id: 'b', name: 'Beta', parent_id: null, document_count: 0, children: [] },
      { id: 'c', name: 'Gamma', parent_id: null, document_count: 0, children: [] },
    ];
    expect(flattenTree(tree)).toEqual([
      { id: 'a', name: 'Alpha', depth: 0 },
      { id: 'b', name: 'Beta', depth: 0 },
      { id: 'c', name: 'Gamma', depth: 0 },
    ]);
  });

  it('starts at a custom depth when depth argument is provided', () => {
    const tree: ApiFolderTreeNode[] = [
      { id: 'f1', name: 'Root', parent_id: null, document_count: 0, children: [] },
    ];
    expect(flattenTree(tree, 2)).toEqual([{ id: 'f1', name: 'Root', depth: 2 }]);
  });
});
