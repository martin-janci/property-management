/// <reference types="vitest/globals" />
/**
 * Unit tests for buildFolderCrumbs — the pure recursive tree-walk that builds
 * a breadcrumb trail from a FolderTreeNode[] for a given folderId.
 */

import type { FolderTreeNode } from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import { buildFolderCrumbs } from './MoveFolderDialog';

// ─── helpers ────────────────────────────────────────────────────────────────

function node(id: string, name: string, children: FolderTreeNode[] = []): FolderTreeNode {
  return { folder: { id, name } as FolderTreeNode['folder'], children };
}

// ─── tree fixture ────────────────────────────────────────────────────────────
//
// root
// ├── a (id: "a")
// │   └── a1 (id: "a1")
// │       └── a1x (id: "a1x")
// └── b (id: "b")
//     └── b1 (id: "b1")

const TREE: FolderTreeNode[] = [
  node('a', 'Rok 2025', [node('a1', 'Faktúry', [node('a1x', 'Q1')])]),
  node('b', 'Zápisnice', [node('b1', 'Schôdze (AGM)')]),
];

// ─── tests ───────────────────────────────────────────────────────────────────

describe('buildFolderCrumbs', () => {
  it('returns [] when folderId is null (root selected)', () => {
    expect(buildFolderCrumbs(TREE, null)).toEqual([]);
  });

  it('returns [] when folderId is not found in tree', () => {
    expect(buildFolderCrumbs(TREE, 'does-not-exist')).toEqual([]);
  });

  it('returns single-item path for a top-level folder', () => {
    expect(buildFolderCrumbs(TREE, 'a')).toEqual([{ id: 'a', name: 'Rok 2025' }]);
    expect(buildFolderCrumbs(TREE, 'b')).toEqual([{ id: 'b', name: 'Zápisnice' }]);
  });

  it('returns two-item path for a second-level folder', () => {
    expect(buildFolderCrumbs(TREE, 'a1')).toEqual([
      { id: 'a', name: 'Rok 2025' },
      { id: 'a1', name: 'Faktúry' },
    ]);
  });

  it('returns three-item path for a third-level folder', () => {
    expect(buildFolderCrumbs(TREE, 'a1x')).toEqual([
      { id: 'a', name: 'Rok 2025' },
      { id: 'a1', name: 'Faktúry' },
      { id: 'a1x', name: 'Q1' },
    ]);
  });

  it('returns correct path for a leaf in second branch', () => {
    expect(buildFolderCrumbs(TREE, 'b1')).toEqual([
      { id: 'b', name: 'Zápisnice' },
      { id: 'b1', name: 'Schôdze (AGM)' },
    ]);
  });

  it('returns [] for an empty tree regardless of folderId', () => {
    expect(buildFolderCrumbs([], 'a')).toEqual([]);
  });
});
