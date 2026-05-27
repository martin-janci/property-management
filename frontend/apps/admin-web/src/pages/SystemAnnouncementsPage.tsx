/**
 * Backward-compat re-export.
 *
 * The page implementation moved to
 * `frontend/apps/admin-web/src/features/system-announcements/SystemAnnouncementsPage.tsx`
 * as part of #521 (867 LoC monolith → focused components + zod-validated form).
 *
 * Existing route imports (`pages/SystemAnnouncementsPage`) keep working via
 * this re-export so the refactor stays a single self-contained PR.
 */

export { default } from '../features/system-announcements/SystemAnnouncementsPage';
