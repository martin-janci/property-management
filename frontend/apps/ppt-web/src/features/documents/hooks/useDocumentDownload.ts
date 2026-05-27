/**
 * useDocumentDownload — shared hook for document download via presigned URL + blob.
 *
 * Calls getDownloadUrl imperatively on click so no request is fired on mount
 * (avoids the eager-fetch problem when rendered in a list of N documents).
 * After obtaining the presigned URL, we fetch as a blob and use createObjectURL
 * so that anchor.download is honoured even for cross-origin S3 presigned URLs
 * (browsers silently ignore anchor.download for cross-origin navigations).
 *
 * Note: getDownloadUrl uses the same fetchApi transport as every other function
 * in @ppt/api-client (listDocuments, moveDocument, getPreviewUrl, etc.) — auth
 * handling is consistent across the entire api-client package.
 *
 * The in-flight guard uses a ref so the `download` callback is stable across
 * renders — no recreation on every isDownloading state change.
 */

import { getDownloadUrl } from '@ppt/api-client';
import { useCallback, useRef, useState } from 'react';
import { useToast } from '../../../components/Toast';

export function useDocumentDownload(documentId: string, fileName: string) {
  const [isDownloading, setIsDownloading] = useState(false);
  const isDownloadingRef = useRef(false);
  const { showToast } = useToast();

  const download = useCallback(async () => {
    if (isDownloadingRef.current) return;
    isDownloadingRef.current = true;
    setIsDownloading(true);
    try {
      const { url } = await getDownloadUrl(documentId);
      const res = await fetch(url);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const blob = await res.blob();
      const blobUrl = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = blobUrl;
      anchor.download = fileName;
      document.body.appendChild(anchor);
      anchor.click();
      document.body.removeChild(anchor);
      URL.revokeObjectURL(blobUrl);
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Stiahnutie zlyhalo',
        message: err instanceof Error ? err.message : 'Dokument sa nepodarilo stiahnuť.',
      });
    } finally {
      isDownloadingRef.current = false;
      setIsDownloading(false);
    }
  }, [documentId, fileName, showToast]);

  return { download, isDownloading };
}
