/**
 * MessageAttachments Component
 *
 * Renders the attachments linked to a received/persisted message (UC-05.9).
 *
 * The message-list endpoint does not embed attachments, so each message
 * resolves its own attachment list lazily via `useMessageAttachments`. Files
 * are shown as compact download chips; clicking one resolves a short-lived
 * presigned download URL and triggers the download.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAttachmentDownload, useMessageAttachments } from '../hooks/useMessaging';

interface MessageAttachmentsProps {
  threadId: string;
  messageId: string;
  /** Render against a dark (current-user) bubble background. */
  isCurrentUser?: boolean;
}

function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${Number.parseFloat((bytes / k ** i).toFixed(1))} ${sizes[i]}`;
}

function getFileIcon(type: string) {
  const color = type.startsWith('image/')
    ? 'text-blue-400'
    : type === 'application/pdf'
      ? 'text-red-400'
      : 'text-gray-400';
  return (
    <svg
      className={`w-4 h-4 flex-shrink-0 ${color}`}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
      />
    </svg>
  );
}

export function MessageAttachments({
  threadId,
  messageId,
  isCurrentUser = false,
}: MessageAttachmentsProps) {
  const { t } = useTranslation();
  const { data } = useMessageAttachments(threadId, messageId);
  const resolveDownloadUrl = useAttachmentDownload();
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const attachments = data?.attachments ?? [];
  if (attachments.length === 0) {
    return null;
  }

  const handleDownload = async (attachmentId: string) => {
    setError(null);
    setDownloadingId(attachmentId);
    try {
      const { url } = await resolveDownloadUrl(threadId, attachmentId);
      const link = document.createElement('a');
      link.href = url;
      link.rel = 'noopener noreferrer';
      link.target = '_blank';
      document.body.appendChild(link);
      link.click();
      link.remove();
    } catch {
      setError(t('messaging.errors.downloadFailed', 'Failed to download attachment'));
    } finally {
      setDownloadingId(null);
    }
  };

  return (
    <div className="mt-2 space-y-1">
      {attachments.map((attachment) => (
        <button
          key={attachment.id}
          type="button"
          onClick={() => handleDownload(attachment.id)}
          disabled={downloadingId === attachment.id}
          className={`flex w-full items-center gap-2 p-2 rounded text-sm text-left transition-colors disabled:opacity-60 ${
            isCurrentUser ? 'bg-white/10 hover:bg-white/20' : 'bg-black/5 hover:bg-black/10'
          }`}
          title={t('documents.download')}
        >
          {getFileIcon(attachment.fileType)}
          <span className="flex-1 truncate">{attachment.fileName}</span>
          <span className="text-xs opacity-70">{formatFileSize(attachment.fileSize)}</span>
          {downloadingId === attachment.id ? (
            <svg
              className="w-4 h-4 flex-shrink-0 animate-spin"
              fill="none"
              viewBox="0 0 24 24"
              aria-hidden="true"
            >
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              />
            </svg>
          ) : (
            <svg
              className="w-4 h-4 flex-shrink-0"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
              />
            </svg>
          )}
        </button>
      ))}
      {error && <p className="text-xs text-red-300">{error}</p>}
    </div>
  );
}
