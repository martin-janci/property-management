/**
 * @ppt/admin-ui — Admin console UI components.
 *
 * Phase 5 (Super-admin Control Plane). Mounts inside ppt-web's `/admin/*`
 * tree. All capability-gated UI affordances flow through `useCapability`
 * — never invent your own role checks.
 */

export type { Capability } from './capabilities';
export { CAPABILITIES } from './capabilities';
export type {
  AuditEntry,
  AuditFilter,
  AuditViewerProps,
} from './components/AuditViewer/AuditViewer';
export { AuditViewer } from './components/AuditViewer/AuditViewer';
export type {
  ImpersonationBannerLabels,
  ImpersonationBannerProps,
} from './components/ImpersonationBanner/ImpersonationBanner';
export { ImpersonationBanner } from './components/ImpersonationBanner/ImpersonationBanner';
export type {
  MfaChallengeLabels,
  MfaChallengeModalProps,
  MfaChallengeProviderProps,
} from './components/MfaChallengeModal';
export {
  MfaChallengeModal,
  MfaChallengeProvider,
  useMfaChallenge,
} from './components/MfaChallengeModal';
export type {
  ResourceTableAction,
  ResourceTableColumn,
  ResourceTableProps,
} from './components/ResourceTable/ResourceTable';
export { ResourceTable } from './components/ResourceTable/ResourceTable';
export type { SettingsField, SettingsFormProps } from './components/SettingsForm/SettingsForm';
export { SettingsForm } from './components/SettingsForm/SettingsForm';

export type { CapabilityCheckerOptions } from './hooks/useCapability';
export {
  CapabilityProvider,
  useCapability,
  useCapabilityChecker,
} from './hooks/useCapability';
