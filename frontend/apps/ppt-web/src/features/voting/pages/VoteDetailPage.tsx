/**
 * Vote detail page (Epic 5, screen `ppt/vote-detail`).
 *
 * One screen that covers the deep-view + ballot + administration + results +
 * discussion stories: it renders the proposal, the state-aware manager action
 * toolbar (publish / close / cancel), the ballot (per-unit, per-question), live
 * or final results, and the comment thread — all wired to `@ppt/api-client`.
 */
import {
  type AnswerMap,
  type AnswerValue,
  useAddVoteComment,
  useCancelVote,
  useCastVote,
  useCloseVote,
  usePublishVote,
  useVote,
  useVoteComments,
  useVoteEligibility,
  useVoteResults,
} from '@ppt/api-client';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Spinner, useToast } from '../../../components';
import { BallotQuestion, QuestionResults, QuorumTile, VoteStatusBadge } from '../components';
import { QUORUM_TYPE_LABELS } from '../types';

interface VoteDetailPageProps {
  voteId: string;
  onBack: () => void;
}

function formatDateTime(value: string | null): string {
  if (!value) return '—';
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? '—' : d.toLocaleString();
}

export function VoteDetailPage({ voteId, onBack }: VoteDetailPageProps) {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { data: vote, isLoading, error } = useVote(voteId);
  const { data: eligibility } = useVoteEligibility(voteId);
  const resultsEnabled = vote?.status === 'active' || vote?.status === 'closed';
  const { data: results } = useVoteResults(voteId, resultsEnabled);
  const { data: comments } = useVoteComments(voteId);

  const castVote = useCastVote(voteId);
  const publishVote = usePublishVote(voteId);
  const closeVote = useCloseVote(voteId);
  const cancelVote = useCancelVote(voteId);
  const addComment = useAddVoteComment(voteId);

  const [selectedUnit, setSelectedUnit] = useState('');
  const [answers, setAnswers] = useState<AnswerMap>({});
  const [commentText, setCommentText] = useState('');

  const eligibleUnits = eligibility?.eligible_units ?? [];
  const effectiveUnit = selectedUnit || eligibleUnits[0]?.unit_id || '';

  const canCast = useMemo(
    () => vote?.status === 'active' && (eligibility?.can_vote ?? false) && !!effectiveUnit,
    [vote?.status, eligibility?.can_vote, effectiveUnit]
  );

  if (isLoading) {
    return (
      <div className="flex justify-center py-24">
        <Spinner size="lg" color="blue" />
      </div>
    );
  }

  if (error || !vote) {
    return (
      <div className="mx-auto max-w-md py-16 text-center">
        <h1 className="text-lg font-semibold text-gray-900">
          {t('voting.detail.unavailableTitle')}
        </h1>
        <p className="mt-2 text-sm text-gray-500">
          {error instanceof Error ? error.message : t('voting.detail.unavailableBody')}
        </p>
        <button
          type="button"
          onClick={onBack}
          className="mt-6 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700"
        >
          {t('voting.detail.backToVoting')}
        </button>
      </div>
    );
  }

  const setAnswer = (questionId: string, value: AnswerValue) => {
    setAnswers((prev) => ({ ...prev, [questionId]: value }));
  };

  const handleCast = async () => {
    if (!effectiveUnit) {
      showToast({ type: 'error', title: t('voting.detail.selectUnitError') });
      return;
    }
    const missing = vote.questions.filter((q) => q.is_required && answers[q.id] === undefined);
    if (missing.length > 0) {
      showToast({ type: 'error', title: t('voting.detail.answerRequiredError') });
      return;
    }
    try {
      await castVote.mutateAsync({ unit_id: effectiveUnit, answers });
      showToast({
        type: 'success',
        title: t('voting.detail.ballotSubmittedTitle'),
        message: t('voting.detail.ballotSubmittedBody'),
      });
      setAnswers({});
    } catch (e) {
      showToast({
        type: 'error',
        title: t('voting.detail.ballotFailedTitle'),
        message: e instanceof Error ? e.message : t('voting.detail.unexpectedError'),
      });
    }
  };

  const runAction = async (
    successTitle: string,
    failureTitle: string,
    fn: () => Promise<unknown>
  ) => {
    try {
      await fn();
      showToast({ type: 'success', title: successTitle });
    } catch (e) {
      showToast({
        type: 'error',
        title: failureTitle,
        message: e instanceof Error ? e.message : t('voting.detail.unexpectedError'),
      });
    }
  };

  const handleCancel = () => {
    const reason = window.prompt(t('voting.detail.cancelReasonPrompt'));
    if (!reason) return;
    void runAction(t('voting.detail.cancelDone'), t('voting.detail.cancelFailed'), () =>
      cancelVote.mutateAsync({ reason })
    );
  };

  const handleAddComment = async () => {
    if (!commentText.trim()) return;
    try {
      await addComment.mutateAsync({ content: commentText.trim(), ai_consent: false });
      setCommentText('');
    } catch (e) {
      showToast({
        type: 'error',
        title: t('voting.detail.commentFailedTitle'),
        message: e instanceof Error ? e.message : t('voting.detail.unexpectedError'),
      });
    }
  };

  const busy =
    publishVote.isPending || closeVote.isPending || cancelVote.isPending || castVote.isPending;

  return (
    <div className="mx-auto max-w-4xl px-4 py-8">
      <button type="button" onClick={onBack} className="text-sm text-gray-500 hover:text-gray-700">
        ← {t('voting.detail.backToVoting')}
      </button>

      {/* Header */}
      <div className="mt-2 flex items-start justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <VoteStatusBadge status={vote.status} />
            <span className="text-xs text-gray-500">
              {t('voting.detail.closes', { date: formatDateTime(vote.end_at) })}
            </span>
          </div>
          <h1 className="mt-1.5 text-2xl font-bold text-gray-900">{vote.title}</h1>
        </div>
        <div className="flex shrink-0 flex-wrap justify-end gap-2">
          {vote.status === 'draft' ? (
            <button
              type="button"
              disabled={busy}
              onClick={() =>
                void runAction(
                  t('voting.detail.publishDone'),
                  t('voting.detail.publishFailed'),
                  () => publishVote.mutateAsync({})
                )
              }
              className="rounded-md bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {t('voting.detail.publishNow')}
            </button>
          ) : null}
          {vote.status === 'active' ? (
            <button
              type="button"
              disabled={busy}
              onClick={() =>
                void runAction(t('voting.detail.closeDone'), t('voting.detail.closeFailed'), () =>
                  closeVote.mutateAsync()
                )
              }
              className="rounded-md border border-gray-300 px-3 py-1.5 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
            >
              {t('voting.detail.closeNow')}
            </button>
          ) : null}
          {vote.status === 'draft' || vote.status === 'scheduled' || vote.status === 'active' ? (
            <button
              type="button"
              disabled={busy}
              onClick={handleCancel}
              className="rounded-md border border-red-200 px-3 py-1.5 text-sm font-medium text-red-600 hover:bg-red-50 disabled:opacity-50"
            >
              {t('voting.detail.cancelVote')}
            </button>
          ) : null}
        </div>
      </div>

      {/* Meta + quorum */}
      <div className="mt-4 flex flex-wrap items-center gap-6 rounded-lg bg-white p-4 shadow">
        <div className="text-sm">
          <span className="text-gray-500">{t('voting.detail.quorumLabel')}</span>{' '}
          <span className="font-medium text-gray-900">{QUORUM_TYPE_LABELS[vote.quorum_type]}</span>
        </div>
        <QuorumTile
          participation={vote.participation_count}
          eligible={vote.eligible_count}
          quorumMet={vote.quorum_met}
        />
      </div>

      {/* Description */}
      {vote.description ? (
        <div className="mt-4 rounded-lg bg-white p-5 shadow">
          <h2 className="text-sm font-semibold text-gray-900">{t('voting.detail.description')}</h2>
          <p className="mt-2 whitespace-pre-wrap text-sm text-gray-700">{vote.description}</p>
        </div>
      ) : null}

      {/* Ballot */}
      {vote.status === 'active' ? (
        <div className="mt-4 rounded-lg bg-white p-5 shadow">
          <h2 className="text-sm font-semibold text-gray-900">{t('voting.detail.castBallot')}</h2>
          {eligibleUnits.length === 0 ? (
            <p className="mt-2 text-sm text-gray-500">
              {eligibility?.reason ?? t('voting.detail.noEligibleUnits')}
            </p>
          ) : (
            <>
              {eligibleUnits.length > 1 ? (
                <label className="mt-3 block text-sm font-medium text-gray-700">
                  {t('voting.detail.votingAsUnit')}
                  <select
                    className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                    value={effectiveUnit}
                    onChange={(e) => setSelectedUnit(e.target.value)}
                  >
                    {eligibleUnits.map((u) => (
                      <option key={u.unit_id} value={u.unit_id}>
                        {u.unit_designation ?? u.unit_id}
                        {u.already_voted ? ` (${t('voting.detail.alreadyVoted')})` : ''}
                      </option>
                    ))}
                  </select>
                </label>
              ) : null}
              <div className="mt-4 space-y-4">
                {vote.questions.map((q) => (
                  <BallotQuestion
                    key={q.id}
                    question={q}
                    value={answers[q.id]}
                    onChange={(v) => setAnswer(q.id, v)}
                  />
                ))}
              </div>
              <button
                type="button"
                disabled={!canCast || castVote.isPending}
                onClick={handleCast}
                className="mt-4 w-full rounded-md bg-blue-600 px-4 py-2.5 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
              >
                {castVote.isPending
                  ? t('voting.detail.submitting')
                  : t('voting.detail.submitBallot')}
              </button>
            </>
          )}
        </div>
      ) : null}

      {/* Results */}
      {results && results.questions.length > 0 ? (
        <div className="mt-4 rounded-lg bg-white p-5 shadow">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold text-gray-900">{t('voting.detail.results')}</h2>
            <span className="text-xs text-gray-500">
              {t('voting.detail.turnout', {
                percent: Math.round(results.participation_rate * 100),
              })}{' '}
              ·{' '}
              {results.quorum_met ? t('voting.detail.quorumMet') : t('voting.detail.quorumNotMet')}
            </span>
          </div>
          <div className="mt-3 space-y-4">
            {results.questions.map((r) => (
              <QuestionResults key={r.question_id} result={r} />
            ))}
          </div>
        </div>
      ) : null}

      {/* Discussion */}
      <div className="mt-4 rounded-lg bg-white p-5 shadow">
        <h2 className="text-sm font-semibold text-gray-900">{t('voting.detail.discussion')}</h2>
        <div className="mt-3 space-y-3">
          {comments && comments.length > 0 ? (
            comments.map((c) => (
              <div key={c.id} className="rounded-md border border-gray-100 bg-gray-50 p-3">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium text-gray-900">{c.user_name}</span>
                  <span className="text-xs text-gray-400">{formatDateTime(c.created_at)}</span>
                </div>
                <p className="mt-1 text-sm text-gray-700">{c.content}</p>
              </div>
            ))
          ) : (
            <p className="text-sm text-gray-500">{t('voting.detail.noComments')}</p>
          )}
        </div>
        <div className="mt-4 flex gap-2">
          <input
            className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
            value={commentText}
            onChange={(e) => setCommentText(e.target.value)}
            placeholder={t('voting.detail.addCommentPlaceholder')}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void handleAddComment();
            }}
          />
          <button
            type="button"
            disabled={addComment.isPending || !commentText.trim()}
            onClick={() => void handleAddComment()}
            className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
          >
            {t('voting.detail.post')}
          </button>
        </div>
      </div>
    </div>
  );
}
