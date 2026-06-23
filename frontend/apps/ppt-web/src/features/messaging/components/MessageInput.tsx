/**
 * MessageInput Component
 *
 * Input field for composing and sending messages with attachment support.
 */

import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Message, MessageAttachment, PendingAttachment } from '../types';
import { AttachmentPreview } from './AttachmentPreview';

interface MessageInputProps {
  onSendMessage: (content: string, attachments?: PendingAttachment[], replyToId?: string) => void;
  isSubmitting?: boolean;
  disabled?: boolean;
  placeholder?: string;
  replyTo?: Message | null;
  onCancelReply?: () => void;
}

// Maximum attachment size: 25 MiB. Mirrors the backend `MAX_ATTACHMENT_SIZE_BYTES`
// (UC-05.9) so the client rejects oversize files before requesting an upload URL.
export const MAX_ATTACHMENT_SIZE_BYTES = 25 * 1024 * 1024;
// Allowed file types — kept in sync with the backend storage allow-list
// (`ALLOWED_MIME_TYPES` in integrations/storage.rs).
const ALLOWED_TYPES = [
  'image/jpeg',
  'image/png',
  'image/gif',
  'image/webp',
  'application/pdf',
  'application/msword',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  'application/vnd.ms-excel',
  'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
  'text/plain',
  'text/csv',
];

export function MessageInput({
  onSendMessage,
  isSubmitting,
  disabled,
  placeholder,
  replyTo,
  onCancelReply,
}: MessageInputProps) {
  const { t } = useTranslation();
  const [content, setContent] = useState('');
  const [attachments, setAttachments] = useState<PendingAttachment[]>([]);
  const [attachmentError, setAttachmentError] = useState<string | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  // Mirror the current pending attachments so the unmount cleanup can revoke
  // their blob URLs without re-subscribing the effect on every change.
  const attachmentsRef = useRef<PendingAttachment[]>([]);
  attachmentsRef.current = attachments;

  // Revoke any outstanding image preview blob URLs on unmount to avoid leaks.
  useEffect(() => {
    return () => {
      for (const attachment of attachmentsRef.current) {
        if (attachment.previewUrl) {
          URL.revokeObjectURL(attachment.previewUrl);
        }
      }
    };
  }, []);

  // Derive display-only metadata for the composer preview from the pending files.
  const previewAttachments: MessageAttachment[] = attachments.map((a) => ({
    id: a.id,
    name: a.file.name,
    type: a.file.type,
    size: a.file.size,
    url: a.previewUrl,
  }));

  const handleContentChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setContent(e.target.value);
    // Auto-resize textarea
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 120)}px`;
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if ((content.trim() || attachments.length > 0) && !isSubmitting && !disabled) {
      onSendMessage(content.trim(), attachments.length > 0 ? attachments : undefined, replyTo?.id);
      // The handed-off File objects keep the bytes; the local preview blob URLs
      // are no longer needed once the composer clears.
      for (const attachment of attachments) {
        if (attachment.previewUrl) {
          URL.revokeObjectURL(attachment.previewUrl);
        }
      }
      setContent('');
      setAttachments([]);
      setAttachmentError(null);
      if (textareaRef.current) {
        textareaRef.current.style.height = 'auto';
      }
      if (onCancelReply) {
        onCancelReply();
      }
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
    }
  };

  const handleAttachmentClick = () => {
    fileInputRef.current?.click();
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (!files || files.length === 0) return;

    setAttachmentError(null);
    const newAttachments: PendingAttachment[] = [];

    for (let i = 0; i < files.length; i++) {
      const file = files[i];

      // Check file size
      if (file.size > MAX_ATTACHMENT_SIZE_BYTES) {
        setAttachmentError(t('messaging.errors.fileTooLarge', { name: file.name }));
        continue;
      }

      // Check file type
      if (!ALLOWED_TYPES.includes(file.type)) {
        setAttachmentError(t('messaging.errors.invalidFileType', { name: file.name }));
        continue;
      }

      // Hold on to the real File so its bytes can be uploaded on send. Only
      // image files get a blob preview URL (used for the composer thumbnail).
      newAttachments.push({
        id: `pending-${Date.now()}-${i}`,
        file,
        previewUrl: file.type.startsWith('image/') ? URL.createObjectURL(file) : '',
      });
    }

    setAttachments((prev) => [...prev, ...newAttachments]);

    // Reset file input
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const handleRemoveAttachment = (attachmentId: string) => {
    setAttachments((prev) => {
      const attachment = prev.find((a) => a.id === attachmentId);
      if (attachment?.previewUrl) {
        URL.revokeObjectURL(attachment.previewUrl);
      }
      return prev.filter((a) => a.id !== attachmentId);
    });
  };

  const canSend = (content.trim() || attachments.length > 0) && !isSubmitting && !disabled;

  return (
    <div className="bg-white border-t">
      {/* Reply preview */}
      {replyTo && (
        <div className="px-4 pt-2 flex items-start gap-2 bg-gray-50 border-b">
          <div className="flex-1 min-w-0 py-2">
            <div className="flex items-center gap-2 text-xs text-gray-500">
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
                  d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6"
                />
              </svg>
              <span>{t('messaging.replyingTo', { name: replyTo.senderName })}</span>
            </div>
            <p className="text-sm text-gray-600 truncate mt-1">{replyTo.content}</p>
          </div>
          <button
            type="button"
            onClick={onCancelReply}
            className="p-1 text-gray-400 hover:text-gray-600 rounded"
            aria-label={t('common.cancel')}
          >
            <svg
              className="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
      )}

      {/* Attachment preview */}
      {attachments.length > 0 && (
        <div className="px-4 pt-3">
          <AttachmentPreview
            attachments={previewAttachments}
            onRemove={handleRemoveAttachment}
            isEditable
          />
        </div>
      )}

      {/* Attachment error */}
      {attachmentError && (
        <div className="px-4 pt-2">
          <p className="text-sm text-red-600">{attachmentError}</p>
        </div>
      )}

      {/* Input form */}
      <form onSubmit={handleSubmit} className="flex items-end gap-2 p-4">
        {/* Attachment button */}
        <button
          type="button"
          onClick={handleAttachmentClick}
          disabled={disabled || isSubmitting}
          className="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-full disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex-shrink-0"
          aria-label={t('messaging.attachFile')}
        >
          <svg
            className="w-5 h-5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15.172 7l-6.586 6.586a2 2 0 102.828 2.828l6.414-6.586a4 4 0 00-5.656-5.656l-6.415 6.585a6 6 0 108.486 8.486L20.5 13"
            />
          </svg>
        </button>

        {/* Hidden file input */}
        <input
          ref={fileInputRef}
          type="file"
          multiple
          accept={ALLOWED_TYPES.join(',')}
          onChange={handleFileChange}
          className="hidden"
        />

        {/* Text input */}
        <div className="flex-1 relative">
          <textarea
            ref={textareaRef}
            value={content}
            onChange={handleContentChange}
            onKeyDown={handleKeyDown}
            placeholder={placeholder || t('messaging.typeMessage')}
            disabled={disabled || isSubmitting}
            rows={1}
            className="w-full px-4 py-2 border border-gray-300 rounded-2xl resize-none focus:ring-blue-500 focus:border-blue-500 disabled:bg-gray-100 disabled:cursor-not-allowed"
            style={{ minHeight: '40px', maxHeight: '120px' }}
          />
        </div>

        {/* Send button */}
        <button
          type="submit"
          disabled={!canSend}
          className="p-2 bg-blue-600 text-white rounded-full hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex-shrink-0"
          aria-label={t('messaging.send')}
        >
          {isSubmitting ? (
            <svg
              className="w-5 h-5 animate-spin"
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
              className="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
              />
            </svg>
          )}
        </button>
      </form>
    </div>
  );
}
