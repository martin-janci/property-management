/**
 * i18n keys for dispute enum values (status, priority, category, …).
 *
 * Components translate these with `t(key)` so every label follows the app
 * language instead of being hard-coded English.
 */

import type { ActionStatus } from '../components/ActionItemCard';
import type { DisputeCategory, DisputePriority, DisputeStatus } from '../components/DisputeCard';
import type { ResolutionStatus } from '../components/ResolutionCard';

export const disputeStatusKeys: Record<DisputeStatus, string> = {
  filed: 'disputes.statusFiled',
  under_review: 'disputes.statusUnderReview',
  mediation: 'disputes.statusInMediation',
  awaiting_response: 'disputes.statusAwaitingResponse',
  resolved: 'disputes.statusResolved',
  escalated: 'disputes.statusEscalated',
  withdrawn: 'disputes.statusWithdrawn',
  closed: 'disputes.statusClosed',
};

export const disputePriorityKeys: Record<DisputePriority, string> = {
  low: 'disputes.priorityLow',
  medium: 'disputes.priorityMedium',
  high: 'disputes.priorityHigh',
  urgent: 'disputes.priorityUrgent',
};

export const disputeCategoryKeys: Record<DisputeCategory, string> = {
  noise: 'disputes.categoryNoise',
  damage: 'disputes.categoryPropertyDamage',
  payment: 'disputes.categoryPayment',
  lease_terms: 'disputes.categoryLeaseTerms',
  common_area: 'disputes.categoryCommonArea',
  parking: 'disputes.categoryParking',
  pets: 'disputes.categoryPets',
  maintenance: 'disputes.categoryMaintenance',
  privacy: 'disputes.categoryPrivacy',
  harassment: 'disputes.categoryHarassment',
  other: 'disputes.categoryOther',
};

export const actionStatusKeys: Record<ActionStatus, string> = {
  pending: 'disputes.action.status.pending',
  in_progress: 'disputes.action.status.inProgress',
  completed: 'disputes.action.status.completed',
  overdue: 'disputes.action.status.overdue',
  escalated: 'disputes.action.status.escalated',
};

export const resolutionStatusKeys: Record<ResolutionStatus, string> = {
  proposed: 'disputes.resolution.status.proposed',
  accepted: 'disputes.resolution.status.accepted',
  rejected: 'disputes.resolution.status.rejected',
  partially_accepted: 'disputes.resolution.status.partiallyAccepted',
  implemented: 'disputes.resolution.status.implemented',
};

export const partyRoleKeys: Record<'complainant' | 'respondent' | 'witness' | 'mediator', string> =
  {
    complainant: 'disputes.partyRole.complainant',
    respondent: 'disputes.partyRole.respondent',
    witness: 'disputes.partyRole.witness',
    mediator: 'disputes.partyRole.mediator',
  };

/** Translate a dispute status value, falling back to the raw value if unknown. */
export function disputeStatusLabel(t: (key: string) => string, status: string): string {
  const key = disputeStatusKeys[status as DisputeStatus];
  return key ? t(key) : status;
}

/** Calendar date in the app language (not the browser locale). */
export function formatDisputeDate(dateString: string, language: string): string {
  return new Date(dateString).toLocaleDateString(language);
}

/** Whole days between `from` and now: positive in the past. */
export function daysSince(dateString: string, now: Date = new Date()): number {
  return Math.floor((now.getTime() - new Date(dateString).getTime()) / 86_400_000);
}
