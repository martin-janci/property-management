/**
 * Phase 5 — admin feature barrel.
 *
 * The host app mounts `<AdminRouter>` under `/admin/*` inside its React Router
 * tree. The host is responsible for fetching the current user's capabilities
 * (typically via `GET /api/v1/admin/capabilities/users/:me`) and passing
 * them in.
 */

export { AdminRouter } from './router';
export {
  AgenciesPage,
  AuditPage,
  FeatureFlagsPage,
  PlatformPage,
  UsersPage,
} from './pages';
export { usePrincipalCapabilities } from './usePrincipalCapabilities';
export type { PrincipalCapabilitiesResult } from './usePrincipalCapabilities';
export { ImpersonationWrapper } from './ImpersonationWrapper';
export type {
  ImpersonationWrapperProps,
  StoredImpersonation,
} from './ImpersonationWrapper';
