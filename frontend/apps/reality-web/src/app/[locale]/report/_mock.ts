export type ReportProblem =
  | 'fraud'
  | 'wrong_price'
  | 'wrong_photos'
  | 'duplicate'
  | 'sold_not_removed'
  | 'other';

export interface ReportProblemOption {
  value: ReportProblem;
  label: string;
  description: string;
}

export const REPORT_PROBLEMS: ReportProblemOption[] = [
  {
    value: 'fraud',
    label: 'Podozrenie z podvodu',
    description: 'Podozrivý inzerát, žiadosť o platbu vopred, falošná identita makléra',
  },
  {
    value: 'wrong_price',
    label: 'Nesprávna cena',
    description: 'Cena nezodpovedá skutočnosti alebo je zámer zavádzať',
  },
  {
    value: 'wrong_photos',
    label: 'Falošné fotografie',
    description: 'Fotografie nepatria k danej nehnuteľnosti',
  },
  {
    value: 'duplicate',
    label: 'Duplicitný inzerát',
    description: 'Rovnaká nehnuteľnosť je inzerovaná viackrát',
  },
  {
    value: 'sold_not_removed',
    label: 'Predaná / neprenajímaná',
    description: 'Nehnuteľnosť je už predaná alebo prenajatá, inzerát nebol odstránený',
  },
  {
    value: 'other',
    label: 'Iný dôvod',
    description: 'Iná technická chyba alebo pravidlá platformy',
  },
];
