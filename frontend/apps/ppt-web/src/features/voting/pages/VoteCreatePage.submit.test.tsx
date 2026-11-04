/// <reference types="vitest/globals" />
/**
 * VoteCreatePage submit flow (round-3 audit #6).
 *
 * - `POST /voting/{id}/questions` parses option ids as UUIDs; the yes/no
 *   options used to go out with the ids "yes"/"no" and got a 422.
 * - When adding a question fails, the freshly created draft vote must be
 *   deleted so no empty orphan vote is left in the list.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { ToastProvider } from '../../../components';
import { toQuestionPayload, VoteCreatePage, yesNoOptions } from './VoteCreatePage';

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

const h = vi.hoisted(() => ({
  createVote: vi.fn(),
  deleteVote: vi.fn(),
  addQuestion: vi.fn(),
  publishVote: vi.fn(),
}));

vi.mock('@ppt/api-client', () => ({
  useBuildings: () => ({ data: { items: [{ id: 'b1', name: 'Building 1' }] } }),
  useCreateVote: () => ({ mutateAsync: h.createVote, isPending: false }),
  useDeleteVote: () => ({ mutateAsync: h.deleteVote, isPending: false }),
  addQuestion: h.addQuestion,
  publishVote: h.publishVote,
}));

function fillAndSaveDraft() {
  const { container } = render(
    <ToastProvider>
      <VoteCreatePage onCreated={onCreated} onCancel={() => {}} />
    </ToastProvider>
  );
  const textboxes = screen.getAllByRole('textbox');
  // [0] title, [1] description, [2] first question text
  fireEvent.change(textboxes[0], { target: { value: 'Roof repair' } });
  fireEvent.change(textboxes[2], { target: { value: 'Approve the budget?' } });
  const endAt = container.querySelector('input[type="datetime-local"]') as HTMLInputElement;
  fireEvent.change(endAt, { target: { value: '2030-01-01T12:00' } });
  const buttons = screen.getAllByRole('button');
  // The last two buttons are "save draft" and "publish".
  fireEvent.click(buttons[buttons.length - 2]);
}

const onCreated = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
  h.createVote.mockResolvedValue({ id: 'vote-1' });
  h.deleteVote.mockResolvedValue(undefined);
  h.publishVote.mockResolvedValue({});
});

describe('yes/no option ids', () => {
  it('are UUIDs', () => {
    for (const option of yesNoOptions()) {
      expect(option.id).toMatch(UUID_RE);
    }
  });

  it('go out as UUIDs in the question payload', () => {
    const payload = toQuestionPayload(
      {
        key: 'k',
        question_text: '  Approve?  ',
        question_type: 'yes_no',
        options: yesNoOptions(),
        is_required: true,
      },
      0
    );
    expect(payload.question_text).toBe('Approve?');
    expect(payload.options).toHaveLength(2);
    for (const option of payload.options) {
      expect(option.id).toMatch(UUID_RE);
    }
  });
});

describe('VoteCreatePage submit', () => {
  it('sends UUID option ids and does not delete the draft on success', async () => {
    h.addQuestion.mockResolvedValue({ id: 'q1' });
    fillAndSaveDraft();

    await waitFor(() => expect(onCreated).toHaveBeenCalledWith('vote-1'));
    expect(h.addQuestion).toHaveBeenCalledTimes(1);
    const [voteId, payload] = h.addQuestion.mock.calls[0];
    expect(voteId).toBe('vote-1');
    for (const option of payload.options) {
      expect(option.id).toMatch(UUID_RE);
    }
    expect(h.deleteVote).not.toHaveBeenCalled();
  });

  it('deletes the draft vote when adding a question fails', async () => {
    h.addQuestion.mockRejectedValue(new Error('HTTP 422'));
    fillAndSaveDraft();

    await waitFor(() => expect(h.deleteVote).toHaveBeenCalledWith('vote-1'));
    expect(onCreated).not.toHaveBeenCalled();
    expect(h.publishVote).not.toHaveBeenCalled();
  });
});
