/// <reference types="vitest/globals" />
/**
 * VoteCard question-count label (round-3 audit #30). GET /votes returns a
 * VoteSummary without `question_count`, which rendered the raw i18n key
 * `voting.card.questionCount` on every card.
 */
import type { VoteSummary } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import { VoteCard } from './VoteCard';

const base = {
  id: 'v-1',
  building_id: 'b-1',
  title: 'Roof repair',
  status: 'active',
  end_at: '2026-12-01T00:00:00Z',
  quorum_type: 'simple_majority',
  participation_count: 3,
  eligible_count: 10,
  quorum_met: false,
} as unknown as VoteSummary;

describe('VoteCard', () => {
  it('hides the question count when the API does not send it', () => {
    render(<VoteCard vote={base} onOpen={vi.fn()} />);
    expect(screen.getByText('Roof repair')).toBeInTheDocument();
    expect(screen.queryByText(/voting\.card\.questionCount/)).not.toBeInTheDocument();
    expect(screen.queryByText(/question/i)).not.toBeInTheDocument();
  });

  it('shows the pluralised question count when it is present', () => {
    render(<VoteCard vote={{ ...base, question_count: 3 } as VoteSummary} onOpen={vi.fn()} />);
    expect(screen.getByText('3 questions')).toBeInTheDocument();
  });
});
