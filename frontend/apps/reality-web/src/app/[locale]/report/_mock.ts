import type { ReportProblemType } from '@ppt/reality-api-client';

// The radio value IS the wire value posted to reality-server as
// `problem_type`, so it must be one of the canonical `ReportProblemType`
// variants — no client-side remapping needed on submit.
export type ReportProblem = ReportProblemType;

// Display order of the problem radio cards. The label/description copy lives in
// next-intl under `pages.report.problems.<value>.{label,description}`; the page
// resolves it per-locale via `useTranslations`, so this module holds only the
// stable wire values and their presentation order.
export const REPORT_PROBLEMS: ReportProblem[] = [
  'fraudulent_listing',
  'price_manipulation',
  'incorrect_information',
  'duplicate_listing',
  'already_sold',
  'other',
];
