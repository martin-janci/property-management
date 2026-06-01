/**
 * Library Exports
 *
 * Centralized exports for ppt-web lib utilities (Story 79.1).
 */

// API Client
export {
  type ApiClientConfig,
  type ApiError,
  type ApiErrorResponse,
  configureApiClient,
  getApiClient,
  getErrorMessage,
  isApiError,
  resetApiClient,
  type TokenGetter,
} from './api';
// Error Handler (Story 79.3)
export {
  type ErrorResponse,
  formatValidationErrors,
  getFieldError,
  type ParsedApiError,
  parseApiError,
  type ValidationDetail,
} from './errorHandler';
// Query Keys
export {
  type AnnouncementFilters,
  type DocumentFilters,
  type DocumentSearchFilters,
  type FaultFilters,
  type FormFilters,
  type FormSubmissionFilters,
  type MessageFilters,
  type NeighborFilters,
  type PaginationParams,
  type QueryKeys,
  queryKeys,
  type VoteFilters,
} from './queryKeys';
// WebSocket (Story 79.4)
export {
  buildWebSocketUrl,
  type ConnectionState,
  type ConnectionStateHandler,
  createWebSocketService,
  type MessageHandler,
  parseServerMessage,
  type ServerWsEnvelope,
  type WebSocketEventType,
  type WebSocketMessage,
  WebSocketService,
  type WebSocketServiceConfig,
  WS_NOTIFICATIONS_PATH,
} from './websocket';
