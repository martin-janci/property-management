/**
 * Vote create page (Epic 5, screen `ppt/vote-create`).
 *
 * A sectioned form covering the create wizard's substance: topic, ballot
 * questions (with per-type option builders), quorum + schedule. On submit it
 * creates the draft vote, attaches each question, and — if the manager chose to
 * publish — publishes it, then hands the new vote id back to the route wrapper.
 */
import {
  type AddQuestionRequest,
  addQuestion,
  publishVote,
  type QuestionType,
  type QuorumType,
  useBuildings,
  useCreateVote,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../../../components';
import { QUESTION_TYPE_LABELS, QUORUM_TYPE_LABELS } from '../types';

interface VoteCreatePageProps {
  onCreated: (voteId: string) => void;
  onCancel: () => void;
}

interface DraftQuestion {
  key: string;
  question_text: string;
  question_type: QuestionType;
  options: Array<{ id: string; text: string }>;
  is_required: boolean;
}

const QUESTION_TYPES: QuestionType[] = ['yes_no', 'single_choice', 'multiple_choice', 'ranked'];
const QUORUM_TYPES: QuorumType[] = ['simple_majority', 'two_thirds', 'weighted'];

function newId(): string {
  // Browser-native; available in all supported targets.
  return crypto.randomUUID();
}

function yesNoOptions() {
  return [
    { id: 'yes', text: 'Yes' },
    { id: 'no', text: 'No' },
  ];
}

function blankQuestion(): DraftQuestion {
  return {
    key: newId(),
    question_text: '',
    question_type: 'yes_no',
    options: yesNoOptions(),
    is_required: true,
  };
}

export function VoteCreatePage({ onCreated, onCancel }: VoteCreatePageProps) {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { data: buildingsData } = useBuildings();
  const buildingId = buildingsData?.items?.[0]?.id ?? '';
  const createVote = useCreateVote();

  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [endAt, setEndAt] = useState('');
  const [quorumType, setQuorumType] = useState<QuorumType>('simple_majority');
  const [quorumPercentage, setQuorumPercentage] = useState('');
  const [questions, setQuestions] = useState<DraftQuestion[]>([blankQuestion()]);
  const [submitting, setSubmitting] = useState(false);

  const updateQuestion = (key: string, patch: Partial<DraftQuestion>) => {
    setQuestions((qs) => qs.map((q) => (q.key === key ? { ...q, ...patch } : q)));
  };

  const setType = (key: string, type: QuestionType) => {
    setQuestions((qs) =>
      qs.map((q) => {
        if (q.key !== key) return q;
        if (type === 'yes_no') return { ...q, question_type: type, options: yesNoOptions() };
        // Seed two empty options for choice/ranked types.
        const seeded =
          q.options.length >= 2 && q.question_type !== 'yes_no'
            ? q.options
            : [
                { id: newId(), text: '' },
                { id: newId(), text: '' },
              ];
        return { ...q, question_type: type, options: seeded };
      })
    );
  };

  const addOption = (key: string) => {
    setQuestions((qs) =>
      qs.map((q) =>
        q.key === key ? { ...q, options: [...q.options, { id: newId(), text: '' }] } : q
      )
    );
  };

  const setOptionText = (key: string, optId: string, text: string) => {
    setQuestions((qs) =>
      qs.map((q) =>
        q.key === key
          ? { ...q, options: q.options.map((o) => (o.id === optId ? { ...o, text } : o)) }
          : q
      )
    );
  };

  const removeOption = (key: string, optId: string) => {
    setQuestions((qs) =>
      qs.map((q) =>
        q.key === key ? { ...q, options: q.options.filter((o) => o.id !== optId) } : q
      )
    );
  };

  const validate = (): string | null => {
    if (!buildingId) return 'No building is available for this account.';
    if (!title.trim()) return 'A title is required.';
    if (!endAt) return 'A closing date is required.';
    if (questions.length === 0) return 'Add at least one question.';
    for (const q of questions) {
      if (!q.question_text.trim()) return 'Every question needs text.';
      if (q.question_type !== 'yes_no') {
        const filled = q.options.filter((o) => o.text.trim());
        if (filled.length < 2)
          return `"${q.question_text || 'Question'}" needs at least 2 options.`;
      }
    }
    return null;
  };

  const handleSubmit = async (publish: boolean) => {
    const err = validate();
    if (err) {
      showToast({ type: 'error', title: 'Check the form', message: err });
      return;
    }
    setSubmitting(true);
    try {
      const vote = await createVote.mutateAsync({
        building_id: buildingId,
        title: title.trim(),
        description: description.trim() || null,
        end_at: new Date(endAt).toISOString(),
        quorum_type: quorumType,
        quorum_percentage: quorumPercentage ? Number(quorumPercentage) : null,
      });

      for (let i = 0; i < questions.length; i++) {
        const q = questions[i];
        const payload: AddQuestionRequest = {
          question_text: q.question_text.trim(),
          question_type: q.question_type,
          options: q.options
            .filter((o) => o.text.trim() || q.question_type === 'yes_no')
            .map((o, idx) => ({ id: o.id, text: o.text.trim(), order: idx })),
          display_order: i,
          is_required: q.is_required,
        };
        await addQuestion(vote.id, payload);
      }

      if (publish) {
        await publishVote(vote.id, {});
      }

      showToast({
        type: 'success',
        title: publish ? 'Vote published' : 'Draft saved',
        message: publish ? 'Eligible owners can now vote.' : 'You can publish it later.',
      });
      onCreated(vote.id);
    } catch (e) {
      showToast({
        type: 'error',
        title: 'Failed to create vote',
        message: e instanceof Error ? e.message : 'Unexpected error',
      });
    } finally {
      setSubmitting(false);
    }
  };

  const inputClass =
    'mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none';

  return (
    <div className="mx-auto max-w-3xl px-4 py-8">
      <button
        type="button"
        onClick={onCancel}
        className="text-sm text-gray-500 hover:text-gray-700"
      >
        ← Back to voting
      </button>
      <h1 className="mt-2 text-2xl font-bold text-gray-900">Create vote</h1>

      {/* Topic */}
      <section className="mt-6 rounded-lg bg-white p-5 shadow">
        <h2 className="text-sm font-semibold text-gray-900">Topic</h2>
        <label className="mt-3 block text-sm font-medium text-gray-700">
          Title
          <input
            className={inputClass}
            value={title}
            maxLength={120}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="e.g. Approve façade renovation budget"
          />
        </label>
        <label className="mt-3 block text-sm font-medium text-gray-700">
          Description
          <textarea
            className={inputClass}
            rows={4}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Context owners need to make an informed decision"
          />
        </label>
      </section>

      {/* Questions */}
      <section className="mt-5 rounded-lg bg-white p-5 shadow">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-gray-900">Questions</h2>
          <button
            type="button"
            onClick={() => setQuestions((qs) => [...qs, blankQuestion()])}
            className="text-sm font-medium text-blue-600 hover:text-blue-800"
          >
            + Add question
          </button>
        </div>

        <div className="mt-4 space-y-5">
          {questions.map((q, idx) => (
            <div key={q.key} className="rounded-lg border border-gray-200 p-4">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold uppercase text-gray-400">
                  Question {idx + 1}
                </span>
                {questions.length > 1 ? (
                  <button
                    type="button"
                    onClick={() => setQuestions((qs) => qs.filter((x) => x.key !== q.key))}
                    className="text-xs text-red-500 hover:text-red-700"
                  >
                    Remove
                  </button>
                ) : null}
              </div>
              <input
                className={inputClass}
                value={q.question_text}
                onChange={(e) => updateQuestion(q.key, { question_text: e.target.value })}
                placeholder="Question text"
              />
              <div className="mt-3 flex flex-wrap gap-2">
                {QUESTION_TYPES.map((t) => (
                  <button
                    key={t}
                    type="button"
                    onClick={() => setType(q.key, t)}
                    className={`rounded-full px-3 py-1 text-xs ${
                      q.question_type === t
                        ? 'bg-blue-600 text-white'
                        : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                    }`}
                  >
                    {QUESTION_TYPE_LABELS[t]}
                  </button>
                ))}
              </div>

              {q.question_type !== 'yes_no' ? (
                <div className="mt-3 space-y-2">
                  {q.options.map((o) => (
                    <div key={o.id} className="flex items-center gap-2">
                      <input
                        className="w-full rounded-md border border-gray-300 px-3 py-1.5 text-sm focus:border-blue-500 focus:outline-none"
                        value={o.text}
                        onChange={(e) => setOptionText(q.key, o.id, e.target.value)}
                        placeholder="Option text"
                      />
                      {q.options.length > 2 ? (
                        <button
                          type="button"
                          onClick={() => removeOption(q.key, o.id)}
                          className="text-gray-400 hover:text-red-500"
                          aria-label={t('aria.removeOption')}
                        >
                          ✕
                        </button>
                      ) : null}
                    </div>
                  ))}
                  <button
                    type="button"
                    onClick={() => addOption(q.key)}
                    className="text-xs font-medium text-blue-600 hover:text-blue-800"
                  >
                    + Add option
                  </button>
                </div>
              ) : (
                <p className="mt-3 text-xs text-gray-400">Options: Yes / No</p>
              )}

              <label className="mt-3 flex items-center gap-2 text-xs text-gray-600">
                <input
                  type="checkbox"
                  checked={q.is_required}
                  onChange={(e) => updateQuestion(q.key, { is_required: e.target.checked })}
                />
                Required
              </label>
            </div>
          ))}
        </div>
      </section>

      {/* Quorum + schedule */}
      <section className="mt-5 rounded-lg bg-white p-5 shadow">
        <h2 className="text-sm font-semibold text-gray-900">Quorum &amp; schedule</h2>
        <label className="mt-3 block text-sm font-medium text-gray-700">
          Quorum rule
          <select
            className={inputClass}
            value={quorumType}
            onChange={(e) => setQuorumType(e.target.value as QuorumType)}
          >
            {QUORUM_TYPES.map((t) => (
              <option key={t} value={t}>
                {QUORUM_TYPE_LABELS[t]}
              </option>
            ))}
          </select>
        </label>
        <label className="mt-3 block text-sm font-medium text-gray-700">
          Quorum percentage (optional)
          <input
            type="number"
            min={0}
            max={100}
            className={inputClass}
            value={quorumPercentage}
            onChange={(e) => setQuorumPercentage(e.target.value)}
            placeholder="e.g. 50"
          />
        </label>
        <label className="mt-3 block text-sm font-medium text-gray-700">
          Closes at
          <input
            type="datetime-local"
            className={inputClass}
            value={endAt}
            onChange={(e) => setEndAt(e.target.value)}
          />
        </label>
      </section>

      <div className="mt-6 flex items-center justify-end gap-3">
        <button
          type="button"
          onClick={onCancel}
          className="rounded-md px-4 py-2 text-sm text-gray-600 hover:text-gray-800"
        >
          Cancel
        </button>
        <button
          type="button"
          disabled={submitting}
          onClick={() => handleSubmit(false)}
          className="rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
        >
          Save draft
        </button>
        <button
          type="button"
          disabled={submitting}
          onClick={() => handleSubmit(true)}
          className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {submitting ? 'Saving…' : 'Publish vote'}
        </button>
      </div>
    </div>
  );
}
