/**
 * MessageBubble Component
 *
 * Displays a single message in a conversation thread with context menu support.
 */

import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Message } from '../types';
import { MessageAttachments } from './MessageAttachments';

interface MessageBubbleProps {
  message: Message;
  isCurrentUser: boolean;
  showAvatar?: boolean;
  showTimestamp?: boolean;
  onReply?: (message: Message) => void;
  onDelete?: (messageId: string) => void;
}

function formatTime(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function formatDate(dateString: string): string {
  const date = new Date(dateString);
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);

  if (date.toDateString() === today.toDateString()) {
    return 'Today';
  }
  if (date.toDateString() === yesterday.toDateString()) {
    return 'Yesterday';
  }
  return date.toLocaleDateString(undefined, {
    weekday: 'long',
    month: 'short',
    day: 'numeric',
  });
}

export function MessageBubble({
  message,
  isCurrentUser,
  showAvatar = true,
  showTimestamp = true,
  onReply,
  onDelete,
}: MessageBubbleProps) {
  const { t } = useTranslation();
  const [showContextMenu, setShowContextMenu] = useState(false);
  const [contextMenuPosition, setContextMenuPosition] = useState({ x: 0, y: 0 });
  const bubbleRef = useRef<HTMLDivElement>(null);

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    const rect = bubbleRef.current?.getBoundingClientRect();
    if (rect) {
      setContextMenuPosition({
        x: e.clientX - rect.left,
        y: e.clientY - rect.top,
      });
    }
    setShowContextMenu(true);
  };

  const handleCloseContextMenu = () => {
    setShowContextMenu(false);
  };

  const handleReply = () => {
    if (onReply) {
      onReply(message);
    }
    setShowContextMenu(false);
  };

  const handleDelete = () => {
    if (onDelete) {
      onDelete(message.id);
    }
    setShowContextMenu(false);
  };

  const handleMenuButtonClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    const rect = e.currentTarget.getBoundingClientRect();
    const bubbleRect = bubbleRef.current?.getBoundingClientRect();
    if (bubbleRect) {
      setContextMenuPosition({
        x: rect.left - bubbleRect.left,
        y: rect.bottom - bubbleRect.top + 4,
      });
    }
    setShowContextMenu(!showContextMenu);
  };

  return (
    <div
      ref={bubbleRef}
      className={`relative flex gap-2 group ${isCurrentUser ? 'flex-row-reverse' : 'flex-row'}`}
      onContextMenu={handleContextMenu}
    >
      {/* Avatar */}
      {showAvatar && !isCurrentUser && (
        <div className="flex-shrink-0">
          {message.senderAvatar ? (
            <img
              src={message.senderAvatar}
              alt={message.senderName}
              className="w-8 h-8 rounded-full object-cover"
            />
          ) : (
            <div className="w-8 h-8 rounded-full bg-gray-300 flex items-center justify-center">
              <span className="text-gray-600 text-xs font-medium">
                {message.senderName.charAt(0).toUpperCase()}
              </span>
            </div>
          )}
        </div>
      )}
      {showAvatar && isCurrentUser && <div className="w-8 flex-shrink-0" />}

      {/* Message Content */}
      <div
        className={`max-w-xs sm:max-w-md lg:max-w-lg ${
          isCurrentUser ? 'items-end' : 'items-start'
        }`}
      >
        {!isCurrentUser && showAvatar && (
          <span className="text-xs text-gray-500 mb-1 block">{message.senderName}</span>
        )}

        {/* Reply reference */}
        {message.replyToContent && (
          <div
            className={`mb-1 px-3 py-1 text-xs rounded-lg border-l-2 ${
              isCurrentUser
                ? 'bg-blue-500/20 border-blue-400 text-blue-100'
                : 'bg-gray-200 border-gray-400 text-gray-600'
            }`}
          >
            <span className="font-medium">{message.replyToSenderName}</span>
            <p className="truncate opacity-80">{message.replyToContent}</p>
          </div>
        )}

        <div className="relative">
          <div
            className={`px-4 py-2 rounded-2xl ${
              isCurrentUser
                ? 'bg-blue-600 text-white rounded-br-md'
                : 'bg-gray-100 text-gray-900 rounded-bl-md'
            }`}
          >
            <p className="text-sm whitespace-pre-wrap break-words">{message.content}</p>

            {/* Attachments (resolved lazily from the API, UC-05.9) */}
            <MessageAttachments
              threadId={message.threadId}
              messageId={message.id}
              isCurrentUser={isCurrentUser}
            />
          </div>

          {/* Context menu button (visible on hover) */}
          <button
            type="button"
            onClick={handleMenuButtonClick}
            className={`absolute top-1 ${
              isCurrentUser ? '-left-8' : '-right-8'
            } p-1 text-gray-400 hover:text-gray-600 rounded opacity-0 group-hover:opacity-100 transition-opacity`}
            aria-label={t('common.more')}
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
                d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z"
              />
            </svg>
          </button>
        </div>

        {showTimestamp && (
          <span
            className={`text-xs text-gray-400 mt-1 block ${
              isCurrentUser ? 'text-right' : 'text-left'
            }`}
          >
            {formatTime(message.createdAt)}
          </span>
        )}
      </div>

      {/* Context Menu */}
      {showContextMenu && (
        <>
          {/* Backdrop */}
          <div
            className="fixed inset-0 z-40"
            onClick={handleCloseContextMenu}
            onKeyDown={(e) => {
              if (e.key === 'Escape') handleCloseContextMenu();
            }}
            role="button"
            tabIndex={0}
            aria-label={t('aria.closeMenu')}
          />

          {/* Menu */}
          <div
            className="absolute z-50 bg-white rounded-lg shadow-lg border border-gray-200 py-1 min-w-[140px]"
            style={{
              left: contextMenuPosition.x,
              top: contextMenuPosition.y,
            }}
          >
            {onReply && (
              <button
                type="button"
                onClick={handleReply}
                className="w-full px-4 py-2 text-left text-sm text-gray-700 hover:bg-gray-100 flex items-center gap-2"
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
                    d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6"
                  />
                </svg>
                {t('messaging.reply')}
              </button>
            )}
            {isCurrentUser && onDelete && (
              <button
                type="button"
                onClick={handleDelete}
                className="w-full px-4 py-2 text-left text-sm text-red-600 hover:bg-red-50 flex items-center gap-2"
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
                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                  />
                </svg>
                {t('common.delete')}
              </button>
            )}
          </div>
        </>
      )}
    </div>
  );
}

interface DateSeparatorProps {
  date: string;
}

export function DateSeparator({ date }: DateSeparatorProps) {
  return (
    <div className="flex items-center justify-center my-4">
      <div className="flex-1 border-t border-gray-200" />
      <span className="px-3 text-xs text-gray-500 bg-gray-50">{formatDate(date)}</span>
      <div className="flex-1 border-t border-gray-200" />
    </div>
  );
}
