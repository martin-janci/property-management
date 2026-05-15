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
  ResourceTableProps,
  ResourceTableColumn,
  ResourceTableAction,
} from './components/ResourceTable/ResourceTable';
export { ResourceTable } from './components/ResourceTable/ResourceTable';

export type { SettingsFormProps, SettingsField } from './components/SettingsForm/SettingsForm';
export { SettingsForm } from './components/SettingsForm/SettingsForm';

export type { AuditViewerProps, AuditEntry, AuditFilter } from './components/AuditViewer/AuditViewer';
export { AuditViewer } from './components/AuditViewer/AuditViewer';

export type { ImpersonationBannerProps } from './components/ImpersonationBanner/ImpersonationBanner';
export { ImpersonationBanner } from './components/ImpersonationBanner/ImpersonationBanner';

export type { CapabilityCheckerOptions } from './hooks/useCapability';
export {
  useCapability,
  CapabilityProvider,
  useCapabilityChecker,
} from './hooks/useCapability';
