/**
 * PendingSubmissionsPanel — manager review of resident person-month
 * submissions (UC-10). Residents propose a count from the mobile app; the
 * manager approves it (the count is written) or rejects it.
 */

import type { PersonMonthSubmissionWithUnit } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface PendingSubmissionsPanelProps {
  submissions: PersonMonthSubmissionWithUnit[];
  isDeciding?: boolean;
  onApprove: (submissionId: string) => void;
  onReject: (submissionId: string) => void;
}

export function PendingSubmissionsPanel({
  submissions,
  isDeciding,
  onApprove,
  onReject,
}: PendingSubmissionsPanelProps) {
  const { t } = useTranslation();
  if (submissions.length === 0) return null;

  return (
    <section
      aria-labelledby="pm-submissions-title"
      className="bg-amber-50 border border-amber-200 rounded-lg p-4 mb-6"
    >
      <h2 id="pm-submissions-title" className="text-lg font-semibold text-gray-900 mb-3">
        {t('personMonths.submissions.title', { count: submissions.length })}
      </h2>
      <ul className="divide-y divide-amber-200">
        {submissions.map((s) => (
          <li key={s.id} className="py-3 flex flex-wrap items-center justify-between gap-3">
            <div className="text-sm text-gray-800">
              <div className="font-medium">
                {s.unit_designation} · {String(s.month).padStart(2, '0')}/{s.year}
              </div>
              <div>
                {t('personMonths.submissions.proposed', {
                  count: s.count,
                  current: s.current_count ?? '—',
                })}
              </div>
              {s.submitter_email && <div className="text-gray-500">{s.submitter_email}</div>}
              {s.note && <div className="text-gray-600 italic">“{s.note}”</div>}
            </div>
            <div className="flex gap-2">
              <button
                type="button"
                disabled={isDeciding}
                onClick={() => onApprove(s.id)}
                className="px-3 py-1.5 text-sm font-medium text-white bg-green-600 rounded-md hover:bg-green-700 disabled:opacity-50"
              >
                {t('personMonths.submissions.approve')}
              </button>
              <button
                type="button"
                disabled={isDeciding}
                onClick={() => onReject(s.id)}
                className="px-3 py-1.5 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
              >
                {t('personMonths.submissions.reject')}
              </button>
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}
