/**
 * Voting feature — UI helper types and label/colour maps (Epic 5, FR37-44).
 *
 * Domain types (Vote, VoteWithDetails, …) come from `@ppt/api-client`; this
 * module only holds presentation helpers shared across the voting pages.
 */
import type { QuestionType, QuorumType, VoteStatus } from '@ppt/api-client';

/** i18n keys for the human-readable status label per vote status. */
export const VOTE_STATUS_LABEL_KEYS: Record<VoteStatus, string> = {
  draft: 'voting.status.draft',
  scheduled: 'voting.status.scheduled',
  active: 'voting.status.active',
  closed: 'voting.status.closed',
  cancelled: 'voting.status.cancelled',
};

/** Tailwind pill colours per vote status. */

export const VOTE_STATUS_COLORS: Record<VoteStatus, string> = {
  draft: 'bg-gray-100 text-gray-700',
  scheduled: 'bg-indigo-100 text-indigo-800',
  active: 'bg-green-100 text-green-800',
  closed: 'bg-blue-100 text-blue-800',
  cancelled: 'bg-red-100 text-red-800',
};

/** i18n keys for question-type labels in the create wizard + ballot. */
export const QUESTION_TYPE_LABEL_KEYS: Record<QuestionType, string> = {
  yes_no: 'voting.questionType.yes_no',
  single_choice: 'voting.questionType.single_choice',
  multiple_choice: 'voting.questionType.multiple_choice',
  ranked: 'voting.questionType.ranked',
};

/** i18n keys for quorum-rule labels in the create wizard + detail. */
export const QUORUM_TYPE_LABEL_KEYS: Record<QuorumType, string> = {
  simple_majority: 'voting.quorumType.simple_majority',
  two_thirds: 'voting.quorumType.two_thirds',
  weighted: 'voting.quorumType.weighted',
};

/** Participation as a 0–100 integer percentage, clamped. */
export function participationPercent(participation: number, eligible: number): number {
  if (!eligible || eligible <= 0) return 0;
  return Math.min(100, Math.round((participation / eligible) * 100));
}
