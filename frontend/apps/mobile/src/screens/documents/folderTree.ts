/**
 * Shared folder-tree helpers for the document move/organization sheets.
 *
 * Single source of truth for the depth-first flatten used by both
 * `MoveDocumentSheet` (whole tree) and `MoveFolderSheet` (tree minus the moving
 * subtree). Previously each sheet carried its own near-identical flatten —
 * `MoveFolderSheet`'s own test had to assert the two stayed in lock-step
 * (GH #1589 finding 2). `flattenTree` now delegates to `flattenTreeExcluding`,
 * so there is exactly one traversal to maintain.
 */

import type { ApiFolderTreeNode } from '../../hooks/useFolderTree';

/** Shape of a depth-first flattened folder entry. */
export interface FlatFolder {
  id: string;
  name: string;
  depth: number;
}

/**
 * Depth-first flatten of the folder tree into a scrollable list, skipping any
 * node in `excludeIds` (and, implicitly, its subtree — children of a skipped
 * node are never visited). Each entry carries a `depth` for indentation.
 */
export function flattenTreeExcluding(
  nodes: ApiFolderTreeNode[],
  excludeIds: ReadonlySet<string>,
  depth = 0
): FlatFolder[] {
  const result: FlatFolder[] = [];
  for (const node of nodes) {
    if (excludeIds.has(node.id)) continue;
    result.push({ id: node.id, name: node.name, depth });
    result.push(...flattenTreeExcluding(node.children ?? [], excludeIds, depth + 1));
  }
  return result;
}

/** Shared empty exclusion set so `flattenTree` allocates nothing per call. */
const NO_EXCLUSIONS: ReadonlySet<string> = new Set<string>();

/**
 * Depth-first flatten of the whole folder tree — `flattenTreeExcluding` with an
 * empty exclusion set. Kept as a named helper for the common "no exclusions"
 * call site.
 */
export function flattenTree(nodes: ApiFolderTreeNode[], depth = 0): FlatFolder[] {
  return flattenTreeExcluding(nodes, NO_EXCLUSIONS, depth);
}

/**
 * Collect `targetId` and all of its descendant ids — the subtree that must be
 * excluded when moving a folder, since a folder cannot become its own parent or
 * a child of one of its descendants.
 */
export function collectDescendantIds(nodes: ApiFolderTreeNode[], targetId: string): Set<string> {
  const ids = new Set<string>();

  function walk(current: ApiFolderTreeNode) {
    ids.add(current.id);
    for (const child of current.children ?? []) {
      walk(child);
    }
  }

  function findAndWalk(nodes: ApiFolderTreeNode[]): boolean {
    for (const node of nodes) {
      if (node.id === targetId) {
        walk(node);
        return true;
      }
      if (findAndWalk(node.children ?? [])) return true;
    }
    return false;
  }

  findAndWalk(nodes);
  return ids;
}
