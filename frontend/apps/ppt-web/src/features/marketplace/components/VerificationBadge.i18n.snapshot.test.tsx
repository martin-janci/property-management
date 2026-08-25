/// <reference types="vitest/globals" />
/**
 * VerificationBadge expiry-copy i18n snapshot / visual-diff regression (#2825).
 *
 * Follow-up to #2824 (PR #2825), where the duplicated inline expiry logic in
 * `VerificationBadge` / `VerificationStatusBadge` was de-duplicated behind the
 * shared {@link getExpiryState} classifier and the hardcoded English expiry
 * suffixes (" (Expired)", " (Expiring soon)", "(Soon)") were moved onto the
 * `marketplace.verificationBadge.*` i18n namespace.
 *
 * The sibling suites cover the two halves in isolation:
 *   - `VerificationBadge.test.tsx`      — the expired/expiring boundary logic.
 *   - `../../i18n/verificationBadgeI18n.test.ts` — key presence across bundles.
 *
 * Neither renders the component in a non-English locale, so a future refactor
 * that re-inlines an English literal (e.g. `(Expired)`) would still pass both:
 * the logic test runs under the English test-global i18n, and the key-presence
 * test never touches the component. This suite closes that gap. It renders the
 * badges through a real per-locale i18next instance (same six-bundle,
 * `fallbackLng: 'en'` resolution the app uses at runtime) for sk / cs / de / en
 * and:
 *
 *   1. snapshots the rendered expiry copy for every locale × state, so any copy
 *      drift shows up as a reviewable visual diff; and
 *   2. asserts — with hard, non-snapshot assertions that `vitest -u` cannot
 *      silently rewrite — that a non-English render never leaks the English
 *      expiry literals and always shows that locale's own bundle string.
 */

import { render } from '@testing-library/react';
import i18next, { type i18n as I18nInstance } from 'i18next';
import { I18nextProvider } from 'react-i18next';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import cs from '../../../../messages/cs.json';
import de from '../../../../messages/de.json';
import en from '../../../../messages/en.json';
import hu from '../../../../messages/hu.json';
import pl from '../../../../messages/pl.json';
import sk from '../../../../messages/sk.json';
import {
  type Badge,
  type Verification,
  VerificationBadge,
  VerificationStatusBadge,
} from './VerificationBadge';

const DAY = 24 * 60 * 60 * 1000;
const NOW = new Date('2026-08-21T12:00:00.000Z');

function iso(offsetMs: number): string {
  return new Date(NOW.getTime() + offsetMs).toISOString();
}

/** Locales this suite exercises (the four called out in #2825). */
const LOCALES = ['en', 'sk', 'cs', 'de'] as const;
type Locale = (typeof LOCALES)[number];

/**
 * A standalone i18next instance mirroring `src/i18n/index.ts` (six bundles,
 * `fallbackLng: 'en'`), so each render exercises the SAME resolution + fallback
 * path the component sees at runtime rather than the English-only test global.
 * `createInstance` keeps it isolated from the app-global i18n singleton.
 */
function makeI18n(lng: Locale): I18nInstance {
  const instance = i18next.createInstance();
  instance.init({
    resources: {
      en: { translation: en },
      sk: { translation: sk },
      cs: { translation: cs },
      de: { translation: de },
      pl: { translation: pl },
      hu: { translation: hu },
    },
    lng,
    fallbackLng: 'en',
    supportedLngs: ['en', 'sk', 'cs', 'de', 'pl', 'hu'],
    interpolation: { escapeValue: false },
  });
  return instance;
}

/** Localized `marketplace.verificationBadge.*` leaf for the given locale. */
const BADGE_STRINGS: Record<Locale, Record<string, string>> = {
  en: en.marketplace.verificationBadge as Record<string, string>,
  sk: sk.marketplace.verificationBadge as Record<string, string>,
  cs: cs.marketplace.verificationBadge as Record<string, string>,
  de: de.marketplace.verificationBadge as Record<string, string>,
};

// The badge label appears twice in the DOM (visible text + the SVG <title>),
// so we read the tooltip from the outer <span>'s `title` attribute directly.
function badgeTitle(container: HTMLElement): string {
  return container.querySelector('span[title]')?.getAttribute('title') ?? '';
}

function renderBadge(lng: Locale, badge: Badge): string {
  const { container } = render(
    <I18nextProvider i18n={makeI18n(lng)}>
      <VerificationBadge badge={badge} />
    </I18nextProvider>
  );
  return badgeTitle(container);
}

function renderStatus(lng: Locale, verification: Verification): string {
  const { container } = render(
    <I18nextProvider i18n={makeI18n(lng)}>
      <VerificationStatusBadge verification={verification} />
    </I18nextProvider>
  );
  // The expiry hint line is the last <p> in the card body.
  const paras = container.querySelectorAll('p');
  return paras[paras.length - 1]?.textContent ?? '';
}

const EXPIRED_BADGE: Badge = {
  type: 'verified_business',
  awardedAt: iso(-400 * DAY),
  expiresAt: iso(-5 * DAY),
};
const EXPIRING_BADGE: Badge = {
  type: 'verified_business',
  awardedAt: iso(-300 * DAY),
  expiresAt: iso(10 * DAY),
};
const VALID_BADGE: Badge = {
  type: 'verified_business',
  awardedAt: iso(-1 * DAY),
  expiresAt: iso(200 * DAY),
};

/** The English literals that MUST NOT appear once a non-English locale is active. */
const ENGLISH_SUFFIXES = ['Expired', 'Expiring soon', 'Soon'] as const;

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(NOW);
});

afterEach(() => {
  vi.useRealTimers();
});

describe('VerificationBadge expiry copy — visual-diff snapshot across locales', () => {
  it('renders the expected tooltip copy for every locale × expiry state', () => {
    const matrix: Record<Locale, { expired: string; expiring: string; valid: string }> =
      Object.fromEntries(
        LOCALES.map((lng) => [
          lng,
          {
            expired: renderBadge(lng, EXPIRED_BADGE),
            expiring: renderBadge(lng, EXPIRING_BADGE),
            valid: renderBadge(lng, VALID_BADGE),
          },
        ])
      ) as Record<Locale, { expired: string; expiring: string; valid: string }>;

    // A future de-dupe/refactor that re-inlines English copy, drops a locale
    // string, or changes the tooltip shape shows up here as a reviewable diff.
    expect(matrix).toMatchInlineSnapshot(`
      {
        "cs": {
          "expired": "Ověřená firma (Vypršelo)",
          "expiring": "Ověřená firma (Brzy vyprší)",
          "valid": "Ověřená firma",
        },
        "de": {
          "expired": "Verifiziertes Unternehmen (Abgelaufen)",
          "expiring": "Verifiziertes Unternehmen (Läuft bald ab)",
          "valid": "Verifiziertes Unternehmen",
        },
        "en": {
          "expired": "Verified Business (Expired)",
          "expiring": "Verified Business (Expiring soon)",
          "valid": "Verified Business",
        },
        "sk": {
          "expired": "Overený podnik (Vypršané)",
          "expiring": "Overený podnik (Čoskoro vyprší)",
          "valid": "Overený podnik",
        },
      }
    `);
  });

  it('renders the expected VerificationStatusBadge expiry line for every locale', () => {
    const expiringVerification = (id: string): Verification => ({
      id,
      type: 'insurance',
      documentName: 'policy.pdf',
      status: 'verified',
      // A fixed calendar date keeps `toLocaleDateString()` stable under vitest's
      // default `en-US` runtime locale regardless of the badge's own language.
      expiryDate: '2026-09-01T00:00:00.000Z',
    });

    const lines = Object.fromEntries(
      LOCALES.map((lng) => [lng, renderStatus(lng, expiringVerification(lng))])
    ) as Record<Locale, string>;

    expect(lines).toMatchInlineSnapshot(`
      {
        "cs": "Platnost do: 9/1/2026 (Brzy)",
        "de": "Läuft ab: 9/1/2026 (Bald)",
        "en": "Expires: 9/1/2026 (Soon)",
        "sk": "Platnosť do: 9/1/2026 (Čoskoro)",
      }
    `);
  });
});

describe('VerificationBadge expiry copy — non-English regression guard', () => {
  // Hard assertions (not snapshots) so `vitest -u` cannot silently bless an
  // English-literal regression by rewriting a stored snapshot.
  const NON_ENGLISH = LOCALES.filter((l): l is Exclude<Locale, 'en'> => l !== 'en');

  for (const lng of NON_ENGLISH) {
    describe(`locale: ${lng}`, () => {
      it('shows the localized (expired) suffix, never the English literal', () => {
        const title = renderBadge(lng, EXPIRED_BADGE);
        expect(title).toContain(BADGE_STRINGS[lng].expired);
        for (const english of ENGLISH_SUFFIXES) {
          expect(title).not.toContain(english);
        }
      });

      it('shows the localized (expiring soon) suffix, never the English literal', () => {
        const title = renderBadge(lng, EXPIRING_BADGE);
        expect(title).toContain(BADGE_STRINGS[lng].expiringSoon);
        for (const english of ENGLISH_SUFFIXES) {
          expect(title).not.toContain(english);
        }
      });

      it('shows the localized (soon) hint on the status badge, never the English literal', () => {
        const line = renderStatus(lng, {
          id: '1',
          type: 'insurance',
          documentName: 'policy.pdf',
          status: 'verified',
          expiryDate: iso(7 * DAY),
        });
        expect(line).toContain(BADGE_STRINGS[lng].soon);
        for (const english of ENGLISH_SUFFIXES) {
          expect(line).not.toContain(english);
        }
      });
    });
  }
});
