/// <reference types="vitest/globals" />
/**
 * Regression test for dx-documentsbrowse-folder-preselect.
 *
 * DocumentsBrowse previously hard-coded `currentFolderId={null}` because
 * DocumentSummary did not carry `folder_id`. Now that the field is threaded
 * through, MoveFolderDialog must open with the document's current folder
 * pre-selected (destination label + selected tree row), so the "Move" button
 * stays disabled until the user actually changes the target.
 */

import type { FolderTreeNode } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock the folder-tree query so the dialog renders a deterministic tree.
const useFolderTreeMock = vi.fn();
vi.mock('@ppt/api-client', () => ({
  useFolderTree: (buildingId?: string) => useFolderTreeMock(buildingId),
}));

import { MoveFolderDialog } from './MoveFolderDialog';

function node(id: string, name: string, children: FolderTreeNode[] = []): FolderTreeNode {
  return {
    folder: { id, name } as FolderTreeNode['folder'],
    children,
    document_count: 0,
  };
}

const TREE: FolderTreeNode[] = [
  node('a', 'Rok 2025', [node('a1', 'Faktúry')]),
  node('b', 'Zápisnice'),
];

describe('MoveFolderDialog pre-select', () => {
  beforeEach(() => {
    useFolderTreeMock.mockReset();
    useFolderTreeMock.mockReturnValue({ data: { tree: TREE }, isLoading: false, error: null });
  });

  it('pre-selects the document current folder and disables Move until changed', () => {
    render(
      <MoveFolderDialog
        documentTitles={['Faktura.pdf']}
        currentFolderId="a1"
        onConfirm={() => {}}
        onCancel={() => {}}
      />
    );

    // The folder name button for the current folder is marked pressed
    // (aria-pressed=true) — i.e. the dialog opened pre-selected on it.
    const selected = screen.getByRole('button', { name: 'Faktúry', pressed: true });
    expect(selected).toBeInTheDocument();

    // Destination label reflects the pre-selected folder, not "root".
    expect(screen.queryByText('Koreň (bez priečinka)')).not.toBeInTheDocument();

    // No net change yet → confirm button disabled.
    const confirm = screen.getByRole('button', { name: 'Presunúť sem' });
    expect(confirm).toBeDisabled();
  });

  it('falls back to root selection when the document has no folder', () => {
    render(
      <MoveFolderDialog
        documentTitles={['Faktura.pdf']}
        currentFolderId={null}
        onConfirm={() => {}}
        onCancel={() => {}}
      />
    );

    const rootOption = screen.getByRole('button', {
      name: /Všetky dokumenty \(bez priečinka\)/,
    });
    expect(rootOption).toHaveAttribute('aria-pressed', 'true');
  });
});
