/// <reference types="vitest/globals" />
/**
 * Portfolio Performance Alert (Epic 144 — Portfolio Performance Analytics) must
 * be translated in all six shipped locales. The component
 * (`features/portfolio-performance/components/PerformanceAlert.tsx`) is rendered
 * by the already-localized PortfolioDashboardPage but previously emitted
 * hardcoded English chrome ("Resolved", "Mark Read", "Resolve", "Metric:",
 * "Current:", "Threshold:"). It now routes every user-facing string through
 * `t('portfolio.alert.*')`; this test pins each of those keys to a non-empty
 * string in every bundle so a dropped translation (or a merge-train bundle drop)
 * fails CI instead of silently falling back to the key or to English.
 */
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;
const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

function getPath(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc === null || typeof acc !== 'object') return undefined;
    return (acc as Json)[part];
  }, obj);
}

// Every key read by PerformanceAlert.tsx.
const KEYS = ['resolved', 'markRead', 'resolve', 'metric', 'current', 'threshold'].map(
  (k) => `portfolio.alert.${k}`
);

// Keys that carry interpolation placeholders the component relies on.
const INTERPOLATED: Record<string, string[]> = {
  'portfolio.alert.metric': ['{{name}}'],
  'portfolio.alert.current': ['{{value}}'],
  'portfolio.alert.threshold': ['{{value}}'],
};

describe('portfolio alert i18n', () => {
  for (const [lang, bundle] of Object.entries(BUNDLES)) {
    it(`${lang} has every key`, () => {
      const missing = KEYS.filter((key) => {
        const v = getPath(bundle, key);
        return typeof v !== 'string' || v.trim() === '';
      });
      expect(missing).toEqual([]);
    });

    it(`${lang} preserves interpolation placeholders`, () => {
      const broken = Object.entries(INTERPOLATED).flatMap(([key, tokens]) => {
        const v = getPath(bundle, key);
        if (typeof v !== 'string') return [key];
        return tokens.filter((tok) => !v.includes(tok)).map((tok) => `${key} missing ${tok}`);
      });
      expect(broken).toEqual([]);
    });
  }

  it('uses the agreed Slovak terminology', () => {
    expect(getPath(sk, 'portfolio.alert.resolved')).toBe('Vyriešené');
    expect(getPath(sk, 'portfolio.alert.resolve')).toBe('Vyriešiť');
  });
});
