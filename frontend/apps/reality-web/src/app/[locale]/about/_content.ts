/**
 * About page content — localized.
 *
 * Long-form prose lives here rather than in messages/*.json because the
 * structure is nested (sections × paragraphs / items) and next-intl's
 * flat-key model would either require a non-trivial array helper or
 * lots of indexed keys (section.mission.paragraph.0, ...). Keeping the
 * tree as data preserves IDE jump-to-definition and lets translators
 * edit one file per locale instead of hunting through a 4-level JSON.
 *
 * If a locale isn't present we fall back to English (declared first).
 */

import type { Locale } from '../../../i18n/config';

export interface AboutSection {
  id: string;
  heading: string;
  paragraphs?: string[];
  items?: { label: string; description: string }[];
}

interface AboutContent {
  hero: { subtitle: string };
  stats: { value: string; label: string }[];
  sections: AboutSection[];
}

const en: AboutContent = {
  hero: {
    subtitle:
      'Reality Portal helps people find a home and helps agents sell more — without unnecessary fees and with fair terms for both sides.',
  },
  stats: [
    { value: '12,000+', label: 'active listings' },
    { value: '850+', label: 'verified agents' },
    { value: '4 countries', label: 'SK · CZ · DE · EU' },
    { value: '180,000', label: 'monthly visits' },
  ],
  sections: [
    {
      id: 'mission',
      heading: 'Our mission',
      paragraphs: [
        'We believe finding a property should not be stressful. We make sure that every buyer, seller, and renter has a clear view of what the market offers, at what price, and from whom.',
        'We combine transparent listings, verified agent identities, and fair pricing into one platform that is as approachable for a family looking for their first flat as it is for a professional investor.',
      ],
    },
    {
      id: 'story',
      heading: 'Our story',
      paragraphs: [
        'Reality Portal was founded in Bratislava in 2024 out of frustration with existing portals — duplicate listings, hidden fees, and agency offers passed off as direct sales.',
        'We started with what we knew: a simple structure, clean search, and strict rules for agency listings. We added a price map, a market-analysis journal, and tools for agencies along the way. We are still building.',
      ],
    },
    {
      id: 'values',
      heading: 'Values that hold us together',
      items: [
        {
          label: 'Transparency',
          description:
            'No hidden fees, no retouched photos. A listing shows what the seller is offering — nothing more, nothing less.',
        },
        {
          label: 'Verification',
          description:
            'Every agent passes an identity check. Private sellers are tagged as such; agencies have a profile listing all their inventory.',
        },
        {
          label: 'Fair pricing',
          description:
            'Always free for private sellers. Tariffs for agencies scale with volume, with no surprise fees and no pressure to subscribe.',
        },
        {
          label: 'Local understanding',
          description:
            'Our team in Bratislava and Prague knows the regional differences — from price levels in Petržalka to market dynamics in Brno.',
        },
      ],
    },
    {
      id: 'team',
      heading: 'Who we do this for',
      paragraphs: [
        'For the family finally buying their own flat. For the retiree downsizing to a smaller place. For the investor comparing the yield on five plots in the same neighbourhood.',
        'And for the agents who do honest work and want clear tools, not yet another subscription with an unreadable contract.',
      ],
    },
    {
      id: 'contact',
      heading: 'Get in touch',
      paragraphs: [
        'Have feedback, an idea, or want to partner with us? Write to info@rlt.sk — we usually reply within 24 hours on business days.',
      ],
    },
  ],
};

const sk: AboutContent = {
  hero: {
    subtitle:
      'Reality Portál pomáha ľuďom nájsť domov a maklérom predať viac — bez zbytočných poplatkov a s férovými podmienkami pre obe strany.',
  },
  stats: [
    { value: '12 000+', label: 'aktívnych inzerátov' },
    { value: '850+', label: 'overených maklérov' },
    { value: '4 krajiny', label: 'SK · CZ · DE · EU' },
    { value: '180 000', label: 'mesačných návštev' },
  ],
  sections: [
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
  ],
};

// cs / de / pl / hu fall back to English until proper translations land.
const BY_LOCALE: Record<Locale, AboutContent> = {
  en,
  sk,
  cs: en,
  de: en,
  pl: en,
  hu: en,
};

export function getAboutContent(locale: string): AboutContent {
  return BY_LOCALE[locale as Locale] ?? en;
}

// Kept for backward compatibility with any importers that didn't migrate yet.
export const ABOUT_HERO = sk.hero;
export const ABOUT_STATS = sk.stats;
export const ABOUT_SECTIONS = sk.sections;
