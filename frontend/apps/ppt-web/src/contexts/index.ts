/**
 * Contexts index for ppt-web.
 *
 * Barrel exports for all context providers and hooks.
 */

export { AuthProvider, useAuth, AuthError } from './AuthContext';
export type {
  AuthContextValue,
  AuthErrorCode,
  AuthState,
  AuthUser,
  LoginCredentials,
} from './AuthContext';

export { OrganizationProvider } from './OrganizationProvider';

export { WebSocketProvider, useWebSocketContext, eventToQueryKeys } from './WebSocketContext';
export type {
  WebSocketProviderProps,
  WebSocketContextValue,
  AuthContextForWebSocket,
} from './WebSocketContext';
