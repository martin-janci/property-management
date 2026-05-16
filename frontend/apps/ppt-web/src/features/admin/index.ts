/**
 * Phase 5 — admin feature barrel.
 *
 * The host app mounts `<AdminRouter>` under `/admin/*` inside its React Router
 * tree. The host is responsible for fetching the current user's capabilities
 * (typically via `GET /api/v1/admin/capabilities/users/:me`) and passing
 * them in.
 */

export type {
  ImpersonationWrapperProps,
  StoredImpersonation,
} from './ImpersonationWrapper';
export { ImpersonationWrapper } from './ImpersonationWrapper';
export type { MfaWrapperProps } from './MfaWrapper';
export { MfaWrapper } from './MfaWrapper';
export {
  AgenciesPage,
  AuditPage,
  FeatureFlagsPage,
  PlatformPage,
  UsersPage,
} from './pages';
export { AdminRouter } from './router';
export type { PrincipalCapabilitiesResult } from './usePrincipalCapabilities';
export { usePrincipalCapabilities } from './usePrincipalCapabilities';
