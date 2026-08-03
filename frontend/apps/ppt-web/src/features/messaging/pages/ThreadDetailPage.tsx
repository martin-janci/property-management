/**
 * ThreadDetailPage - View and send messages in a conversation thread.
 *
 * Displays the full message history for a thread and allows
 * sending new messages with reply and delete functionality.
 */

import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { DateSeparator, MessageBubble } from '../components/MessageBubble';
import { MessageInput } from '../components/MessageInput';
import type { Message, PendingAttachment, SendMessageRequest, ThreadWithMessages } from '../types';

interface ThreadDetailPageProps {
  thread: ThreadWithMessages;
  currentUserId: string;
  isLoading?: boolean;
  isSending?: boolean;
  onSendMessage: (data: SendMessageRequest) => void;
  onDeleteMessage?: (messageId: string) => void;
  onMarkAsRead: () => void;
  onBack: () => void;
}

function getParticipantNames(thread: ThreadWithMessages, excludeUserId: string): string {
  const otherParticipants = thread.participants.filter((p) => p.userId !== excludeUserId);
  if (otherParticipants.length === 0) return 'Unknown';
  if (otherParticipants.length === 1) return otherParticipants[0].userName;
  if (otherParticipants.length === 2) {
    return `${otherParticipants[0].userName} and ${otherParticipants[1].userName}`;
  }
  return `${otherParticipants[0].userName} and ${otherParticipants.length - 1} others`;
}

function shouldShowDateSeparator(currentDate: string, previousDate?: string): boolean {
  if (!previousDate) return true;
  const current = new Date(currentDate).toDateString();
  const previous = new Date(previousDate).toDateString();
  return current !== previous;
}

function shouldShowAvatar(
  currentMessage: { senderId: string; createdAt: string },
  previousMessage?: { senderId: string; createdAt: string }
): boolean {
  if (!previousMessage) return true;
  if (previousMessage.senderId !== currentMessage.senderId) return true;
  // Show avatar if more than 5 minutes apart
  const diff =
    new Date(currentMessage.createdAt).getTime() - new Date(previousMessage.createdAt).getTime();
  return diff > 5 * 60 * 1000;
}

export function ThreadDetailPage({
  thread,
  currentUserId,
  isLoading,
  isSending,
  onSendMessage,
  onDeleteMessage,
  onMarkAsRead,
  onBack,
}: ThreadDetailPageProps) {
  const { t } = useTranslation();
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const [replyTo, setReplyTo] = useState<Message | null>(null);
  const [deleteConfirmId, setDeleteConfirmId] = useState<string | null>(null);

  // Mark as read when thread is viewed
  useEffect(() => {
    if (thread.unreadCount > 0) {
      onMarkAsRead();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [thread.unreadCount, onMarkAsRead]);

  // Scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  });

  const handleSendMessage = (
    content: string,
    attachments?: PendingAttachment[],
    replyToId?: string
  ) => {
    onSendMessage({ content, attachments, replyToId });
  };

  const handleReply = (message: Message) => {
    setReplyTo(message);
  };

  const handleCancelReply = () => {
    setReplyTo(null);
  };

  const handleDeleteClick = (messageId: string) => {
    setDeleteConfirmId(messageId);
  };

  const handleConfirmDelete = () => {
    if (deleteConfirmId && onDeleteMessage) {
      onDeleteMessage(deleteConfirmId);
      setDeleteConfirmId(null);
    }
  };

  const handleCancelDelete = () => {
    setDeleteConfirmId(null);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto h-[calc(100vh-8rem)] flex flex-col">
      {/* Header */}
      <div className="bg-white border-b px-4 py-3 flex items-center gap-3">
        <button
          type="button"
          onClick={onBack}
          className="p-1 text-gray-500 hover:text-gray-700 rounded-md hover:bg-gray-100"
          aria-label={t('common.back')}
        >
          <svg
            className="w-6 h-6"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
        </button>

        {/* Participant Info */}
        <div className="flex items-center gap-3 flex-1 min-w-0">
          {thread.participants.length === 2 ? (
            <>
              {thread.participants.find((p) => p.userId !== currentUserId)?.userAvatar ? (
                <img
                  src={thread.participants.find((p) => p.userId !== currentUserId)?.userAvatar}
                  alt=""
                  className="w-10 h-10 rounded-full object-cover"
                />
              ) : (
                <div className="w-10 h-10 rounded-full bg-gray-300 flex items-center justify-center">
                  <span className="text-gray-600 font-medium">
                    {thread.participants
                      .find((p) => p.userId !== currentUserId)
                      ?.userName.charAt(0)
                      .toUpperCase()}
                  </span>
                </div>
              )}
            </>
          ) : (
            <div className="w-10 h-10 rounded-full bg-gray-300 flex items-center justify-center">
              <span className="text-gray-600 text-sm font-medium">{thread.participantCount}</span>
            </div>
          )}
          <div className="min-w-0">
            <h1 className="font-semibold text-gray-900 truncate">
              {thread.subject || getParticipantNames(thread, currentUserId)}
            </h1>
            <p className="text-sm text-gray-500">
              {thread.participantCount}{' '}
              {thread.participantCount === 1
                ? t('messaging.participant')
                : t('messaging.participants')}
            </p>
          </div>
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto bg-gray-50 px-4 py-4">
        {thread.messages.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <svg
                className="mx-auto h-12 w-12 text-gray-400"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
                />
              </svg>
              <p className="mt-4 text-gray-500">{t('messaging.noMessages')}</p>
              <p className="mt-1 text-sm text-gray-400">{t('messaging.startTyping')}</p>
            </div>
          </div>
        ) : (
          <div className="space-y-3">
            {thread.messages.map((message, index) => {
              const previousMessage = thread.messages[index - 1];
              const showDate = shouldShowDateSeparator(
                message.createdAt,
                previousMessage?.createdAt
              );
              const showAvatar = shouldShowAvatar(message, previousMessage);

              return (
                <div key={message.id}>
                  {showDate && <DateSeparator date={message.createdAt} />}
                  <MessageBubble
                    message={message}
                    isCurrentUser={message.senderId === currentUserId}
                    showAvatar={showAvatar}
                    showTimestamp={showAvatar}
                    onReply={handleReply}
                    onDelete={onDeleteMessage ? handleDeleteClick : undefined}
                  />
                </div>
              );
            })}
            <div ref={messagesEndRef} />
          </div>
        )}
      </div>

      {/* Message Input */}
      <MessageInput
        onSendMessage={handleSendMessage}
        isSubmitting={isSending}
        replyTo={replyTo}
        onCancelReply={handleCancelReply}
      />

      {/* Delete Confirmation Dialog */}
      {deleteConfirmId && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          {/* Backdrop */}
          <div
            className="absolute inset-0 bg-black/50"
            onClick={handleCancelDelete}
            onKeyDown={(e) => {
              if (e.key === 'Escape') handleCancelDelete();
            }}
            role="button"
            tabIndex={0}
            aria-label={t('aria.closeDialog')}
          />

          {/* Dialog */}
          <div className="relative bg-white rounded-lg shadow-xl max-w-sm w-full mx-4 p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-2">
              {t('messaging.deleteMessageTitle')}
            </h2>
            <p className="text-gray-600 mb-6">{t('messaging.deleteMessageConfirm')}</p>
            <div className="flex justify-end gap-3">
              <button
                type="button"
                onClick={handleCancelDelete}
                className="px-4 py-2 text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors"
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                onClick={handleConfirmDelete}
                className="px-4 py-2 text-white bg-red-600 rounded-md hover:bg-red-700 transition-colors"
              >
                {t('common.delete')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
