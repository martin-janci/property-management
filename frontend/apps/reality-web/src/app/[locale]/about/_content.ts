/**
 * About page content — Slovak source. Mirror translation work into the locale
 * JSON later if the page becomes a localization target; for now legal/info
 * pages on this primarily-Slovak portal ship in Slovak only (matches the
 * cookies/security/help pattern in this repo).
 */

export interface AboutSection {
  id: string;
  heading: string;
  paragraphs?: string[];
  items?: { label: string; description: string }[];
}

export const ABOUT_HERO = {
  subtitle:
    'Reality Portál pomáha ľuďom nájsť domov a maklérom predať viac — bez zbytočných poplatkov a s férovými podmienkami pre obe strany.',
};

export const ABOUT_STATS: { value: string; label: string }[] = [
  { value: '12 000+', label: 'aktívnych inzerátov' },
  { value: '850+', label: 'overených maklérov' },
  { value: '4 krajiny', label: 'SK · CZ · DE · EU' },
  { value: '180 000', label: 'mesačných návštev' },
];

export const ABOUT_SECTIONS: AboutSection[] = [
  {
    id: 'mission',
    heading: 'Naša misia',
    paragraphs: [
      'Veríme, že hľadanie nehnuteľnosti by nemalo byť stresujúce. Robíme to, aby každý — kupujúci, predávajúci aj nájomca — mal jasný prehľad o tom, čo trh ponúka, za akú cenu a od koho.',
      'Spájame transparentné inzeráty, overené identity maklérov a férové ceny do jednej platformy, ktorá je rovnako blízka rodine hľadajúcej prvý byt ako profesionálnemu investorovi.',
    ],
  },
  {
    id: 'story',
    heading: 'Príbeh portálu',
    paragraphs: [
      'Reality Portál vznikol v Bratislave v roku 2024 z frustrácie z existujúcich portálov — duplicitné inzeráty, skryté poplatky a maklérske ponuky vydávané za priame predaje.',
      'Začali sme tým, čo poznáme: jednoduchá štruktúra, čisté vyhľadávanie a striktné pravidlá pre maklérske inzeráty. Postupne sme pridali mapu cien, žurnál s analýzami trhu a nástroje pre agentúry. Stále staviame ďalej.',
    ],
  },
  {
    id: 'values',
    heading: 'Hodnoty, ktoré nás držia',
    items: [
      {
        label: 'Transparentnosť',
        description:
          'Žiadne skryté poplatky, žiadne prikrášlené fotografie. Inzerát ukazuje to, čo predávajúci ponúka — nič viac, nič menej.',
      },
      {
        label: 'Overenie',
        description:
          'Každý maklér prechádza identifikáciou. Súkromní predávajúci sú označení, agentúry majú profil so všetkými inzerátmi.',
      },
      {
        label: 'Férová cena',
        description:
          'Pre súkromných predávajúcich vždy zadarmo. Pre maklérov tarif podľa objemu, bez prekvapení a bez nátlaku na predplatné.',
      },
      {
        label: 'Lokálne porozumenie',
        description:
          'Tím v Bratislave a Prahe pozná regionálne rozdiely — od cenovej hladiny v Petržalke po realitnú dynamiku v Brne.',
      },
    ],
  },
  {
    id: 'team',
    heading: 'Pre koho to robíme',
    paragraphs: [
      'Pre rodinu, ktorá konečne kupuje vlastný byt. Pre dôchodcu, ktorý znižuje a hľadá menšie. Pre investora, ktorý porovnáva výnosnosť piatich pozemkov v rovnakej štvrti.',
      'A pre maklérov, ktorí svoju prácu robia poctivo a chcú jasné nástroje, nie ďalšie predplatné s nečitateľným kontraktom.',
    ],
  },
  {
    id: 'contact',
    heading: 'Spojte sa s nami',
    paragraphs: [
      'Máte spätnú väzbu, návrh alebo chcete spolupracovať? Napíšte nám na info@rlt.sk — odpovedáme zvyčajne do 24 hodín v pracovných dňoch.',
    ],
  },
];
