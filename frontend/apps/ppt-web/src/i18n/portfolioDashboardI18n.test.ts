/// <reference types="vitest/globals" />
/**
 * Portfolio Dashboard (Epic 144 — Portfolio Performance Analytics) must be
 * translated in all six shipped locales. The page
 * (`features/portfolio-performance/pages/PortfolioDashboardPage.tsx`) routes
 * every user-facing string through `t('portfolio.dashboard.*')`; this test
 * pins each of those keys to a non-empty string in every bundle so a dropped
 * translation (or a merge-train bundle drop) fails CI instead of silently
 * falling back to the key or to English.
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

// Every key read by PortfolioDashboardPage.tsx (incl. the inline SummaryCards).
const KEYS = [
  'title',
  'meta',
  'exportReport',
  'exportReportUnavailable',
  'calculateMetrics',
  'calculating',
  'loading',
  'loadError',
  'loadErrorDetail',
  'retry',
  'alerts',
  'cashFlowLoading',
  'cashFlowError',
  'propertyPerformance',
  'viewAllProperties',
  'viewAllPropertiesUnavailable',
  'propertiesLoading',
  'propertiesError',
  'propertiesEmpty',
  'metricsError',
  'metricsTableTitle',
  'debtToEquity',
  'cards.portfolioValue',
  'cards.totalEquity',
  'cards.ytdNoi',
  'cards.ytdCashFlow',
  'cards.ltvRatio',
  'cards.ytdReturn',
  'toast.metricsRecalculated',
  'toast.metricsRecalculatedDetail',
  'toast.calculateFailed',
  'toast.markReadFailed',
  'toast.resolveFailed',
].map((k) => `portfolio.dashboard.${k}`);

// Keys that carry interpolation placeholders the page relies on.
const INTERPOLATED: Record<string, string[]> = {
  'portfolio.dashboard.meta': ['{{portfolioId}}', '{{asOfDate}}'],
  'portfolio.dashboard.debtToEquity': ['{{ratio}}'],
};

describe('portfolio dashboard i18n', () => {
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
    expect(getPath(sk, 'portfolio.dashboard.title')).toBe('Prehľad portfólia');
    expect(getPath(sk, 'portfolio.dashboard.retry')).toBe('Skúsiť znova');
  });
});
