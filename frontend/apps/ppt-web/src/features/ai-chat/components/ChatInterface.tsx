/**
 * ChatInterface component
 * Epic 127: AI Chatbot Interface
 *
 * Main chat interface combining message list, input, and session management.
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  useAiChatMessages,
  useAiChatSessions,
  useCreateSession,
  useDeleteSession,
  useMessageFeedback,
  useSendMessage,
} from '../hooks';
import type { ChatMessage } from '../types';
import { ChatInput } from './ChatInput';
import { ChatMessageBubble } from './ChatMessageBubble';
import { SessionList } from './SessionList';
import { getDefaultSuggestedQuestions, SuggestedQuestions } from './SuggestedQuestions';

interface ChatInterfaceProps {
  initialSessionId?: string;
  showSidebar?: boolean;
  onSessionChange?: (sessionId: string | null) => void;
}

export function ChatInterface({
  initialSessionId,
  showSidebar = true,
  onSessionChange,
}: ChatInterfaceProps) {
  const { t } = useTranslation();
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(initialSessionId || null);
  const [sidebarOpen, setSidebarOpen] = useState(showSidebar);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Queries
  const { data: sessions = [], isLoading: sessionsLoading } = useAiChatSessions();
  const { data: messages = [], isLoading: messagesLoading } = useAiChatMessages(currentSessionId);

  // Mutations
  const createSession = useCreateSession();
  const deleteSession = useDeleteSession();
  const messageFeedback = useMessageFeedback();
  const sendMessage = useSendMessage(currentSessionId || '');

  // Scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Notify parent of session changes
  useEffect(() => {
    onSessionChange?.(currentSessionId);
  }, [currentSessionId, onSessionChange]);

  const handleNewSession = useCallback(async () => {
    try {
      const session = await createSession.mutateAsync({
        title: undefined,
        context: {},
      });
      setCurrentSessionId(session.id);
    } catch {
      // Error handling is managed by mutation
    }
  }, [createSession]);

  const handleSelectSession = useCallback((sessionId: string) => {
    setCurrentSessionId(sessionId);
  }, []);

  const handleDeleteSession = useCallback(
    async (sessionId: string) => {
      if (window.confirm(t('aiChat.confirmDelete'))) {
        try {
          await deleteSession.mutateAsync(sessionId);
          if (currentSessionId === sessionId) {
            setCurrentSessionId(null);
          }
        } catch {
          // Error handling is managed by mutation
        }
      }
    },
    [deleteSession, currentSessionId, t]
  );

  const handleSendMessage = useCallback(
    async (content: string) => {
      // Create session if needed
      if (!currentSessionId) {
        try {
          const session = await createSession.mutateAsync({
            title: content.slice(0, 50),
            context: {},
          });
          setCurrentSessionId(session.id);
          // Need to wait for session to be set, then send
          // This is a simplified version - in production you'd want better state management
          setTimeout(async () => {
            await sendMessage.mutateAsync({ content });
          }, 100);
        } catch {
          // Error handling
        }
      } else {
        try {
          await sendMessage.mutateAsync({ content });
        } catch {
          // Error handling
        }
      }
    },
    [currentSessionId, createSession, sendMessage]
  );

  const handleSuggestedQuestion = useCallback(
    (question: string) => {
      handleSendMessage(question);
    },
    [handleSendMessage]
  );

  const handleFeedback = useCallback(
    async (messageId: string, helpful: boolean) => {
      try {
        await messageFeedback.mutateAsync({
          messageId,
          helpful,
        });
      } catch {
        // Error handling
      }
    },
    [messageFeedback]
  );

  const suggestedQuestions = getDefaultSuggestedQuestions();
  const isSending = sendMessage.isPending || createSession.isPending;
  const showWelcome = !currentSessionId || messages.length === 0;

  return (
    <div className="flex h-full bg-white rounded-lg shadow-sm overflow-hidden">
      {/* Sidebar toggle for mobile */}
      {showSidebar && (
        <button
          type="button"
          onClick={() => setSidebarOpen(!sidebarOpen)}
          className="md:hidden absolute top-4 left-4 z-10 p-2 bg-white rounded-lg shadow"
          aria-label={sidebarOpen ? t('aiChat.closeSidebar') : t('aiChat.openSidebar')}
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
              d="M4 6h16M4 12h16M4 18h16"
            />
          </svg>
        </button>
      )}

      {/* Sidebar */}
      {showSidebar && (
        <div className={`${sidebarOpen ? 'block' : 'hidden'} md:block w-80 flex-shrink-0`}>
          <SessionList
            sessions={sessions}
            currentSessionId={currentSessionId}
            onSelectSession={handleSelectSession}
            onNewSession={handleNewSession}
            onDeleteSession={handleDeleteSession}
            isLoading={sessionsLoading}
          />
        </div>
      )}

      {/* Main chat area */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-blue-100 flex items-center justify-center">
              <svg
                className="w-6 h-6 text-blue-600"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
                />
              </svg>
            </div>
            <div>
              <h1 className="font-semibold text-gray-900">{t('aiChat.title')}</h1>
              <p className="text-sm text-gray-500">{t('aiChat.subtitle')}</p>
            </div>
          </div>
        </div>

        {/* Messages area */}
        <div className="flex-1 overflow-y-auto p-4">
          {messagesLoading ? (
            <div className="flex items-center justify-center h-full">
              <div className="flex items-center gap-2 text-gray-500">
                <div className="w-5 h-5 border-2 border-gray-300 border-t-blue-600 rounded-full animate-spin" />
                {t('aiChat.loading')}
              </div>
            </div>
          ) : showWelcome ? (
            <SuggestedQuestions
              questions={suggestedQuestions}
              onSelect={handleSuggestedQuestion}
              disabled={isSending}
            />
          ) : (
            <div role="log" aria-live="polite" aria-label={t('aiChat.messageLog')}>
              {messages.map((message: ChatMessage) => (
                <ChatMessageBubble
                  key={message.id}
                  message={message}
                  onFeedback={handleFeedback}
                  showFeedback={message.role === 'assistant'}
                />
              ))}
              <div ref={messagesEndRef} />
            </div>
          )}
        </div>

        {/* Input area */}
        <ChatInput
          onSend={handleSendMessage}
          disabled={isSending}
          placeholder={t('aiChat.inputPlaceholder')}
        />

        {/* Error display */}
        {sendMessage.isError && (
          <div className="mx-4 mb-4 p-3 bg-red-50 text-red-700 rounded-lg text-sm" role="alert">
            {t('aiChat.sendError')}
          </div>
        )}
      </div>
    </div>
  );
}
