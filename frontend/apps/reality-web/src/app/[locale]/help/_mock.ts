export interface HelpCategory {
  id: string;
  icon: string;
  title: string;
  description: string;
  articleCount: number;
}

export interface FaqItem {
  id: string;
  question: string;
  answer: string;
  category: string;
}

export const MOCK_CATEGORIES: HelpCategory[] = [
  {
    id: 'listings',
    icon: '🏠',
    title: 'Inzeráty',
    description: 'Ako pridať, upraviť alebo odstrániť inzerát',
    articleCount: 12,
  },
  {
    id: 'account',
    icon: '👤',
    title: 'Môj účet',
    description: 'Registrácia, prihlásenie a nastavenia profilu',
    articleCount: 8,
  },
  {
    id: 'search',
    icon: '🔍',
    title: 'Vyhľadávanie',
    description: 'Filtre, uložené vyhľadávania a upozornenia',
    articleCount: 6,
  },
  {
    id: 'payments',
    icon: '💳',
    title: 'Platby',
    description: 'Fakturácia, kredity a predplatné',
    articleCount: 10,
  },
];

export const MOCK_FAQ: FaqItem[] = [
  {
    id: 'faq-1',
    question: 'Ako pridám novú nehnuteľnosť na predaj?',
    answer:
      'Prihláste sa do svojho účtu a kliknite na „Pridať inzerát" v hornom menu. Vyplňte typ nehnuteľnosti, polohu, popis a pridajte fotografie. Inzerát bude po overení zverejnený do 2 hodín.',
    category: 'listings',
  },
  {
    id: 'faq-2',
    question: 'Ako môžem kontaktovať predajcu?',
    answer:
      'Na stránke každej ponuky nájdete tlačidlo „Kontaktovať makléra". Môžete poslať správu, naplánovať obhliadku alebo zavolať priamo (ak maklér zverejnil telefón).',
    category: 'listings',
  },
  {
    id: 'faq-3',
    question: 'Ako si vytvorím upozornenie na nové ponuky?',
    answer:
      'Nastavte si filtre vyhľadávania a kliknite na „Uložiť vyhľadávanie". Môžete zvoliť frekvenciu upozornení – okamžite, denne alebo týždenne.',
    category: 'search',
  },
  {
    id: 'faq-4',
    question: 'Je registrácia zdarma?',
    answer:
      'Áno, registrácia a prehliadanie ponúk sú bezplatné. Makléri a agentúry platia predplatné za pridávanie inzerátov. Pozrite si naše cenové plány.',
    category: 'account',
  },
  {
    id: 'faq-5',
    question: 'Ako zmením heslo?',
    answer:
      'Prejdite na „Môj účet" → „Heslo" a zadajte aktuálne a nové heslo. Ak ste zabudli heslo, použite odkaz „Zabudnuté heslo" na prihlasovacej stránke.',
    category: 'account',
  },
  {
    id: 'faq-6',
    question: 'Ako nahlásim podvodnú ponuku?',
    answer:
      'Na stránke inzerátu kliknite na ikonu „..." a vyberte „Nahlásiť ponuku". Náš tím podnet preverí do 24 hodín.',
    category: 'listings',
  },
];
