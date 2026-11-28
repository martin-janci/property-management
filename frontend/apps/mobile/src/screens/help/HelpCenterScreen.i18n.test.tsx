/**
 * HelpCenterScreen — FAQ-content & category-label i18n regression test
 * (code-review-mobile-rn-helpcenter-content-hardcoded-english).
 *
 * The Help Center shipped its FAQ questions/answers, category labels and
 * tutorial copy as hardcoded English literals baked into `HelpCenter.ts`, even
 * though react-i18next was already wired for the surrounding chrome. sk/cs/de/
 * pl/hu users saw a localized shell wrapped around an English FAQ list.
 *
 * This suite locks the contract: every FAQ/category/tutorial string is now
 * sourced from an i18n key under the `help.*` namespace and resolved via `t()`.
 *
 * Mirrors FeedbackScreen.i18n.test.tsx:
 *  1. Render layer — `src/test/setup.ts` mocks react-i18next so `t(key) => key`.
 *     We assert the screen renders the i18n KEYS and that the OLD hardcoded
 *     English literals are gone.
 *  2. Resource layer — the locale JSON files carry the keys and locale-distinct
 *     values.
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { HelpCenterScreen } from './HelpCenterScreen';

describe('HelpCenterScreen FAQ/category i18n', () => {
  it('renders FAQ + category i18n keys, not hardcoded literals', async () => {
    await render(<HelpCenterScreen onNavigate={jest.fn()} />);

    // FAQ questions are rendered from keys (t(key) => key under the mock).
    expect(screen.getByText('help.faqs.faq-1.question')).toBeTruthy();
    expect(screen.getByText('help.faqs.faq-4.question')).toBeTruthy();

    // Category chips render the label KEY (+ count) and the "all" filter key.
    expect(screen.getByText(/help\.categories\.getting_started/)).toBeTruthy();
    expect(screen.getByText('help.allCategories')).toBeTruthy();
  });

  it('no longer renders the old hardcoded English FAQ/category literals', async () => {
    await render(<HelpCenterScreen onNavigate={jest.fn()} />);

    expect(screen.queryByText('How do I view my buildings?')).toBeNull();
    expect(screen.queryByText('How do I report a fault?')).toBeNull();
    expect(screen.queryByText('Getting Started')).toBeNull();
    expect(screen.queryByText('Faults & Maintenance')).toBeNull();
    // The old bare "All" category chip literal is gone.
    expect(screen.queryByText('All')).toBeNull();
  });
});

describe('help FAQ/category/tutorial keys resolve to real, locale-distinct translations', () => {
  it('en carries the expected English copy', () => {
    expect(en.help.categories.getting_started).toBe('Getting Started');
    expect(en.help.faqs['faq-1'].question).toBe('How do I view my buildings?');
    expect(en.help.tutorials['tutorial-voting'].title).toBe('Participating in Building Votes');
    expect(en.help.content.dashboard.title).toBe('Dashboard Help');
    expect(en.help.allCategories).toBe('All');
  });

  it('every locale defines the FAQ, category, tutorial and content keys', () => {
    for (const locale of [en, sk, cs, de, pl, hu]) {
      // 10 FAQs, 10 categories, 3 tutorials, 5 help-content entries.
      expect(Object.keys(locale.help.faqs)).toHaveLength(10);
      expect(Object.keys(locale.help.categories)).toHaveLength(10);
      expect(Object.keys(locale.help.tutorials)).toHaveLength(3);
      expect(Object.keys(locale.help.content)).toHaveLength(5);
      // Every FAQ has a non-empty question and answer.
      for (const faq of Object.values(locale.help.faqs)) {
        expect(faq.question.length).toBeGreaterThan(0);
        expect(faq.answer.length).toBeGreaterThan(0);
      }
    }
  });

  it('translates FAQ/category copy away from English in every other locale', () => {
    for (const locale of [sk, cs, de, pl, hu]) {
      expect(locale.help.categories.getting_started).not.toBe(en.help.categories.getting_started);
      expect(locale.help.faqs['faq-1'].question).not.toBe(en.help.faqs['faq-1'].question);
      expect(locale.help.faqs['faq-9'].answer).not.toBe(en.help.faqs['faq-9'].answer);
    }
  });
});
