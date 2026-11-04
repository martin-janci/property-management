/**
 * SuggestedQuestions component
 * Epic 127: AI Chatbot Interface
 *
 * Displays quick-start questions for new conversations.
 */

import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';
import type { SuggestedQuestion } from '../types';

interface SuggestedQuestionsProps {
  questions: SuggestedQuestion[];
  onSelect: (question: string) => void;
  disabled?: boolean;
}

export function SuggestedQuestions({
  questions,
  onSelect,
  disabled = false,
}: SuggestedQuestionsProps) {
  const { t } = useTranslation();

  if (questions.length === 0) {
    return null;
  }

  return (
    <div className="py-8 px-4">
      <div className="text-center mb-6">
        <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-blue-100 mb-4">
          <svg
            className="w-8 h-8 text-blue-600"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
            />
          </svg>
        </div>
        <h2 className="text-xl font-semibold text-gray-900 mb-2">{t('aiChat.welcomeTitle')}</h2>
        <p className="text-gray-600 max-w-md mx-auto">{t('aiChat.welcomeDescription')}</p>
      </div>

      <div className="max-w-2xl mx-auto">
        <h3 className="text-sm font-medium text-gray-500 mb-3">{t('aiChat.suggestedQuestions')}</h3>
        <div className="grid gap-2 sm:grid-cols-2">
          {questions.map((question) => (
            <button
              key={question.id}
              type="button"
              onClick={() => onSelect(question.text)}
              disabled={disabled}
              className="text-left px-4 py-3 rounded-lg border border-gray-200 hover:border-blue-300 hover:bg-blue-50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <span className="text-xs font-medium text-blue-600 uppercase tracking-wide">
                {question.category}
              </span>
              <p className="text-gray-900 mt-1">{question.text}</p>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

const DEFAULT_QUESTIONS = [
  { id: '1', key: 'reportFault', category: 'faults' },
  { id: '2', key: 'upcomingVotes', category: 'voting' },
  { id: '3', key: 'meterReadings', category: 'utilities' },
  { id: '4', key: 'buildingContacts', category: 'community' },
  { id: '5', key: 'ownersMeeting', category: 'events' },
  { id: '6', key: 'paymentHistory', category: 'finance' },
] as const;

/** Default suggested questions for property management context, in the active language. */
export function getDefaultSuggestedQuestions(t: TFunction): SuggestedQuestion[] {
  return DEFAULT_QUESTIONS.map(({ id, key, category }) => ({
    id,
    text: t(`aiChat.suggested.${key}`),
    category: t(`aiChat.categories.${category}`),
  }));
}
