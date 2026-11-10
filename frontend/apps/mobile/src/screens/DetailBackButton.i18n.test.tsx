/**
 * ArticleDetail / OutageDetail back-button i18n regression test
 * (code-review-mobile-rn-detail-back-button-hardcoded-en).
 *
 * The back buttons rendered the literals '‹ News' / '‹ Outages' while the rest
 * of each screen resolved through t(). `src/test/setup.ts` mocks react-i18next
 * so `t(key) => key`; we assert the keys render (not the literals) and that
 * every locale carries a translated value.
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../locales/cs.json';
import de from '../locales/de.json';
import en from '../locales/en.json';
import hu from '../locales/hu.json';
import pl from '../locales/pl.json';
import sk from '../locales/sk.json';
import { ArticleDetailScreen } from './news/ArticleDetailScreen';
import { OutageDetailScreen } from './outages/OutageDetailScreen';

jest.mock('../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../hooks/useApi').useApiQuery as jest.Mock;

describe('detail screens back-button i18n', () => {
  beforeEach(() => {
    mockUseApiQuery.mockReturnValue({ data: undefined, isLoading: true, error: null });
  });

  it('ArticleDetailScreen resolves the back label via t()', () => {
    render(<ArticleDetailScreen articleId="a1" onBack={jest.fn()} />);
    expect(screen.getByText('news.backToList')).toBeTruthy();
    expect(screen.queryByText('‹ News')).toBeNull();
  });

  it('OutageDetailScreen resolves the back label via t()', () => {
    render(<OutageDetailScreen outageId="o1" onBack={jest.fn()} />);
    expect(screen.getByText('outages.backToList')).toBeTruthy();
    expect(screen.queryByText('‹ Outages')).toBeNull();
  });

  it('every locale defines both back labels', () => {
    for (const locale of [en, sk, cs, de, hu, pl]) {
      expect(locale.news.backToList.length).toBeGreaterThan(0);
      expect(locale.outages.backToList.length).toBeGreaterThan(0);
    }
    expect(en.news.backToList).toBe('‹ News');
    expect(en.outages.backToList).toBe('‹ Outages');
  });
});
