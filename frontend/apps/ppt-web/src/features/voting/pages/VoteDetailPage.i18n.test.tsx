/// <reference types="vitest/globals" />
/**
 * VoteDetailPage i18n regression test.
 *
 * Guards against the page hard-coding English user-facing copy (the state it
 * shipped in before the i18n pass). The Slovak case in particular would fail
 * on the pre-fix implementation, which rendered literal English regardless of
 * the active locale.
 */
import type { VoteWithDetails } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import i18n from 'i18next';
import sk from '../../../../messages/sk.json';
import { ToastProvider } from '../../../components';
import { VoteDetailPage } from './VoteDetailPage';

const h = vi.hoisted(() => ({
  state: {
    vote: { data: undefined as unknown, isLoading: false, error: null as unknown },
    eligibility: { data: undefined as unknown },
    results: { data: undefined as unknown },
    comments: { data: [] as unknown[] },
  },
  mutation: () => ({ mutateAsync: () => Promise.resolve(undefined), isPending: false }),
}));

vi.mock('@ppt/api-client', () => ({
  useVote: () => h.state.vote,
  useVoteEligibility: () => h.state.eligibility,
  useVoteResults: () => h.state.results,
  useVoteComments: () => h.state.comments,
  useCastVote: () => h.mutation(),
  usePublishVote: () => h.mutation(),
  useCloseVote: () => h.mutation(),
  useCancelVote: () => h.mutation(),
  useAddVoteComment: () => h.mutation(),
}));

function renderPage() {
  return render(
    <ToastProvider>
      <VoteDetailPage voteId="v1" onBack={() => {}} />
    </ToastProvider>
  );
}

beforeEach(() => {
  h.state.vote = { data: undefined, isLoading: false, error: null };
  h.state.eligibility = { data: undefined };
  h.state.results = { data: undefined };
  h.state.comments = { data: [] };
});

afterEach(async () => {
  await i18n.changeLanguage('en');
});

describe('VoteDetailPage i18n', () => {
  it('renders the unavailable state with translated copy (Slovak)', async () => {
    i18n.addResourceBundle('sk', 'translation', sk, true, true);
    await i18n.changeLanguage('sk');
    h.state.vote = { data: undefined, isLoading: false, error: new Error('nope') };

    renderPage();

    expect(screen.getByRole('heading', { name: 'Hlasovanie nie je dostupné' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Späť na hlasovanie' })).toBeInTheDocument();
  });

  it('renders manager actions and discussion copy from i18n (English)', () => {
    h.state.vote = {
      data: {
        id: 'v1',
        status: 'draft',
        title: 'Roof repair',
        description: null,
        quorum_type: 'simple_majority',
        end_at: '2026-09-01T10:00:00Z',
        participation_count: 0,
        eligible_count: 10,
        quorum_met: false,
        questions: [],
      } as unknown as VoteWithDetails,
      isLoading: false,
      error: null,
    };

    renderPage();

    expect(screen.getByRole('button', { name: 'Publish now' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel vote' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Discussion' })).toBeInTheDocument();
    expect(screen.getByText('No comments yet.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Post' })).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Add a comment')).toBeInTheDocument();
  });
});
