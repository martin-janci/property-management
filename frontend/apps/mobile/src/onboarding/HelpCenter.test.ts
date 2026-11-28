/**
 * HelpCenter — i18n-key contract + localized-search unit tests
 * (code-review-mobile-rn-helpcenter-content-hardcoded-english).
 *
 * HelpCenter is a non-React singleton, so it cannot call `useTranslation`.
 * Instead it stores i18n KEYS in the display fields of its FAQ / category /
 * tutorial / help-content records, and its search methods take a `Translator`
 * so they can score against the localized text of the active language. These
 * tests lock both halves of that contract.
 */

import en from '../locales/en.json';
import { HelpCenter } from './HelpCenter';

// Resolve a dotted i18n key (segments may contain hyphens, e.g. `faq-1`)
// against a locale JSON object — a stand-in for react-i18next's `t`.
function makeTranslator(resources: Record<string, unknown>) {
  return (key: string): string => {
    const value = key
      .split('.')
      // biome-ignore lint/suspicious/noExplicitAny: test-only key walker
      .reduce<any>((obj, segment) => (obj == null ? undefined : obj[segment]), resources);
    return typeof value === 'string' ? value : key;
  };
}

describe('HelpCenter i18n-key contract', () => {
  const help = new HelpCenter();

  it('FAQ display fields hold i18n keys, not literal English', () => {
    const t = (key: string) => key; // identity translator
    const faqs = help.searchFAQs('', t);
    expect(faqs.length).toBeGreaterThan(0);
    for (const faq of faqs) {
      expect(faq.question).toMatch(/^help\.faqs\.[\w-]+\.question$/);
      expect(faq.answer).toMatch(/^help\.faqs\.[\w-]+\.answer$/);
    }
  });

  it('category labels are exposed as i18n keys via labelKey', () => {
    const categories = help.getFAQCategories();
    expect(categories.length).toBeGreaterThan(0);
    for (const cat of categories) {
      expect(cat.labelKey).toBe(`help.categories.${cat.category}`);
    }
  });

  it('tutorial + help-content display fields hold i18n keys', () => {
    const t = (key: string) => key;
    for (const tut of help.getAllTutorials()) {
      expect(tut.title).toMatch(/^help\.tutorials\.[\w-]+\.title$/);
      expect(tut.description).toMatch(/^help\.tutorials\.[\w-]+\.description$/);
      for (const step of tut.steps ?? []) {
        expect(step.title).toMatch(/^help\.tutorials\..+\.title$/);
        expect(step.content).toMatch(/^help\.tutorials\..+\.content$/);
      }
    }
    const dashboard = help.getHelpForScreen('Dashboard');
    expect(dashboard?.title).toMatch(/^help\.content\.dashboard\.title$/);
    expect(dashboard?.fullContent).toMatch(/^help\.content\.dashboard\.fullContent$/);
    // searchTutorials with identity translator returns key-bearing records.
    expect(help.searchTutorials('', t).length).toBeGreaterThan(0);
  });
});

describe('HelpCenter localized search scoring', () => {
  const help = new HelpCenter();
  const t = makeTranslator(en as unknown as Record<string, unknown>);

  it('ranks the password FAQ first when searching localized answer text', () => {
    const results = help.searchFAQs('password', t);
    expect(results.length).toBeGreaterThan(0);
    expect(results[0].id).toBe('faq-8');
  });

  it('matches FAQs against the resolved localized question text', () => {
    const results = help.searchFAQs('two-factor authentication', t);
    expect(results.map((r) => r.id)).toContain('faq-10');
  });

  it('filters tutorials by resolved localized title/description', () => {
    const results = help.searchTutorials('report', t);
    expect(results.map((r) => r.id)).toContain('tutorial-faults');
  });
});
