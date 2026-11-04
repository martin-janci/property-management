/** Status pill for a vote (Epic 5). */
import type { VoteStatus } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { VOTE_STATUS_COLORS, VOTE_STATUS_LABEL_KEYS } from '../types';

export function VoteStatusBadge({ status }: { status: VoteStatus }) {
  const { t } = useTranslation();
  return (
    <span
      className={`inline-block rounded px-2 py-0.5 text-xs font-medium ${VOTE_STATUS_COLORS[status]}`}
    >
      {t(VOTE_STATUS_LABEL_KEYS[status])}
    </span>
  );
}
