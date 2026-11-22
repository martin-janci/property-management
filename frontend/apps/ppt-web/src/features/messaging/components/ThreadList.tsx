/**
 * ThreadList Component
 *
 * Displays a list of message threads with unread indicators and bulk selection support.
 */

import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { MessageThread } from '../types';

interface ThreadListProps {
  threads: MessageThread[];
  selectedThreadId?: string;
  isLoading?: boolean;
  onSelectThread?: (threadId: string) => void;
  onCreateNew: () => void;
  bulkSelectMode?: boolean;
  selectedThreadIds?: Set<string>;
  onToggleSelect?: (threadId: string, selected: boolean) => void;
}

function formatTimeAgo(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const seconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (seconds < 60) return 'just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
  if (seconds < 604800) return `${Math.floor(seconds / 86400)}d`;
  return date.toLocaleDateString(getActiveLocale());
}

function getParticipantNames(thread: MessageThread, maxNames = 2): string {
  const names = thread.participants.map((p) => p.userName);
  if (names.length <= maxNames) {
    return names.join(', ');
  }
  return `${names.slice(0, maxNames).join(', ')} +${names.length - maxNames}`;
}

export function ThreadList({
  threads,
  selectedThreadId,
  isLoading,
  onSelectThread,
  onCreateNew,
  bulkSelectMode = false,
  selectedThreadIds = new Set(),
  onToggleSelect,
}: ThreadListProps) {
  const { t } = useTranslation();

  if (isLoading) {
    return (
      <div className="divide-y divide-gray-200">
        {['skeleton-1', 'skeleton-2', 'skeleton-3'].map((key) => (
          <div key={key} className="p-4 animate-pulse">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-full bg-gray-200" />
              <div className="flex-1 space-y-2">
                <div className="h-4 bg-gray-200 rounded w-1/3" />
                <div className="h-3 bg-gray-200 rounded w-2/3" />
              </div>
            </div>
          </div>
        ))}
      </div>
    );
  }

  if (threads.length === 0) {
    return (
      <div className="p-8 text-center">
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
        <h3 className="mt-4 text-lg font-medium text-gray-900">{t('messaging.noThreads')}</h3>
        <p className="mt-2 text-sm text-gray-500">{t('messaging.noThreadsDescription')}</p>
        <button
          type="button"
          onClick={onCreateNew}
          className="mt-4 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
        >
          {t('messaging.startConversation')}
        </button>
      </div>
    );
  }

  const handleThreadClick = (threadId: string) => {
    if (bulkSelectMode && onToggleSelect) {
      onToggleSelect(threadId, !selectedThreadIds.has(threadId));
    } else if (onSelectThread) {
      onSelectThread(threadId);
    }
  };

  const handleCheckboxChange = (e: React.ChangeEvent<HTMLInputElement>, threadId: string) => {
    e.stopPropagation();
    if (onToggleSelect) {
      onToggleSelect(threadId, e.target.checked);
    }
  };

  return (
    <div className="divide-y divide-gray-200">
      {threads.map((thread) => (
        <button
          key={thread.id}
          type="button"
          onClick={() => handleThreadClick(thread.id)}
          className={`w-full p-4 text-left hover:bg-gray-50 transition-colors ${
            selectedThreadId === thread.id ? 'bg-blue-50' : ''
          } ${thread.unreadCount > 0 ? 'bg-blue-25' : ''} ${
            bulkSelectMode && selectedThreadIds.has(thread.id) ? 'bg-blue-50' : ''
          }`}
        >
          <div className="flex items-start gap-3">
            {/* Checkbox for bulk select mode */}
            {bulkSelectMode && (
              <div className="flex-shrink-0 pt-1">
                <input
                  type="checkbox"
                  checked={selectedThreadIds.has(thread.id)}
                  onChange={(e) => handleCheckboxChange(e, thread.id)}
                  onClick={(e) => e.stopPropagation()}
                  className="w-4 h-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
                />
              </div>
            )}

            {/* Avatar(s) */}
            <div className="relative flex-shrink-0">
              {thread.participants.length === 1 ? (
                thread.participants[0].userAvatar ? (
                  <img
                    src={thread.participants[0].userAvatar}
                    alt={thread.participants[0].userName}
                    className="w-10 h-10 rounded-full object-cover"
                  />
                ) : (
                  <div className="w-10 h-10 rounded-full bg-gray-300 flex items-center justify-center">
                    <span className="text-gray-600 font-medium">
                      {thread.participants[0].userName.charAt(0).toUpperCase()}
                    </span>
                  </div>
                )
              ) : (
                <div className="w-10 h-10 rounded-full bg-gray-300 flex items-center justify-center">
                  <span className="text-gray-600 text-xs font-medium">
                    {thread.participantCount}
                  </span>
                </div>
              )}
              {thread.unreadCount > 0 && (
                <span className="absolute -top-1 -right-1 w-5 h-5 bg-blue-600 text-white text-xs rounded-full flex items-center justify-center">
                  {thread.unreadCount > 9 ? '9+' : thread.unreadCount}
                </span>
              )}
            </div>

            {/* Content */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between">
                <h3
                  className={`text-sm truncate ${
                    thread.unreadCount > 0
                      ? 'font-semibold text-gray-900'
                      : 'font-medium text-gray-700'
                  }`}
                >
                  {thread.subject || getParticipantNames(thread)}
                </h3>
                <div className="flex items-center gap-2 flex-shrink-0 ml-2">
                  {thread.isArchived && (
                    <span title={t('messaging.archived')}>
                      <svg
                        className="w-4 h-4 text-gray-400"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                        aria-hidden="true"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4"
                        />
                      </svg>
                    </span>
                  )}
                  {thread.lastMessageAt && (
                    <span className="text-xs text-gray-500">
                      {formatTimeAgo(thread.lastMessageAt)}
                    </span>
                  )}
                </div>
              </div>
              {thread.lastMessagePreview && (
                <p
                  className={`text-sm truncate mt-1 ${
                    thread.unreadCount > 0 ? 'text-gray-800' : 'text-gray-500'
                  }`}
                >
                  {thread.lastMessageSenderName && (
                    <span className="font-medium">{thread.lastMessageSenderName}: </span>
                  )}
                  {thread.lastMessagePreview}
                </p>
              )}
            </div>
          </div>
        </button>
      ))}
    </div>
  );
}
