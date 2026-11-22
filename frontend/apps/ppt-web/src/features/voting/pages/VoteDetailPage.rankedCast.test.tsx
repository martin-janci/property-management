/// <reference types="vitest/globals" />
/**
 * VoteDetailPage ranked-ballot cast regression test.
 *
 * Closes out the carried "wire-the-button-but-not-the-POST" pattern for the
 * ranked ballot: the `Ranked` input renders a default ordering (the declared
 * option order) before the voter touches the reorder controls, but that visible
 * default was never written into the `answers` state. A voter who accepted the
 * shown ranking and pressed "Submit ballot" therefore either hit the
 * "answer required" guard (required question — the ballot looked complete yet
 * refused to submit) or silently POSTed without the ranking (optional question).
 *
 * This test drives the real submit path with an untouched ranked question and
 * asserts the cast POST carries the displayed default ranking. It fails on the
 * pre-fix implementation (castVote is never called because the required ranked
 * answer reads as missing).
 */
import type { VoteWithDetails } from '@ppt/api-client';
import { fireEvent, render, screen } from '@testing-library/react';
import i18n from 'i18next';
import { ToastProvider } from '../../../components';
import { VoteDetailPage } from './VoteDetailPage';

const castSpy = vi.fn(() => Promise.resolve(undefined));

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
  useCastVote: () => ({ mutateAsync: castSpy, isPending: false }),
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
  castSpy.mockClear();
  castSpy.mockResolvedValue(undefined);
  h.state.vote = {
    data: {
      id: 'v1',
      status: 'active',
      title: 'Board seat ranking',
      description: null,
      quorum_type: 'simple_majority',
      end_at: '2026-12-01T10:00:00Z',
      participation_count: 0,
      eligible_count: 10,
      quorum_met: false,
      questions: [
        {
          id: 'q1',
          question_text: 'Rank the candidates',
          question_type: 'ranked',
          is_required: true,
          description: null,
          options: [
            { id: 'opt-a', text: 'Alice' },
            { id: 'opt-b', text: 'Bob' },
            { id: 'opt-c', text: 'Carol' },
          ],
        },
      ],
    } as unknown as VoteWithDetails,
    isLoading: false,
    error: null,
  };
  h.state.eligibility = {
    data: {
      can_vote: true,
      reason: null,
      eligible_units: [{ unit_id: 'u1', unit_designation: 'Unit 1', already_voted: false }],
    },
  };
  h.state.results = { data: undefined };
  h.state.comments = { data: [] };
});

afterEach(async () => {
  await i18n.changeLanguage('en');
});

describe('VoteDetailPage ranked cast', () => {
  it('POSTs the displayed default ranking when the voter does not reorder', () => {
    renderPage();

    fireEvent.click(screen.getByRole('button', { name: 'Submit ballot' }));

    expect(castSpy).toHaveBeenCalledTimes(1);
    expect(castSpy).toHaveBeenCalledWith({
      unit_id: 'u1',
      answers: { q1: ['opt-a', 'opt-b', 'opt-c'] },
    });
  });
});
