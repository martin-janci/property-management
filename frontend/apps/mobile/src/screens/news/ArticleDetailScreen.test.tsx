/**
 * ArticleDetailScreen (UC-13) — state + data-contract characterization tests.
 *
 * The back-button i18n and the App-level navigation wiring are already locked
 * by `DetailBackButton.i18n.test.tsx` and `App.outageNewsDetail.test.tsx`.
 * This suite covers the screen's own render branches and the one invariant the
 * file comment calls out but nothing enforced: a missing `articleId` must NOT
 * fire a `GET /api/v1/news/undefined` request (the query is gated on
 * `enabled: !!articleId`).
 *
 * `useApiQuery` is mocked so the branches can be driven directly, and
 * react-i18next is mocked in `src/test/setup.ts` (`t(key) => key`), so the
 * assertions check for the i18n keys rather than translated copy.
 */

import { render, screen } from '@testing-library/react-native';
import { ArticleDetailScreen, articleDetailKey } from './ArticleDetailScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

function mockQuery(state: { data?: unknown; isLoading?: boolean; error?: unknown }) {
  mockUseApiQuery.mockReturnValue({
    data: state.data,
    isLoading: state.isLoading ?? false,
    error: state.error ?? null,
  });
}

describe('articleDetailKey', () => {
  it('keys the cache on the article id', () => {
    expect(articleDetailKey('a-42')).toEqual(['news', 'detail', 'a-42']);
  });
});

describe('ArticleDetailScreen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockQuery({ data: undefined, isLoading: false, error: null });
  });

  describe('missing articleId', () => {
    it('renders the empty state and the back button', () => {
      render(<ArticleDetailScreen onBack={jest.fn()} />);
      expect(screen.getByText('news.emptyTitle')).toBeTruthy();
      expect(screen.getByText('news.backToList')).toBeTruthy();
    });

    it('gates the query so no /api/v1/news/undefined request fires', () => {
      render(<ArticleDetailScreen onBack={jest.fn()} />);
      expect(mockUseApiQuery).toHaveBeenCalledWith(
        ['news', 'detail', ''],
        expect.any(String),
        expect.objectContaining({ enabled: false })
      );
    });
  });

  describe('with an articleId', () => {
    it('fetches /api/v1/news/{id} with an enabled, id-keyed query', () => {
      mockQuery({ data: undefined, isLoading: true });
      render(<ArticleDetailScreen articleId="a-1" onBack={jest.fn()} />);
      expect(mockUseApiQuery).toHaveBeenCalledWith(
        ['news', 'detail', 'a-1'],
        '/api/v1/news/a-1',
        expect.objectContaining({ enabled: true })
      );
    });

    it('shows the loading indicator while fetching', () => {
      mockQuery({ data: undefined, isLoading: true });
      render(<ArticleDetailScreen articleId="a-1" onBack={jest.fn()} />);
      expect(screen.getByText('common.loading')).toBeTruthy();
    });

    it('shows the error state when the request fails', () => {
      mockQuery({ data: undefined, isLoading: false, error: new Error('boom') });
      render(<ArticleDetailScreen articleId="a-1" onBack={jest.fn()} />);
      expect(screen.getByText('news.loadError')).toBeTruthy();
    });

    it('shows the error state when data is missing even without an error', () => {
      mockQuery({ data: undefined, isLoading: false, error: null });
      render(<ArticleDetailScreen articleId="a-1" onBack={jest.fn()} />);
      expect(screen.getByText('news.loadError')).toBeTruthy();
    });

    it('renders the article title, category badge and body on success', () => {
      mockQuery({
        data: {
          id: 'a-1',
          title: 'Lift maintenance scheduled',
          category: 'building',
          body: 'The lift will be out of service on Tuesday.',
          created_at: '2026-01-02T10:00:00Z',
        },
      });
      render(<ArticleDetailScreen articleId="a-1" onBack={jest.fn()} />);
      expect(screen.getByText('Lift maintenance scheduled')).toBeTruthy();
      // The global `t:(key)=>key` stub (src/test/setup.ts) ignores the badge's
      // `defaultValue`, so the rendered label is the i18n KEY — assert on that,
      // consistent with this file's key-based approach (see header comment).
      expect(screen.getByText('news.categories.building')).toBeTruthy();
      expect(screen.getByText('The lift will be out of service on Tuesday.')).toBeTruthy();
    });

    it('falls back excerpt -> summary when body is absent', () => {
      mockQuery({
        data: {
          id: 'a-2',
          title: 'No body here',
          body: null,
          excerpt: 'Short excerpt stands in for the body.',
          summary: 'A summary that should be ignored when an excerpt exists.',
          created_at: '2026-01-02T10:00:00Z',
        },
      });
      render(<ArticleDetailScreen articleId="a-2" onBack={jest.fn()} />);
      expect(screen.getByText('Short excerpt stands in for the body.')).toBeTruthy();
      expect(
        screen.queryByText('A summary that should be ignored when an excerpt exists.')
      ).toBeNull();
    });

    it('falls back to summary when both body and excerpt are absent', () => {
      mockQuery({
        data: {
          id: 'a-3',
          title: 'Summary only',
          body: null,
          excerpt: null,
          summary: 'Only the summary is available.',
          created_at: '2026-01-02T10:00:00Z',
        },
      });
      render(<ArticleDetailScreen articleId="a-3" onBack={jest.fn()} />);
      expect(screen.getByText('Only the summary is available.')).toBeTruthy();
    });

    it('renders without a category badge or body when the article is minimal', () => {
      mockQuery({
        data: {
          id: 'a-4',
          title: 'Bare article',
          created_at: '2026-01-02T10:00:00Z',
        },
      });
      render(<ArticleDetailScreen articleId="a-4" onBack={jest.fn()} />);
      expect(screen.getByText('Bare article')).toBeTruthy();
      // Back button is always present; this confirms the success branch
      // rendered rather than the error/empty branch.
      expect(screen.getByText('news.backToList')).toBeTruthy();
    });
  });
});
