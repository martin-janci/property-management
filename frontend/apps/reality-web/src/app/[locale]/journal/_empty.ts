import type { Locale } from '@/i18n/config';

/**
 * Placeholder empty-state copy for the Journal index.
 *
 * Lives next to `_mock.ts` on purpose: both are throwaway scaffolding for the
 * pre-CMS journal. The mock articles only render in local development; every
 * deployed build shows this honest "coming soon" state instead (see page.tsx).
 * Keeping the copy here — rather than in the global `messages/*.json` bundles —
 * means the whole placeholder layer (`_mock.ts` + `_empty.ts`) gets deleted in
 * one move when the real feed lands and brings its own localized content.
 */
export interface JournalEmptyCopy {
  title: string;
  body: string;
}

const EMPTY_COPY: Record<Locale, JournalEmptyCopy> = {
  en: {
    title: 'Articles coming soon',
    body: "We're putting the finishing touches on Reality Journal. Check back soon for analysis, tips and news from the world of real estate.",
  },
  sk: {
    title: 'Články čoskoro',
    body: 'Pripravujeme pre vás Reality Žurnál. Čoskoro tu nájdete analýzy, tipy a správy zo sveta nehnuteľností.',
  },
  cs: {
    title: 'Články už brzy',
    body: 'Připravujeme pro vás Reality Žurnál. Brzy zde najdete analýzy, tipy a novinky ze světa nemovitostí.',
  },
  de: {
    title: 'Artikel in Kürze',
    body: 'Wir arbeiten am Reality Journal. Schon bald finden Sie hier Analysen, Tipps und Neuigkeiten aus der Immobilienwelt.',
  },
  pl: {
    title: 'Artykuły już wkrótce',
    body: 'Przygotowujemy dla Ciebie Reality magazyn. Wkrótce znajdziesz tu analizy, porady i wiadomości ze świata nieruchomości.',
  },
  hu: {
    title: 'Hamarosan cikkek',
    body: 'A Reality magazinon dolgozunk. Hamarosan elemzéseket, tippeket és híreket találsz itt az ingatlanvilágból.',
  },
};

/** Empty-state copy for `locale`, falling back to English for any unknown value. */
export function journalEmptyCopy(locale: string): JournalEmptyCopy {
  return EMPTY_COPY[locale as Locale] ?? EMPTY_COPY.en;
}
