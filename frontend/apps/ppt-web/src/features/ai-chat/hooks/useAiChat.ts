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
  SendMessageResponse,
  SendMessageVariables,
} from '../types';

// Path is relative to the api-client baseURL (`/api/v1`). Routing through
// `getApiClient()` ensures the shared axios interceptors apply: Bearer-token
// injection, ErrorResponse → ApiError transformation, 401 → onUnauthorized,
// and transient 5xx/429 retry with backoff. A raw `fetch()` here would bypass
// the interceptor and go out unauthenticated (401 in prod).
const API_BASE = '/ai/chat';

/**
 * Query keys for AI chat.
 *
 * The paged reads (`sessions`, `messages`, `escalated`) are split into a
 * *scope* key and a *paged* key: the scope key (`sessions()`,
 * `messages(sessionId)`, `escalated()`) is the invalidation prefix, and the
 * paged key (`sessionsList(limit, offset)`, …) folds the `{ limit, offset }`
 * pagination inputs into the key as the last element — mirroring the repo's
 * `queryKeys.ts` factory convention (`list: (filters) => [...lists(), filters]`).
 *
 * This matters because the query functions read `limit`/`offset`: if the key
 * omitted them, two different pages (e.g. offset 0 and offset 100) would
 * collide on the same cache entry and TanStack Query would serve the first
 * page's cached rows for the second page's inputs. Folding the inputs into the
 * key gives each page its own cache entry. Invalidating the scope key still
 * prefix-matches every page, so `invalidateQueries({ queryKey: sessions() })`
 * continues to refresh all pages.
 */
export const aiChatKeys = {
  all: ['ai-chat'] as const,
  sessions: () => [...aiChatKeys.all, 'sessions'] as const,
  sessionsList: (limit: number, offset: number) =>
    [...aiChatKeys.sessions(), { limit, offset }] as const,
  session: (id: string) => [...aiChatKeys.all, 'session', id] as const,
  messages: (sessionId: string) => [...aiChatKeys.all, 'messages', sessionId] as const,
  messagesList: (sessionId: string, limit: number, offset: number) =>
    [...aiChatKeys.messages(sessionId), { limit, offset }] as const,
  escalated: () => [...aiChatKeys.all, 'escalated'] as const,
  escalatedList: (limit: number, offset: number) =>
    [...aiChatKeys.escalated(), { limit, offset }] as const,
};

/** Fetch user's chat sessions */
export function useAiChatSessions(limit = 50, offset = 0) {
  return useQuery({
    queryKey: aiChatKeys.sessionsList(limit, offset),
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
    queryKey: aiChatKeys.messagesList(sessionId ?? '', limit, offset),
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

/**
 * Send a message in a session.
 *
 * The target session id is passed per-call as a mutate variable rather than
 * bound at hook-call time. This lets a caller send the first message straight
 * to a just-created session (`mutateAsync({ sessionId, content })`) without
 * relying on a state update + timer to thread the id through — which previously
 * raced and could escape the caller's try/catch.
 */
export function useSendMessage() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ sessionId, content }: SendMessageVariables) => {
      const res = await getApiClient().post<SendMessageResponse>(
        `${API_BASE}/sessions/${sessionId}/messages`,
        { content }
      );
      return res.data;
    },
    onSuccess: (data, { sessionId }) => {
      // Append the new messages to every cached page of this session. Message
      // pages are now keyed by `{ limit, offset }`, so the optimistic write
      // targets the `messages(sessionId)` scope prefix via `setQueriesData`
      // (plural) rather than a single fixed key — otherwise the write would
      // land on a stale key that no live query reads and the new turn would
      // not appear until a refetch.
      queryClient.setQueriesData<ChatMessage[]>(
        { queryKey: aiChatKeys.messages(sessionId) },
        (old) => {
          if (!old) return [data.userMessage, data.assistantMessage];
          return [...old, data.userMessage, data.assistantMessage];
        }
      );
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
    queryKey: aiChatKeys.escalatedList(limit, offset),
    queryFn: async () => {
      const res = await getApiClient().get<{ messages: ChatMessage[] }>(`${API_BASE}/escalated`, {
        params: { limit, offset },
      });
      return res.data.messages;
    },
    staleTime: 60 * 1000, // 1 minute
  });
}
