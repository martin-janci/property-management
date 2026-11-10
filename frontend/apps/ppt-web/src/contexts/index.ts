/**
 * Contexts index for ppt-web.
 *
 * Barrel exports for all context providers and hooks.
 */

export type {
  AuthContextValue,
  AuthErrorCode,
  AuthState,
  AuthUser,
  LoginCredentials,
} from './AuthContext';
export { AuthError, AuthProvider, useAuth } from './AuthContext';

export { OrganizationProvider } from './OrganizationProvider';
export type {
  AuthContextForWebSocket,
  WebSocketContextValue,
  WebSocketProviderProps,
} from './WebSocketContext';
export {
  categoryToQueryKeys,
  eventToQueryKeys,
  useWebSocketContext,
  WebSocketProvider,
} from './WebSocketContext';
