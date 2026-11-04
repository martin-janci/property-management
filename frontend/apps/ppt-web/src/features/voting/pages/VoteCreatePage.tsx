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
  useDeleteVote,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../../../components';
import { QUESTION_TYPE_LABEL_KEYS, QUORUM_TYPE_LABEL_KEYS } from '../types';

interface VoteCreatePageProps {
  onCreated: (voteId: string) => void;
  onCancel: () => void;
}

export interface DraftQuestion {
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

// Persisted option payload for yes/no questions (stored server-side as data,
// not rendered as UI copy — the ballot renders its own localized Yes/No).
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

const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/**
 * Build the `POST /voting/{id}/questions` body for a draft question. The
 * server parses every option id as a UUID and rejects anything else with 422
 * (the yes/no options carry the local ids "yes"/"no"), so non-UUID ids are
 * replaced with fresh UUIDs here.
 */
export function toQuestionPayload(q: DraftQuestion, index: number): AddQuestionRequest {
  return {
    question_text: q.question_text.trim(),
    question_type: q.question_type,
    options: q.options
      .filter((o) => o.text.trim() || q.question_type === 'yes_no')
      .map((o, idx) => ({
        id: UUID_PATTERN.test(o.id) ? o.id : newId(),
        text: o.text.trim(),
        order: idx,
      })),
    display_order: index,
    is_required: q.is_required,
  };
}

export function VoteCreatePage({ onCreated, onCancel }: VoteCreatePageProps) {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { data: buildingsData } = useBuildings();
  const buildingId = buildingsData?.items?.[0]?.id ?? '';
  const createVote = useCreateVote();
  const discardDraft = useDeleteVote();

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
    if (!buildingId) return t('voting.create.errors.noBuilding');
    if (!title.trim()) return t('voting.create.errors.titleRequired');
    if (!endAt) return t('voting.create.errors.endAtRequired');
    if (questions.length === 0) return t('voting.create.errors.noQuestions');
    for (const q of questions) {
      if (!q.question_text.trim()) return t('voting.create.errors.questionTextRequired');
      if (q.question_type !== 'yes_no') {
        const filled = q.options.filter((o) => o.text.trim());
        if (filled.length < 2)
          return t('voting.create.errors.minOptions', { question: q.question_text });
      }
    }
    return null;
  };

  const handleSubmit = async (publish: boolean) => {
    const err = validate();
    if (err) {
      showToast({ type: 'error', title: t('voting.create.checkForm'), message: err });
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

      try {
        for (let i = 0; i < questions.length; i++) {
          await addQuestion(vote.id, toQuestionPayload(questions[i], i));
        }
      } catch (questionError) {
        // Don't leave an empty draft behind when a question is rejected.
        await discardDraft.mutateAsync(vote.id).catch(() => undefined);
        throw questionError;
      }

      if (publish) {
        await publishVote(vote.id, {});
      }

      showToast({
        type: 'success',
        title: publish ? t('voting.detail.publishDone') : t('voting.create.draftSaved'),
        message: publish ? t('voting.create.publishedBody') : t('voting.create.draftSavedBody'),
      });
      onCreated(vote.id);
    } catch (e) {
      showToast({
        type: 'error',
        title: t('voting.create.createFailed'),
        message: e instanceof Error ? e.message : t('voting.detail.unexpectedError'),
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
        ← {t('voting.detail.backToVoting')}
      </button>
      <h1 className="mt-2 text-2xl font-bold text-gray-900">{t('voting.create.title')}</h1>

      {/* Topic */}
      <section className="mt-6 rounded-lg bg-white p-5 shadow">
        <h2 className="text-sm font-semibold text-gray-900">{t('voting.create.topic')}</h2>
        <label className="mt-3 block text-sm font-medium text-gray-700">
          {t('voting.create.titleLabel')}
          <input
            className={inputClass}
            value={title}
            maxLength={120}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={t('voting.create.titlePlaceholder')}
          />
        </label>
        <label className="mt-3 block text-sm font-medium text-gray-700">
          {t('voting.detail.description')}
          <textarea
            className={inputClass}
            rows={4}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder={t('voting.create.descriptionPlaceholder')}
          />
        </label>
      </section>

      {/* Questions */}
      <section className="mt-5 rounded-lg bg-white p-5 shadow">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-gray-900">{t('voting.create.questions')}</h2>
          <button
            type="button"
            onClick={() => setQuestions((qs) => [...qs, blankQuestion()])}
            className="text-sm font-medium text-blue-600 hover:text-blue-800"
          >
            + {t('voting.create.addQuestion')}
          </button>
        </div>

        <div className="mt-4 space-y-5">
          {questions.map((q, idx) => (
            <div key={q.key} className="rounded-lg border border-gray-200 p-4">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold uppercase text-gray-400">
                  {t('voting.create.questionNumber', { number: idx + 1 })}
                </span>
                {questions.length > 1 ? (
                  <button
                    type="button"
                    onClick={() => setQuestions((qs) => qs.filter((x) => x.key !== q.key))}
                    className="text-xs text-red-500 hover:text-red-700"
                  >
                    {t('voting.create.remove')}
                  </button>
                ) : null}
              </div>
              <input
                className={inputClass}
                value={q.question_text}
                onChange={(e) => updateQuestion(q.key, { question_text: e.target.value })}
                placeholder={t('voting.create.questionTextPlaceholder')}
              />
              <div className="mt-3 flex flex-wrap gap-2">
                {QUESTION_TYPES.map((type) => (
                  <button
                    key={type}
                    type="button"
                    onClick={() => setType(q.key, type)}
                    className={`rounded-full px-3 py-1 text-xs ${
                      q.question_type === type
                        ? 'bg-blue-600 text-white'
                        : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                    }`}
                  >
                    {t(QUESTION_TYPE_LABEL_KEYS[type])}
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
                        placeholder={t('voting.create.optionTextPlaceholder')}
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
                    + {t('voting.create.addOption')}
                  </button>
                </div>
              ) : (
                <p className="mt-3 text-xs text-gray-400">{t('voting.create.yesNoOptions')}</p>
              )}

              <label className="mt-3 flex items-center gap-2 text-xs text-gray-600">
                <input
                  type="checkbox"
                  checked={q.is_required}
                  onChange={(e) => updateQuestion(q.key, { is_required: e.target.checked })}
                />
                {t('voting.create.required')}
              </label>
            </div>
          ))}
        </div>
      </section>

      {/* Quorum + schedule */}
      <section className="mt-5 rounded-lg bg-white p-5 shadow">
        <h2 className="text-sm font-semibold text-gray-900">
          {t('voting.create.quorumAndSchedule')}
        </h2>
        <label className="mt-3 block text-sm font-medium text-gray-700">
          {t('voting.create.quorumRule')}
          <select
            className={inputClass}
            value={quorumType}
            onChange={(e) => setQuorumType(e.target.value as QuorumType)}
          >
            {QUORUM_TYPES.map((type) => (
              <option key={type} value={type}>
                {t(QUORUM_TYPE_LABEL_KEYS[type])}
              </option>
            ))}
          </select>
        </label>
        <label className="mt-3 block text-sm font-medium text-gray-700">
          {t('voting.create.quorumPercentage')}
          <input
            type="number"
            min={0}
            max={100}
            className={inputClass}
            value={quorumPercentage}
            onChange={(e) => setQuorumPercentage(e.target.value)}
            placeholder={t('voting.create.quorumPercentagePlaceholder')}
          />
        </label>
        <label className="mt-3 block text-sm font-medium text-gray-700">
          {t('voting.create.closesAt')}
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
          {t('common.cancel')}
        </button>
        <button
          type="button"
          disabled={submitting}
          onClick={() => handleSubmit(false)}
          className="rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
        >
          {t('voting.create.saveDraft')}
        </button>
        <button
          type="button"
          disabled={submitting}
          onClick={() => handleSubmit(true)}
          className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {submitting ? t('voting.create.saving') : t('voting.create.publishVote')}
        </button>
      </div>
    </div>
  );
}
