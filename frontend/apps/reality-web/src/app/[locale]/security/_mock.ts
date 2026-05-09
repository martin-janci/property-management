export interface FraudScenario {
  id: string;
  icon: string;
  title: string;
  description: string;
  warning: string;
}

export interface Guarantee {
  id: string;
  icon: string;
  title: string;
  description: string;
}

export const MOCK_FRAUD_SCENARIOS: FraudScenario[] = [
  {
    id: 'advance-payment',
    icon: '💳',
    title: 'Záloha pred obhliadkou',
    description:
      'Podvodník požaduje zaplatenie zálohy alebo rezervačného poplatku skôr, ako sa vám umožní pozrieť nehnuteľnosť osobne.',
    warning: 'Nikdy neplaťte pred osobnou obhliadkou a overením totožnosti predajcu.',
  },
  {
    id: 'fake-listing',
    icon: '🖼️',
    title: 'Falošná ponuka',
    description:
      'Inzerát používa fotografie z inej nehnuteľnosti a ponúka nereálne nízku cenu, aby nalákalo obete. Nehnuteľnosť v skutočnosti nie je na predaj ani prenájom.',
    warning: 'Porovnajte fotografie pomocou spätného vyhľadávania obrázkov a overte adresu na mieste.',
  },
  {
    id: 'identity-theft',
    icon: '🎭',
    title: 'Krádež identity makléra',
    description:
      'Podvodník sa vydáva za legitímneho realitného makléra, kopíruje jeho profil a kontaktuje záujemcov s falošnými ponukami.',
    warning: 'Vždy si overte licenčné číslo makléra na webstránke Národnej asociácie realitných maklérov SR.',
  },
];

export const MOCK_GUARANTEES: Guarantee[] = [
  {
    id: 'verified',
    icon: '✅',
    title: 'Overení makléri',
    description: 'Každý maklér prechádza manuálnou verifikáciou licencie pred aktiváciou profilu.',
  },
  {
    id: 'secure',
    icon: '🔒',
    title: 'Šifrované spojenie',
    description: 'Všetka komunikácia prebieha cez TLS 1.3 a osobné údaje sú šifrované pri ukladaní.',
  },
  {
    id: 'report',
    icon: '🚩',
    title: 'Jednoduché nahlásenie',
    description: 'Každú ponuku môžete okamžite nahlásiť jedným klikom. Náš tím preverí podnet do 24 hodín.',
  },
  {
    id: '2fa',
    icon: '🛡️',
    title: 'Dvojfaktorová autentifikácia',
    description: 'Povinná 2FA pre všetkých maklérov a agentúry chráni pred neoprávneným prístupom k účtom.',
  },
];
