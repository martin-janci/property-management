/**
 * Voting feature — UI helper types and label/colour maps (Epic 5, FR37-44).
 *
 * Domain types (Vote, VoteWithDetails, …) come from `@ppt/api-client`; this
 * module only holds presentation helpers shared across the voting pages.
 */
import type { QuestionType, QuorumType, VoteStatus } from '@ppt/api-client';

/** Human-readable status label + Tailwind pill colours per vote status. */
export const VOTE_STATUS_LABELS: Record<VoteStatus, string> = {
  draft: 'Draft',
  scheduled: 'Scheduled',
  active: 'Open',
  closed: 'Closed',
  cancelled: 'Cancelled',
};

export const VOTE_STATUS_COLORS: Record<VoteStatus, string> = {
  draft: 'bg-gray-100 text-gray-700',
  scheduled: 'bg-indigo-100 text-indigo-800',
  active: 'bg-green-100 text-green-800',
  closed: 'bg-blue-100 text-blue-800',
  cancelled: 'bg-red-100 text-red-800',
};

/** Question-type labels for the create wizard + ballot. */
export const QUESTION_TYPE_LABELS: Record<QuestionType, string> = {
  yes_no: 'Yes / No',
  single_choice: 'Single choice',
  multiple_choice: 'Multiple choice',
  ranked: 'Ranked preference',
};

/** Quorum-rule labels for the create wizard + detail. */
export const QUORUM_TYPE_LABELS: Record<QuorumType, string> = {
  simple_majority: 'Simple majority (50% + 1)',
  two_thirds: 'Two-thirds (2/3)',
  weighted: 'Weighted by ownership share',
};

/** Participation as a 0–100 integer percentage, clamped. */
export function participationPercent(participation: number, eligible: number): number {
  if (!eligible || eligible <= 0) return 0;
  return Math.min(100, Math.round((participation / eligible) * 100));
}
