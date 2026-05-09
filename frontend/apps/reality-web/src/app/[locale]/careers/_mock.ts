export interface JobPosition {
  id: string;
  title: string;
  team: string;
  location: string;
  type: 'fulltime' | 'parttime' | 'contract';
}

export interface Benefit {
  icon: string;
  title: string;
  description: string;
}

export const MOCK_POSITIONS: JobPosition[] = [
  { id: '1', title: 'Senior Frontend Developer', team: 'Technológia', location: 'Bratislava / Remote', type: 'fulltime' },
  { id: '2', title: 'Backend Engineer (Rust)', team: 'Technológia', location: 'Bratislava / Remote', type: 'fulltime' },
  { id: '3', title: 'Product Designer', team: 'Dizajn', location: 'Bratislava', type: 'fulltime' },
  { id: '4', title: 'Real Estate Content Specialist', team: 'Marketing', location: 'Bratislava', type: 'fulltime' },
  { id: '5', title: 'Sales Account Manager', team: 'Predaj', location: 'Bratislava / Košice', type: 'fulltime' },
  { id: '6', title: 'Customer Support Specialist', team: 'Podpora', location: 'Remote', type: 'parttime' },
];

export const MOCK_BENEFITS: Benefit[] = [
  { icon: '🏠', title: 'Flexibilná práca', description: 'Hybridný model – pracujte z domova alebo v kancelárii podľa potreby.' },
  { icon: '📈', title: 'Rast a rozvoj', description: 'Ročný vzdelávací rozpočet €2 000 + prístup k online kurzom.' },
  { icon: '🏥', title: 'Zdravotné benefity', description: 'Súkromné zdravotné poistenie pre vás aj vašu rodinu.' },
  { icon: '🎯', title: 'Podiel na zisku', description: 'Motivačný program a akciové opcie pre kľúčových zamestnancov.' },
  { icon: '🌍', title: 'Tím zo srdca', description: 'Medzinárodný tím s plochými hierarchiami a otvorenou kultúrou.' },
  { icon: '☕', title: 'Kancelária v centre', description: 'Moderné priestory v Bratislave s neobmedzenou kávou a ovocím.' },
];
