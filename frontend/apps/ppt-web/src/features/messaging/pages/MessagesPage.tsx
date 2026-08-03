/**
 * MessagesPage - Main messaging page listing all message threads.
 *
 * Displays a list of message threads with unread indicators, filter tabs,
 * bulk selection, and provides navigation to create new conversations or view existing ones.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ThreadList } from '../components/ThreadList';
import type { ListThreadsParams, MessageThread, ThreadFilterTab } from '../types';

interface MessagesPageProps {
  threads: MessageThread[];
  total: number;
  unreadCount: number;
  isLoading?: boolean;
  onNavigateToThread: (threadId: string) => void;
  onNavigateToCreate: () => void;
  onFilterChange: (params: ListThreadsParams) => void;
  onDeleteThreads?: (threadIds: string[]) => void;
  onArchiveThreads?: (threadIds: string[]) => void;
}

export function MessagesPage({
  threads,
  total,
  unreadCount,
  isLoading,
  onNavigateToThread,
  onNavigateToCreate,
  onFilterChange,
  onDeleteThreads,
  onArchiveThreads,
}: MessagesPageProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');
  const [page, setPage] = useState(1);
  const [activeTab, setActiveTab] = useState<ThreadFilterTab>('all');
  const [bulkSelectMode, setBulkSelectMode] = useState(false);
  const [selectedThreadIds, setSelectedThreadIds] = useState<Set<string>>(new Set());
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [showArchiveConfirm, setShowArchiveConfirm] = useState(false);
  const pageSize = 20;

  const handleSearch = (query: string) => {
    setSearchQuery(query);
    setPage(1);
    onFilterChange({ search: query || undefined, page: 1, pageSize, filter: activeTab });
  };

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    onFilterChange({
      search: searchQuery || undefined,
      page: newPage,
      pageSize,
      filter: activeTab,
    });
  };

  const handleTabChange = (tab: ThreadFilterTab) => {
    setActiveTab(tab);
    setPage(1);
    setSelectedThreadIds(new Set());
    onFilterChange({ search: searchQuery || undefined, page: 1, pageSize, filter: tab });
  };

  const handleToggleBulkSelect = () => {
    setBulkSelectMode(!bulkSelectMode);
    if (bulkSelectMode) {
      setSelectedThreadIds(new Set());
    }
  };

  const handleSelectThread = (threadId: string, selected: boolean) => {
    setSelectedThreadIds((prev) => {
      const newSet = new Set(prev);
      if (selected) {
        newSet.add(threadId);
      } else {
        newSet.delete(threadId);
      }
      return newSet;
    });
  };

  const handleSelectAll = () => {
    if (selectedThreadIds.size === threads.length) {
      setSelectedThreadIds(new Set());
    } else {
      setSelectedThreadIds(new Set(threads.map((t) => t.id)));
    }
  };

  const handleDeleteSelected = () => {
    if (selectedThreadIds.size > 0) {
      setShowDeleteConfirm(true);
    }
  };

  const handleConfirmDelete = () => {
    if (onDeleteThreads && selectedThreadIds.size > 0) {
      onDeleteThreads(Array.from(selectedThreadIds));
      setSelectedThreadIds(new Set());
      setBulkSelectMode(false);
    }
    setShowDeleteConfirm(false);
  };

  const handleArchiveSelected = () => {
    if (selectedThreadIds.size > 0) {
      setShowArchiveConfirm(true);
    }
  };

  const handleConfirmArchive = () => {
    if (onArchiveThreads && selectedThreadIds.size > 0) {
      onArchiveThreads(Array.from(selectedThreadIds));
      setSelectedThreadIds(new Set());
      setBulkSelectMode(false);
    }
    setShowArchiveConfirm(false);
  };

  const totalPages = Math.ceil(total / pageSize);

  const tabs: { key: ThreadFilterTab; label: string }[] = [
    { key: 'all', label: t('messaging.filterAll') },
    { key: 'unread', label: t('messaging.filterUnread') },
    { key: 'archived', label: t('messaging.filterArchived') },
  ];

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{t('messaging.title')}</h1>
          {unreadCount > 0 && (
            <p className="mt-1 text-sm text-gray-500">
              {t('messaging.unreadMessages', { count: unreadCount })}
            </p>
          )}
        </div>
        <div className="flex items-center gap-2">
          {/* Bulk select toggle */}
          <button
            type="button"
            onClick={handleToggleBulkSelect}
            className={`inline-flex items-center gap-2 px-3 py-2 border rounded-md transition-colors ${
              bulkSelectMode
                ? 'border-blue-600 text-blue-600 bg-blue-50'
                : 'border-gray-300 text-gray-700 hover:bg-gray-50'
            }`}
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
                d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            {t('common.select')}
          </button>

          {/* New message button */}
          <button
            type="button"
            onClick={onNavigateToCreate}
            className="inline-flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
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
                d="M12 4v16m8-8H4"
              />
            </svg>
            {t('messaging.newMessage')}
          </button>
        </div>
      </div>

      {/* Bulk actions bar */}
      {bulkSelectMode && (
        <div className="mb-4 p-3 bg-gray-100 rounded-lg flex items-center justify-between">
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={selectedThreadIds.size === threads.length && threads.length > 0}
                onChange={handleSelectAll}
                className="w-4 h-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
              />
              <span className="text-sm text-gray-700">{t('common.selectAll')}</span>
            </label>
            {selectedThreadIds.size > 0 && (
              <span className="text-sm text-gray-500">
                {t('messaging.selectedCount', { count: selectedThreadIds.size })}
              </span>
            )}
          </div>
          <div className="flex items-center gap-2">
            {onArchiveThreads && activeTab !== 'archived' && (
              <button
                type="button"
                onClick={handleArchiveSelected}
                disabled={selectedThreadIds.size === 0}
                className="inline-flex items-center gap-2 px-3 py-1.5 text-sm text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
                    d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4"
                  />
                </svg>
                {t('messaging.archive')}
              </button>
            )}
            {onDeleteThreads && (
              <button
                type="button"
                onClick={handleDeleteSelected}
                disabled={selectedThreadIds.size === 0}
                className="inline-flex items-center gap-2 px-3 py-1.5 text-sm text-red-600 bg-white border border-red-300 rounded-md hover:bg-red-50 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
        </div>
      )}

      {/* Filter tabs */}
      <div className="mb-4 border-b border-gray-200">
        <nav className="-mb-px flex gap-4" aria-label={t('aria.tabs')}>
          {tabs.map((tab) => (
            <button
              key={tab.key}
              type="button"
              onClick={() => handleTabChange(tab.key)}
              className={`py-3 px-1 border-b-2 text-sm font-medium transition-colors ${
                activeTab === tab.key
                  ? 'border-blue-600 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }`}
            >
              {tab.label}
              {tab.key === 'unread' && unreadCount > 0 && (
                <span className="ml-2 px-2 py-0.5 text-xs bg-blue-100 text-blue-600 rounded-full">
                  {unreadCount}
                </span>
              )}
            </button>
          ))}
        </nav>
      </div>

      {/* Search */}
      <div className="mb-4">
        <div className="relative">
          <svg
            className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => handleSearch(e.target.value)}
            placeholder={t('messaging.searchConversations')}
            className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
          />
        </div>
      </div>

      {/* Thread List */}
      <div className="bg-white rounded-lg shadow overflow-hidden">
        <ThreadList
          threads={threads}
          isLoading={isLoading}
          onSelectThread={bulkSelectMode ? undefined : onNavigateToThread}
          onCreateNew={onNavigateToCreate}
          bulkSelectMode={bulkSelectMode}
          selectedThreadIds={selectedThreadIds}
          onToggleSelect={handleSelectThread}
        />
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="mt-4 flex items-center justify-between">
          <p className="text-sm text-gray-500">
            {t('common.showing')} {(page - 1) * pageSize + 1} {t('common.to')}{' '}
            {Math.min(page * pageSize, total)} {t('common.of')} {total}
          </p>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => handlePageChange(page - 1)}
              disabled={page === 1}
              className="px-3 py-1 border border-gray-300 rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              {t('common.previous')}
            </button>
            <span className="text-sm text-gray-600">
              {page} / {totalPages}
            </span>
            <button
              type="button"
              onClick={() => handlePageChange(page + 1)}
              disabled={page === totalPages}
              className="px-3 py-1 border border-gray-300 rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              {t('common.next')}
            </button>
          </div>
        </div>
      )}

      {/* Delete Confirmation Dialog */}
      {showDeleteConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div
            className="absolute inset-0 bg-black/50"
            onClick={() => setShowDeleteConfirm(false)}
            onKeyDown={(e) => {
              if (e.key === 'Escape') setShowDeleteConfirm(false);
            }}
            role="button"
            tabIndex={0}
            aria-label={t('aria.closeDialog')}
          />
          <div className="relative bg-white rounded-lg shadow-xl max-w-sm w-full mx-4 p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-2">
              {t('messaging.deleteConversationsTitle')}
            </h2>
            <p className="text-gray-600 mb-6">
              {t('messaging.deleteConversationsConfirm', { count: selectedThreadIds.size })}
            </p>
            <div className="flex justify-end gap-3">
              <button
                type="button"
                onClick={() => setShowDeleteConfirm(false)}
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

      {/* Archive Confirmation Dialog */}
      {showArchiveConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div
            className="absolute inset-0 bg-black/50"
            onClick={() => setShowArchiveConfirm(false)}
            onKeyDown={(e) => {
              if (e.key === 'Escape') setShowArchiveConfirm(false);
            }}
            role="button"
            tabIndex={0}
            aria-label={t('aria.closeDialog')}
          />
          <div className="relative bg-white rounded-lg shadow-xl max-w-sm w-full mx-4 p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-2">
              {t('messaging.archiveConversationsTitle')}
            </h2>
            <p className="text-gray-600 mb-6">
              {t('messaging.archiveConversationsConfirm', { count: selectedThreadIds.size })}
            </p>
            <div className="flex justify-end gap-3">
              <button
                type="button"
                onClick={() => setShowArchiveConfirm(false)}
                className="px-4 py-2 text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors"
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                onClick={handleConfirmArchive}
                className="px-4 py-2 text-white bg-blue-600 rounded-md hover:bg-blue-700 transition-colors"
              >
                {t('messaging.archive')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
