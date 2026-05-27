/**
 * Validation rules for the system-announcement create/edit form (#521).
 *
 * The follow-up suggested a `react-hook-form` + `zod` migration. Neither is
 * a current admin-web dependency, and we can't add deps in this refactor,
 * so we replace the prior native-HTML5 hints with a small, tested validator
 * that returns a field → message map. The form component renders these
 * inline (no more `required` browser tooltips). When `react-hook-form` /
 * `zod` are introduced project-wide, this module is the single seam to
 * swap for `zodResolver(annFormSchema)`.
 */

import type { SystemAnnouncementSeverity } from '@ppt/api-client';

const ALLOWED_SEVERITIES: ReadonlySet<SystemAnnouncementSeverity> = new Set([
  'info',
  'warning',
  'critical',
]);

export interface AnnFormValues {
  title: string;
  message: string;
  severity: SystemAnnouncementSeverity;
  start_at: string;
  end_at: string;
  is_dismissible: boolean;
  requires_acknowledgment: boolean;
}

export type AnnFormFieldErrors = Partial<Record<keyof AnnFormValues, string>>;

/**
 * Validate form values. Returns an empty object when the form is valid.
 *
 * Order matters per field — the first failing rule wins so we don't double-
 * surface a value (e.g. "required" and "max length" at once).
 */
export function validateAnnForm(values: AnnFormValues): AnnFormFieldErrors {
  const errors: AnnFormFieldErrors = {};

  const title = values.title.trim();
  if (!title) {
    errors.title = 'Title is required.';
  } else if (title.length > 200) {
    errors.title = 'Title must be 200 characters or fewer.';
  }

  const message = values.message.trim();
  if (!message) {
    errors.message = 'Message is required.';
  } else if (message.length > 2000) {
    errors.message = 'Message must be 2000 characters or fewer.';
  }

  if (!ALLOWED_SEVERITIES.has(values.severity)) {
    errors.severity = 'Severity must be info, warning, or critical.';
  }

  if (!values.start_at) {
    errors.start_at = 'Publish-at is required.';
  }

  if (values.end_at) {
    const startTs = new Date(values.start_at).getTime();
    const endTs = new Date(values.end_at).getTime();
    if (Number.isFinite(startTs) && Number.isFinite(endTs) && endTs <= startTs) {
      errors.end_at = 'Expire-at must be after Publish-at.';
    }
  }

  return errors;
}
