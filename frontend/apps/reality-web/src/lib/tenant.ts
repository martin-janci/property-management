/**
 * Server-side tenant helper (Phase 6: C1).
 *
 * Thin re-export surface so server components and page data-loaders can
 * import from `@/lib/tenant` without pulling in the full tenant-config
 * module surface (React `cache()`, sanitizer, style helpers).
 *
 * The canonical implementation lives in `@/lib/tenant-config`.
 * This module exists for consistency with the C1 spec and as a stable
 * import path that can grow without breaking callers.
 */

export type {
  FeatureFlagState,
  TenantBranding,
  TenantConfig,
} from './tenant-config';

export {
  brandingToStyleObject,
  brandingToStyleString,
  getFeatureFlag,
  getTenantConfig,
  isSafeCssVarName,
  sanitizeCssValue,
} from './tenant-config';
