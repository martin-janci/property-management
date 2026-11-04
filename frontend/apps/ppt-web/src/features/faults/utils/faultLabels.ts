/**
 * i18n key maps for fault enum values (status, priority, category).
 * Shared by fault list, card and triage components so enum labels are
 * translated consistently.
 */

import type { FaultCategory, FaultPriority, FaultStatus } from '../components/FaultCard';

export const faultStatusKeys: Record<FaultStatus, string> = {
  new: 'faults.statusNew',
  triaged: 'faults.statusTriaged',
  in_progress: 'faults.statusInProgress',
  waiting_parts: 'faults.statusWaitingParts',
  scheduled: 'faults.statusScheduled',
  resolved: 'faults.statusResolved',
  closed: 'faults.statusClosed',
  reopened: 'faults.statusReopened',
};

export const faultPriorityKeys: Record<FaultPriority, string> = {
  low: 'faults.priorityLow',
  medium: 'faults.priorityMedium',
  high: 'faults.priorityHigh',
  urgent: 'faults.priorityUrgent',
};

export const faultCategoryKeys: Record<FaultCategory, string> = {
  plumbing: 'faults.category.plumbing',
  electrical: 'faults.category.electrical',
  heating: 'faults.category.heating',
  structural: 'faults.category.structural',
  exterior: 'faults.category.exterior',
  elevator: 'faults.category.elevator',
  common_area: 'faults.category.common_area',
  security: 'faults.category.security',
  cleaning: 'faults.category.cleaning',
  other: 'faults.category.other',
};

export const faultStatuses = Object.keys(faultStatusKeys) as FaultStatus[];
export const faultPriorities: FaultPriority[] = ['urgent', 'high', 'medium', 'low'];
export const faultCategories = Object.keys(faultCategoryKeys) as FaultCategory[];
