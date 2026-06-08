/** Status pill for a vote (Epic 5). */
import type { VoteStatus } from '@ppt/api-client';
import { VOTE_STATUS_COLORS, VOTE_STATUS_LABELS } from '../types';

export function VoteStatusBadge({ status }: { status: VoteStatus }) {
  return (
    <span
      className={`inline-block rounded px-2 py-0.5 text-xs font-medium ${VOTE_STATUS_COLORS[status]}`}
    >
      {VOTE_STATUS_LABELS[status]}
    </span>
  );
}
