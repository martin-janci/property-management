/**
 * Contact page content — Slovak source. See pages/about/_content.ts for the
 * rationale on Slovak-only content for legal/info pages.
 */

export const CONTACT_HERO = {
  subtitle:
    'Pomoc pre používateľov, partnerstvo s realitnými agentúrami, médiami alebo technický kontakt — vyberte si kanál nižšie.',
};

export interface ContactChannel {
  id: string;
  icon: string;
  title: string;
  description: string;
  cta: string;
  href: string;
}

export const CONTACT_CHANNELS: ContactChannel[] = [
  {
    id: 'support',
    icon: '💬',
    title: 'Podpora pre používateľov',
    description:
      'Otázky k inzerátom, účtu alebo platobnej zóne. Odpovedáme spravidla do 24 hodín v pracovných dňoch.',
    cta: 'info@rlt.sk',
    href: 'mailto:info@rlt.sk',
  },
  {
    id: 'agencies',
    icon: '🏢',
    title: 'Realitné agentúry',
    description:
      'Cenové plány, hromadný import inzerátov, integrácia s vaším CRM alebo demo cez videohovor.',
    cta: 'agency@rlt.sk',
    href: 'mailto:agency@rlt.sk',
  },
  {
    id: 'press',
    icon: '📰',
    title: 'Médiá a tlač',
    description:
      'Tlačové vyjadrenia, dáta o trhu, expertné komentáre, rozhovory. Spätnú väzbu poskytujeme zvyčajne do 48 hodín.',
    cta: 'press@rlt.sk',
    href: 'mailto:press@rlt.sk',
  },
  {
    id: 'security',
    icon: '🔒',
    title: 'Bezpečnostné nahlásenia',
    description:
      'Zraniteľnosti, abuse alebo podvodné inzeráty. Hlásenia spracovávame s vyššou prioritou.',
    cta: 'security@rlt.sk',
    href: 'mailto:security@rlt.sk',
  },
];

export const CONTACT_OFFICE = {
  heading: 'Sídlo a fakturačné údaje',
  rows: [
    { label: 'Spoločnosť', value: 'Reality Portál s.r.o.' },
    { label: 'Adresa', value: 'Mlynské nivy 5, 821 09 Bratislava, Slovenská republika' },
    { label: 'IČO', value: '00 000 000' },
    { label: 'IČ DPH', value: 'SK0000000000' },
    { label: 'Zápis', value: 'Okresný súd Bratislava I, oddiel Sro' },
  ],
};

export const CONTACT_HOURS = {
  heading: 'Pracovné hodiny',
  rows: [
    { label: 'Pondelok – Piatok', value: '9:00 – 18:00 (CET)' },
    { label: 'Sobota a nedeľa', value: 'Iba urgentné nahlásenia' },
    { label: 'Sviatky', value: 'Zavreté' },
  ],
};
