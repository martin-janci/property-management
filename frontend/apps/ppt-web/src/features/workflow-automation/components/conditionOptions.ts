/**
 * Option values for automation conditions and actions.
 *
 * These mirror the Postgres enums the automation engine compares against
 * (`fault_category`, `fault_priority`, `fault_status`, `document_category`).
 * A condition only matches when its value is one of these, so the builder
 * must not offer values the database never stores.
 */

export const FAULT_CATEGORIES = [
  'plumbing',
  'electrical',
  'heating',
  'structural',
  'exterior',
  'elevator',
  'common_area',
  'security',
  'cleaning',
  'other',
] as const;

export const FAULT_PRIORITIES = ['low', 'medium', 'high', 'urgent'] as const;

export const FAULT_STATUSES = [
  'new',
  'triaged',
  'in_progress',
  'waiting_parts',
  'scheduled',
  'resolved',
  'closed',
  'reopened',
] as const;

export const DOCUMENT_TYPES = [
  'contracts',
  'invoices',
  'reports',
  'manuals',
  'certificates',
  'other',
] as const;
