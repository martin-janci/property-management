import type { ReportProblemType } from '@ppt/reality-api-client';

// The radio value IS the wire value posted to reality-server as
// `problem_type`, so it must be one of the canonical `ReportProblemType`
// variants — no client-side remapping needed on submit.
export type ReportProblem = ReportProblemType;

export interface ReportProblemOption {
  value: ReportProblem;
  label: string;
  description: string;
}

export const REPORT_PROBLEMS: ReportProblemOption[] = [
  {
    value: 'fraudulent_listing',
    label: 'Podozrenie z podvodu',
    description: 'Podozrivý inzerát, žiadosť o platbu vopred, falošná identita makléra',
  },
  {
    value: 'price_manipulation',
    label: 'Nesprávna cena',
    description: 'Cena nezodpovedá skutočnosti alebo je zámer zavádzať',
  },
  {
    value: 'incorrect_information',
    label: 'Falošné fotografie',
    description: 'Fotografie nepatria k danej nehnuteľnosti',
  },
  {
    value: 'duplicate_listing',
    label: 'Duplicitný inzerát',
    description: 'Rovnaká nehnuteľnosť je inzerovaná viackrát',
  },
  {
    value: 'already_sold',
    label: 'Predaná / neprenájmaná',
    description:
      'Nehnuteľnosť je už predaná alebo prenajatá, inzerát nebol odstránený',
  },
  {
    value: 'other',
    label: 'Iný dôvod',
    description: 'Iná technická chyba alebo pravidlá platformy',
  },
];
