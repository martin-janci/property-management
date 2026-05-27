/**
 * Shared move-with-feedback hook used by DocumentsBrowse and FolderTreePage.
 * Centralises the mutation call, success/error toast messages, and isPending flag
 * so the two components cannot diverge.
 *
 * useMoveDocument from @ppt/api-client already invalidates documentKeys.lists()
 * and documentKeys.folders() in its onSuccess handler — callers do not need to
 * add explicit queryClient.invalidateQueries calls.
 */

import type { DocumentSummary } from '@ppt/api-client';
import { useMoveDocument } from '@ppt/api-client';
import { useCallback } from 'react';
import { useToast } from '../../../components/Toast';

export function useMoveDocumentWithToast() {
  const moveDocument = useMoveDocument();
  const { showToast } = useToast();

  const executeMoveWithToast = useCallback(
    async (doc: DocumentSummary, folderId: string | null): Promise<boolean> => {
      try {
        await moveDocument.mutateAsync({ documentId: doc.id, folderId });
        showToast({
          type: 'success',
          title: 'Dokument presunutý',
          message: folderId
            ? `„${doc.title}" bol presunutý do priečinka.`
            : `„${doc.title}" bol presunutý do koreňa.`,
        });
        return true;
      } catch (err) {
        showToast({
          type: 'error',
          title: 'Presun zlyhal',
          message: err instanceof Error ? err.message : 'Dokument sa nepodarilo presunúť.',
        });
        return false;
      }
    },
    [moveDocument, showToast]
  );

  return { executeMoveWithToast, isPending: moveDocument.isPending };
}
