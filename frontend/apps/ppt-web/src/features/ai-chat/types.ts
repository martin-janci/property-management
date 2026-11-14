/**
 * AI Chat Feature Types
 * Epic 127: AI Chatbot Interface
 *
 * TypeScript types for the AI chat feature.
 */

/** Message role in a conversation */
export type MessageRole = 'user' | 'assistant' | 'system';

/** Confidence level for AI responses */
export type ConfidenceLevel = 'low' | 'medium' | 'high';

/** AI source reference in response */
export interface AiSource {
  sourceType: string;
  sourceId: string;
  title: string;
  snippet?: string;
  relevanceScore: number;
}

/** Individual chat message */
export interface ChatMessage {
  id: string;
  sessionId: string;
  role: MessageRole;
  content: string;
  confidence?: number;
  sources: AiSource[];
  escalated: boolean;
  escalationReason?: string;
  tokensUsed?: number;
  latencyMs?: number;
  createdAt: string;
}

/** Chat session */
export interface ChatSession {
  id: string;
  organizationId: string;
  userId: string;
  title?: string;
  context: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
  lastMessageAt?: string;
}

/** Session summary for listing */
export interface ChatSessionSummary {
  id: string;
  title?: string;
  messageCount: number;
  lastMessageAt?: string;
  createdAt: string;
}

/** Request to create a chat session */
export interface CreateSessionRequest {
  title?: string;
  context?: Record<string, unknown>;
}

/** Request to send a message (API request body) */
export interface SendMessageRequest {
  content: string;
}

/**
 * Variables for the send-message mutation. The target session id is chosen at
 * mutate time (not at hook-call time), so a message sent immediately after a
 * session is created can be bound to the freshly-created id without waiting for
 * a re-render.
 */
export interface SendMessageVariables extends SendMessageRequest {
  sessionId: string;
}

/** Response from send message */
export interface SendMessageResponse {
  userMessage: ChatMessage;
  assistantMessage: ChatMessage;
}

/** Feedback on an AI message */
export interface MessageFeedback {
  messageId: string;
  rating?: number;
  helpful?: boolean;
  feedbackText?: string;
}

/** Suggested questions for quick start */
export interface SuggestedQuestion {
  id: string;
  text: string;
  category: string;
}

/** AI Chat state for context/store */
export interface AiChatState {
  currentSessionId: string | null;
  sessions: ChatSessionSummary[];
  messages: ChatMessage[];
  isLoading: boolean;
  isSending: boolean;
  error: string | null;
}

/** Streaming chunk for real-time display */
export interface StreamingChunk {
  type: 'start' | 'content' | 'end' | 'error';
  content?: string;
  messageId?: string;
  error?: string;
}
