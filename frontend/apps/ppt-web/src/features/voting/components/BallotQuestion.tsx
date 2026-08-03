/**
 * Ballot input for a single question, rendered via a discriminated switch on
 * `question.question_type` (Epic 5, vote-detail screen-map). Controlled: parent
 * owns the `AnswerValue` and receives changes through `onChange`.
 */
import type { AnswerValue, VoteQuestion } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface BallotQuestionProps {
  question: VoteQuestion;
  value: AnswerValue | undefined;
  onChange: (value: AnswerValue) => void;
  disabled?: boolean;
}

export function BallotQuestion({ question, value, onChange, disabled }: BallotQuestionProps) {
  return (
    <fieldset className="rounded-lg border border-gray-200 p-4" disabled={disabled}>
      <legend className="px-1 text-sm font-semibold text-gray-900">
        {question.question_text}
        {question.is_required ? <span className="ml-1 text-red-500">*</span> : null}
      </legend>
      {question.description ? (
        <p className="mb-2 text-sm text-gray-500">{question.description}</p>
      ) : null}
      <div className="mt-2 space-y-2">{renderInput(question, value, onChange)}</div>
    </fieldset>
  );
}

function renderInput(
  question: VoteQuestion,
  value: AnswerValue | undefined,
  onChange: (value: AnswerValue) => void
) {
  switch (question.question_type) {
    case 'yes_no':
      return <YesNo value={value} onChange={onChange} />;
    case 'single_choice':
      return <SingleChoice question={question} value={value} onChange={onChange} />;
    case 'multiple_choice':
      return <MultipleChoice question={question} value={value} onChange={onChange} />;
    case 'ranked':
      return <Ranked question={question} value={value} onChange={onChange} />;
    default:
      return null;
  }
}

const optionRow =
  'flex cursor-pointer items-center gap-3 rounded-md border border-gray-200 px-3 py-2 text-sm hover:bg-gray-50';

function YesNo({
  value,
  onChange,
}: {
  value: AnswerValue | undefined;
  onChange: (v: AnswerValue) => void;
}) {
  return (
    <div className="grid grid-cols-2 gap-3">
      {[
        { label: 'Yes', val: true },
        { label: 'No', val: false },
      ].map((opt) => {
        const selected = value === opt.val;
        return (
          <button
            key={opt.label}
            type="button"
            onClick={() => onChange(opt.val)}
            className={`rounded-md border px-4 py-3 text-sm font-medium ${
              selected
                ? opt.val
                  ? 'border-green-500 bg-green-50 text-green-800'
                  : 'border-red-500 bg-red-50 text-red-800'
                : 'border-gray-200 text-gray-700 hover:bg-gray-50'
            }`}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}

function SingleChoice({
  question,
  value,
  onChange,
}: {
  question: VoteQuestion;
  value: AnswerValue | undefined;
  onChange: (v: AnswerValue) => void;
}) {
  return (
    <>
      {question.options.map((opt) => (
        <label key={opt.id} className={optionRow}>
          <input
            type="radio"
            name={`q-${question.id}`}
            checked={value === opt.id}
            onChange={() => onChange(opt.id)}
          />
          <span>{opt.text}</span>
        </label>
      ))}
    </>
  );
}

function MultipleChoice({
  question,
  value,
  onChange,
}: {
  question: VoteQuestion;
  value: AnswerValue | undefined;
  onChange: (v: AnswerValue) => void;
}) {
  const selected = Array.isArray(value) ? value : [];
  const toggle = (id: string) => {
    onChange(selected.includes(id) ? selected.filter((s) => s !== id) : [...selected, id]);
  };
  return (
    <>
      {question.options.map((opt) => (
        <label key={opt.id} className={optionRow}>
          <input
            type="checkbox"
            checked={selected.includes(opt.id)}
            onChange={() => toggle(opt.id)}
          />
          <span>{opt.text}</span>
        </label>
      ))}
    </>
  );
}

function Ranked({
  question,
  value,
  onChange,
}: {
  question: VoteQuestion;
  value: AnswerValue | undefined;
  onChange: (v: AnswerValue) => void;
}) {
  const { t } = useTranslation();
  // Default ranking = declared option order until the voter reorders.
  const order = Array.isArray(value) && value.length ? value : question.options.map((o) => o.id);
  const labelFor = (id: string) => question.options.find((o) => o.id === id)?.text ?? id;

  const move = (index: number, delta: number) => {
    const next = [...order];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  };

  return (
    <ol className="space-y-2">
      {order.map((id, index) => (
        <li
          key={id}
          className="flex items-center gap-3 rounded-md border border-gray-200 px-3 py-2"
        >
          <span className="w-5 text-sm font-semibold text-gray-500">{index + 1}.</span>
          <span className="flex-1 text-sm">{labelFor(id)}</span>
          <button
            type="button"
            onClick={() => move(index, -1)}
            disabled={index === 0}
            className="rounded px-2 py-0.5 text-gray-500 hover:bg-gray-100 disabled:opacity-30"
            aria-label={t('aria.moveUp')}
          >
            ↑
          </button>
          <button
            type="button"
            onClick={() => move(index, 1)}
            disabled={index === order.length - 1}
            className="rounded px-2 py-0.5 text-gray-500 hover:bg-gray-100 disabled:opacity-30"
            aria-label={t('aria.moveDown')}
          >
            ↓
          </button>
        </li>
      ))}
    </ol>
  );
}
