export interface Feature {
  id: string;
  icon: string;
  title: string;
  description: string;
}

export interface PricingPlan {
  id: string;
  name: string;
  price: number;
  period: string;
  currency: string;
  features: string[];
  highlighted?: boolean;
}

export const MOCK_FEATURES: Feature[] = [
  {
    id: 'listings',
    icon: '📋',
    title: 'Neobmedzené inzeráty',
    description: 'Publikujte ľubovoľný počet nehnuteľností bez skrytých poplatkov.',
  },
  {
    id: 'leads',
    icon: '📨',
    title: 'Správa dopytov',
    description: 'Centralizovaná schránka pre všetky dopyty od záujemcov.',
  },
  {
    id: 'analytics',
    icon: '📊',
    title: 'Analytika & štatistiky',
    description: 'Sledujte výkonnosť ponúk, zobrazenia a konverzie v reálnom čase.',
  },
  {
    id: 'crm',
    icon: '🤝',
    title: 'CRM integrácia',
    description: 'Prepojte s populárnymi CRM systémami cez XML feed alebo API.',
  },
  {
    id: 'brand',
    icon: '🎨',
    title: 'Vlastný branding',
    description: 'Logo, farby a profilová stránka agentúry s vlastnou URL adresou.',
  },
  {
    id: 'team',
    icon: '👥',
    title: 'Tímový prístup',
    description: 'Pridajte viacerých maklérov a spravujte oprávnenia centrálne.',
  },
];

export const MOCK_PRICING: PricingPlan[] = [
  {
    id: 'starter',
    name: 'Štart',
    price: 29,
    period: 'mesiac',
    currency: '€',
    features: ['До 10 aktívnych inzerátov', 'Základná štatistika', 'E-mailová podpora'],
  },
  {
    id: 'pro',
    name: 'Profesionál',
    price: 79,
    period: 'mesiac',
    currency: '€',
    highlighted: true,
    features: [
      'Neobmedzené inzeráty',
      'Správa dopytov',
      'Pokročilá analytika',
      'CRM integrácia',
      'Prioritná podpora',
    ],
  },
  {
    id: 'agency',
    name: 'Agentúra',
    price: 199,
    period: 'mesiac',
    currency: '€',
    features: [
      'Všetko v Profesionál',
      'Tím do 20 maklérov',
      'Vlastný branding',
      'Dedikovaný account manager',
      'SLA 99,9 %',
    ],
  },
];
