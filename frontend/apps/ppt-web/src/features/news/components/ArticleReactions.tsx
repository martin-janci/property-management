/**
 * Article Reactions Component (Epic 59, Story 59.2)
 * Allows users to react to articles with different emotions
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ReactionCounts, ReactionType } from '../types';

interface ArticleReactionsProps {
  reactionCounts: ReactionCounts;
  userReaction?: ReactionType | null;
  onToggleReaction: (reaction: ReactionType) => Promise<void>;
  disabled?: boolean;
}

const reactionEmojis: Record<ReactionType, string> = {
  like: '👍',
  love: '❤️',
  surprised: '😮',
  sad: '😢',
  angry: '😠',
};

const reactionLabels: Record<ReactionType, string> = {
  like: 'Like',
  love: 'Love',
  surprised: 'Surprised',
  sad: 'Sad',
  angry: 'Angry',
};

export function ArticleReactions({
  reactionCounts,
  userReaction,
  onToggleReaction,
  disabled = false,
}: ArticleReactionsProps) {
  const { t } = useTranslation();
  const [isLoading, setIsLoading] = useState(false);

  const handleReactionClick = async (reaction: ReactionType) => {
    if (disabled || isLoading) return;

    setIsLoading(true);
    try {
      await onToggleReaction(reaction);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="flex items-center gap-2 flex-wrap" aria-label={t('aria.articleReactions')}>
      {(Object.keys(reactionEmojis) as ReactionType[]).map((reaction) => {
        const count = reactionCounts[reaction];
        const isActive = userReaction === reaction;

        return (
          <button
            key={reaction}
            type="button"
            onClick={() => handleReactionClick(reaction)}
            disabled={disabled || isLoading}
            className={`
              inline-flex items-center gap-1 px-3 py-1.5 rounded-full text-sm font-medium
              transition-all duration-200
              ${
                isActive
                  ? 'bg-blue-100 text-blue-700 ring-2 ring-blue-500'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }
              ${disabled || isLoading ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
            `}
            aria-label={`${reactionLabels[reaction]} reaction${isActive ? ' (active)' : ''}`}
            aria-pressed={isActive}
          >
            <span className="text-lg" aria-hidden="true">
              {reactionEmojis[reaction]}
            </span>
            {count > 0 && (
              <span className="text-xs font-semibold min-w-[1rem] text-center">{count}</span>
            )}
          </button>
        );
      })}

      {reactionCounts.total > 0 && (
        <span className="text-sm text-gray-500 ml-2" aria-live="polite">
          {reactionCounts.total} {reactionCounts.total === 1 ? 'reaction' : 'reactions'}
        </span>
      )}
    </div>
  );
}
