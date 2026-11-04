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
import { useTranslation } from 'react-i18next';
import { useToast } from '../../../components/Toast';

export function useMoveDocumentWithToast() {
  const moveDocument = useMoveDocument();
  const { showToast } = useToast();
  const { t } = useTranslation();

  const executeMoveWithToast = useCallback(
    async (doc: DocumentSummary, folderId: string | null): Promise<boolean> => {
      try {
        await moveDocument.mutateAsync({ documentId: doc.id, folderId });
        showToast({
          type: 'success',
          title: t('documents.move.successTitle'),
          message: folderId
            ? t('documents.move.successToFolder', { title: doc.title })
            : t('documents.move.successToRoot', { title: doc.title }),
        });
        return true;
      } catch (err) {
        showToast({
          type: 'error',
          title: t('documents.move.failedTitle'),
          message: err instanceof Error ? err.message : t('documents.move.failedMessage'),
        });
        return false;
      }
    },
    [moveDocument, showToast, t]
  );

  return { executeMoveWithToast, isPending: moveDocument.isPending };
}
