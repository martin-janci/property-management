/**
 * AI Chat Hooks
 * Epic 127: AI Chatbot Interface
 *
 * TanStack Query hooks for AI chat feature.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getApiClient } from '../../../lib/api';
import type {
  ChatMessage,
  ChatSession,
  ChatSessionSummary,
  CreateSessionRequest,
  MessageFeedback,
  SendMessageRequest,
  SendMessageResponse,
} from '../types';

// Path is relative to the api-client baseURL (`/api/v1`). Routing through
// `getApiClient()` ensures the shared axios interceptors apply: Bearer-token
// injection, ErrorResponse → ApiError transformation, 401 → onUnauthorized,
// and transient 5xx/429 retry with backoff. A raw `fetch()` here would bypass
// the interceptor and go out unauthenticated (401 in prod).
const API_BASE = '/ai/chat';

/** Query keys for AI chat */
export const aiChatKeys = {
  all: ['ai-chat'] as const,
  sessions: () => [...aiChatKeys.all, 'sessions'] as const,
  session: (id: string) => [...aiChatKeys.all, 'session', id] as const,
  messages: (sessionId: string) => [...aiChatKeys.all, 'messages', sessionId] as const,
  escalated: () => [...aiChatKeys.all, 'escalated'] as const,
};

/** Fetch user's chat sessions */
export function useAiChatSessions(limit = 50, offset = 0) {
  return useQuery({
    queryKey: aiChatKeys.sessions(),
    queryFn: async () => {
      const res = await getApiClient().get<{ sessions: ChatSessionSummary[] }>(
        `${API_BASE}/sessions`,
        { params: { limit, offset } }
      );
      return res.data.sessions;
    },
    staleTime: 30 * 1000, // 30 seconds
  });
}

/** Fetch a single session with details */
export function useAiChatSession(sessionId: string | null) {
  return useQuery({
    queryKey: aiChatKeys.session(sessionId ?? ''),
    queryFn: async () => {
      if (!sessionId) return null;
      const res = await getApiClient().get<ChatSession>(`${API_BASE}/sessions/${sessionId}`);
      return res.data;
    },
    enabled: !!sessionId,
  });
}

/** Fetch messages for a session */
export function useAiChatMessages(sessionId: string | null, limit = 100, offset = 0) {
  return useQuery({
    queryKey: aiChatKeys.messages(sessionId ?? ''),
    queryFn: async () => {
      if (!sessionId) return [];
      const res = await getApiClient().get<{ messages: ChatMessage[] }>(
        `${API_BASE}/sessions/${sessionId}/messages`,
        { params: { limit, offset } }
      );
      return res.data.messages;
    },
    enabled: !!sessionId,
    staleTime: 10 * 1000, // 10 seconds
  });
}

/** Create a new chat session */
export function useCreateSession() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (request: CreateSessionRequest) => {
      const res = await getApiClient().post<ChatSession>(`${API_BASE}/sessions`, request);
      return res.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: aiChatKeys.sessions() });
    },
  });
}

/** Send a message in a session */
export function useSendMessage(sessionId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (request: SendMessageRequest) => {
      const res = await getApiClient().post<SendMessageResponse>(
        `${API_BASE}/sessions/${sessionId}/messages`,
        request
      );
      return res.data;
    },
    onSuccess: (data) => {
      // Update messages cache with new messages
      queryClient.setQueryData<ChatMessage[]>(aiChatKeys.messages(sessionId), (old) => {
        if (!old) return [data.userMessage, data.assistantMessage];
        return [...old, data.userMessage, data.assistantMessage];
      });
      // Also invalidate to ensure we have latest
      queryClient.invalidateQueries({ queryKey: aiChatKeys.sessions() });
    },
  });
}

/** Delete a chat session */
export function useDeleteSession() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (sessionId: string) => {
      // Route through the shared client: it throws on a non-2xx response, so a
      // failed delete rejects the mutation instead of being swallowed and
      // reported as success (which previously fired onSuccess and hid the
      // failure from the user).
      await getApiClient().delete(`${API_BASE}/sessions/${sessionId}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: aiChatKeys.sessions() });
    },
  });
}

/** Provide feedback on a message */
export function useMessageFeedback() {
  return useMutation({
    mutationFn: async (feedback: MessageFeedback) => {
      await getApiClient().post<void>(`${API_BASE}/messages/${feedback.messageId}/feedback`, {
        rating: feedback.rating,
        helpful: feedback.helpful,
        feedback_text: feedback.feedbackText,
      });
    },
  });
}

/** Fetch escalated messages */
export function useEscalatedMessages(limit = 50, offset = 0) {
  return useQuery({
    queryKey: aiChatKeys.escalated(),
    queryFn: async () => {
      const res = await getApiClient().get<{ messages: ChatMessage[] }>(`${API_BASE}/escalated`, {
        params: { limit, offset },
      });
      return res.data.messages;
    },
    staleTime: 60 * 1000, // 1 minute
  });
}
