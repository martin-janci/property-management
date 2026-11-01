/**
 * VoteDetailScreen (UC-04 / Epic 5).
 *
 * Inline voting flow that the list screen can't do on its own — `GET /voting`
 * only returns `VoteSummary` (id/title/status/quorum) without the questions or
 * options the user has to choose between.
 *
 * Loads in three steps:
 *   1. `GET /api/v1/voting/{id}` → `VoteWithDetails` for the title block
 *   2. `GET /api/v1/voting/{id}/questions` → questions + options for ballot
 *   3. `GET /api/v1/voting/{id}/eligibility` → which unit(s) the user can
 *      vote on behalf of, and whether they've already voted
 *
 * Then `POST /api/v1/voting/{id}/cast` with `{ unit_id, answers }` where
 * `answers` is `{ [question_id]: option_id | option_id[] }` for
 * single/yes_no vs multiple_choice / ranked questions.
 *
 * Out of scope (handled by the manager-facing web UI):
 *   - delegation_id selection (stub here; uses null)
 *   - comments / audit log
 *   - results visualization (closed votes show vote.results from the
 *     summary, but live result charts are deferred)
 */

import { useQueryClient } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
  Alert,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { useApiMutation, useApiQuery } from '../../hooks/useApi';
import { formatDate } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';

interface ApiQuestionOption {
  id: string;
  text: string;
  order: number;
}

/** Mirrors `VoteQuestion` from `db/models/vote.rs`. */
interface ApiVoteQuestion {
  id: string;
  vote_id: string;
  question_text: string;
  description: string | null;
  question_type: 'yes_no' | 'single_choice' | 'multiple_choice' | 'ranked' | string;
  options: ApiQuestionOption[];
  display_order: number;
  is_required: boolean;
}

/** Mirrors `VoteWithDetails` (Vote fields are flattened in the JSON). */
interface ApiVoteDetails {
  id: string;
  title: string;
  description: string | null;
  status: string;
  start_at: string | null;
  end_at: string;
  quorum_type: string;
  quorum_percentage: number | null;
  participation_count: number | null;
  eligible_count: number | null;
  building_name: string;
  created_by_name: string;
  question_count: number;
}

interface ApiEligibleUnit {
  unit_id: string;
  unit_designation: string;
  ownership_share: string;
  is_owner: boolean;
  is_delegated: boolean;
  delegation_id: string | null;
  already_voted: boolean;
}

interface ApiVoteEligibility {
  vote_id: string;
  user_id: string;
  eligible_units: ApiEligibleUnit[];
  can_vote: boolean;
  reason: string | null;
}

interface ApiVoteReceipt {
  response_id: string;
  vote_id: string;
  unit_id: string;
  submitted_at: string;
  response_hash: string;
  confirmation_number: string;
}

interface CastVoteVariables {
  unit_id: string;
  delegation_id?: string | null;
  answers: Record<string, string | string[]>;
}

interface VoteDetailScreenProps {
  voteId?: string;
  onBack?: () => void;
}

export function VoteDetailScreen({ voteId, onBack }: VoteDetailScreenProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  // Every hook below must run unconditionally on each render, so the
  // `voteId`-missing case is handled with an early return *after* all hooks
  // (see below) — never before — to respect React's Rules of Hooks. The
  // queries are gated on `enabled: !!voteId` so a missing id never fetches
  // `/voting/undefined`.
  const hasVoteId = !!voteId;

  const detailQuery = useApiQuery<ApiVoteDetails>(
    ['voting', 'detail', voteId],
    `/api/v1/voting/${voteId}`,
    { staleTime: 30_000, enabled: hasVoteId }
  );

  const questionsQuery = useApiQuery<ApiVoteQuestion[]>(
    ['voting', 'questions', voteId],
    `/api/v1/voting/${voteId}/questions`,
    { staleTime: 30_000, enabled: hasVoteId }
  );

  const eligibilityQuery = useApiQuery<ApiVoteEligibility>(
    ['voting', 'eligibility', voteId],
    `/api/v1/voting/${voteId}/eligibility`,
    { staleTime: 30_000, enabled: hasVoteId }
  );

  const castMutation = useApiMutation<ApiVoteReceipt, CastVoteVariables>(
    `/api/v1/voting/${voteId}/cast`,
    'POST'
  );

  // selections: question_id -> option_id (single) or option_id[] (multi/ranked)
  const [selections, setSelections] = useState<Record<string, string | string[]>>({});

  const isLoading = detailQuery.isLoading || questionsQuery.isLoading || eligibilityQuery.isLoading;
  const loadError = detailQuery.error ?? questionsQuery.error ?? eligibilityQuery.error;

  const detail = detailQuery.data;
  const questions = useMemo(
    () => (questionsQuery.data ?? []).slice().sort((a, b) => a.display_order - b.display_order),
    [questionsQuery.data]
  );
  const eligibility = eligibilityQuery.data;

  // Pick the first eligible unit that hasn't yet voted. Multi-unit owners
  // can switch with the unit picker below.
  const defaultUnit = useMemo(
    () =>
      eligibility?.eligible_units.find((u) => !u.already_voted) ?? eligibility?.eligible_units[0],
    [eligibility]
  );
  const [selectedUnitId, setSelectedUnitId] = useState<string | undefined>(undefined);

  // `voteId` is missing only if navigation was wired up wrong — render an
  // explicit error rather than silently fetching `/voting/undefined`. This
  // early return MUST stay below every hook call above so hook order is
  // stable across renders (React's Rules of Hooks).
  if (!voteId) {
    return (
      <View style={s.container}>
        <View style={s.header}>
          {onBack && (
            <Pressable onPress={onBack} style={styles.backLink}>
              <Text style={styles.backLinkText}>
                ← {t('common.back', { defaultValue: 'Back' })}
              </Text>
            </Pressable>
          )}
          <Text style={s.headerTitle}>{t('voting.title', { defaultValue: 'Voting' })}</Text>
        </View>
        <View style={s.emptyState}>
          <Text style={s.emptyIcon}>⚠️</Text>
          <Text style={s.emptyTitle}>
            {t('voting.missingId', { defaultValue: 'No vote selected' })}
          </Text>
        </View>
      </View>
    );
  }

  const activeUnit =
    eligibility?.eligible_units.find((u) => u.unit_id === selectedUnitId) ?? defaultUnit;

  const isActive = detail?.status === 'active';
  const alreadyVoted = activeUnit?.already_voted ?? false;
  const canVote = !!eligibility?.can_vote && isActive && !alreadyVoted;

  const isAnswerComplete = (q: ApiVoteQuestion): boolean => {
    if (!q.is_required) return true;
    // Ranked questions always carry a full ordering: the declared option order
    // is the default ballot until the voter reorders, so they're complete as
    // long as there are options to rank.
    if (q.question_type === 'ranked') return (q.options?.length ?? 0) > 0;
    const v = selections[q.id];
    if (Array.isArray(v)) return v.length > 0;
    return typeof v === 'string' && v.length > 0;
  };
  const allRequiredAnswered = questions.every(isAnswerComplete);

  // Ranked answers are ordered option-id arrays whose position expresses
  // preference (the backend weights rank 0 highest — see repositories/vote.rs).
  // A ranked question the voter never touched still submits the declared option
  // order rather than a missing answer, so the ballot always carries a complete
  // ordering instead of a bare option set.
  const buildAnswers = (): Record<string, string | string[]> => {
    const answers: Record<string, string | string[]> = { ...selections };
    for (const q of questions) {
      if (q.question_type === 'ranked' && !Array.isArray(answers[q.id])) {
        answers[q.id] = [...(q.options ?? [])].sort((a, b) => a.order - b.order).map((o) => o.id);
      }
    }
    return answers;
  };

  const submit = () => {
    if (!activeUnit) return;
    if (!allRequiredAnswered) {
      Alert.alert(
        t('voting.confirmVoteTitle', { defaultValue: 'Confirm vote' }),
        t('voting.answerRequired', { defaultValue: 'Please answer all required questions' })
      );
      return;
    }
    Alert.alert(
      t('voting.confirmVoteTitle', { defaultValue: 'Confirm vote' }),
      t('voting.confirmVoteMessage', {
        defaultValue: 'Submit your ballot? This action cannot be undone.',
      }),
      [
        { text: t('common.cancel', { defaultValue: 'Cancel' }), style: 'cancel' },
        {
          text: t('voting.submit', { defaultValue: 'Submit' }),
          style: 'default',
          onPress: () => {
            castMutation.mutate(
              {
                unit_id: activeUnit.unit_id,
                delegation_id: activeUnit.delegation_id,
                answers: buildAnswers(),
              },
              {
                onSuccess: (receipt) => {
                  // Refresh eligibility so the screen flips into "voted" state,
                  // and the list view's summary too.
                  queryClient.invalidateQueries({ queryKey: ['voting', 'eligibility', voteId] });
                  queryClient.invalidateQueries({ queryKey: ['voting', 'detail', voteId] });
                  queryClient.invalidateQueries({ queryKey: ['voting', 'list'] });
                  Alert.alert(
                    t('voting.voteSubmitted', { defaultValue: 'Ballot submitted' }),
                    `${t('voting.confirmationNumber', { defaultValue: 'Confirmation' })}: ${receipt.confirmation_number}`
                  );
                },
                onError: (err) => {
                  Alert.alert(
                    t('voting.castError', { defaultValue: "Couldn't submit ballot" }),
                    err.message
                  );
                },
              }
            );
          },
        },
      ]
    );
  };

  const toggleSingle = (questionId: string, optionId: string) => {
    setSelections((prev) => ({ ...prev, [questionId]: optionId }));
  };

  const toggleMulti = (questionId: string, optionId: string) => {
    setSelections((prev) => {
      const current = prev[questionId];
      const existing = Array.isArray(current) ? current : [];
      const next = existing.includes(optionId)
        ? existing.filter((id) => id !== optionId)
        : [...existing, optionId];
      return { ...prev, [questionId]: next };
    });
  };

  // Ranked questions capture the full option ordering (position = preference).
  // The card hands back the reordered list; we store it verbatim.
  const setRank = (questionId: string, order: string[]) => {
    setSelections((prev) => ({ ...prev, [questionId]: order }));
  };

  return (
    <View style={s.container}>
      <View style={s.header}>
        {onBack && (
          <Pressable onPress={onBack} style={styles.backLink}>
            <Text style={styles.backLinkText}>
              ← {t('voting.title', { defaultValue: 'Voting' })}
            </Text>
          </Pressable>
        )}
        <Text style={s.headerTitle} numberOfLines={2}>
          {detail?.title ?? t('common.loading', { defaultValue: 'Loading…' })}
        </Text>
        {detail && (
          <Text style={s.headerSubtitle}>
            {detail.building_name} ·{' '}
            {formatDate(detail.end_at, {
              year: 'numeric',
              month: 'short',
              day: 'numeric',
            })}
          </Text>
        )}
      </View>

      <ScrollView style={s.scrollView}>
        {isLoading ? (
          <View style={s.emptyState}>
            <ActivityIndicator color={colors.accent} />
          </View>
        ) : loadError ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>
              {t('voting.loadError', { defaultValue: "Couldn't load vote" })}
            </Text>
          </View>
        ) : (
          <>
            {detail?.description ? (
              <View style={s.card}>
                <Text style={s.cardBody}>{detail.description}</Text>
              </View>
            ) : null}

            <StatusBanner
              status={detail?.status}
              alreadyVoted={alreadyVoted}
              canVote={canVote}
              eligibilityReason={eligibility?.reason}
            />

            {eligibility && eligibility.eligible_units.length > 1 && (
              <UnitPicker
                units={eligibility.eligible_units}
                activeUnitId={activeUnit?.unit_id}
                onSelect={(id) => setSelectedUnitId(id)}
              />
            )}

            {questions.length === 0 ? (
              <View style={s.emptyState}>
                <Text style={s.emptyIcon}>📋</Text>
                <Text style={s.emptyTitle}>
                  {t('voting.noQuestions', { defaultValue: 'No questions yet' })}
                </Text>
              </View>
            ) : (
              questions.map((q, idx) => (
                <QuestionCard
                  key={q.id}
                  index={idx + 1}
                  question={q}
                  selection={selections[q.id]}
                  disabled={!canVote}
                  onSingle={(optionId) => toggleSingle(q.id, optionId)}
                  onMulti={(optionId) => toggleMulti(q.id, optionId)}
                  onRank={(order) => setRank(q.id, order)}
                />
              ))
            )}

            {canVote && (
              <Pressable
                style={[s.primaryButton, castMutation.isPending && styles.buttonPending]}
                onPress={submit}
                disabled={castMutation.isPending}
              >
                {castMutation.isPending ? (
                  <ActivityIndicator color="#fff" />
                ) : (
                  <Text style={s.primaryButtonText}>
                    {t('voting.submit', { defaultValue: 'Submit ballot' })}
                  </Text>
                )}
              </Pressable>
            )}

            <View style={s.bottomSpacer} />
          </>
        )}
      </ScrollView>
    </View>
  );
}

interface StatusBannerProps {
  status: string | undefined;
  alreadyVoted: boolean;
  canVote: boolean;
  eligibilityReason: string | null | undefined;
}

function StatusBanner({ status, alreadyVoted, canVote, eligibilityReason }: StatusBannerProps) {
  const { t } = useTranslation();
  if (alreadyVoted) {
    return (
      <View style={[styles.banner, styles.bannerSuccess]}>
        <Text style={styles.bannerIcon}>✓</Text>
        <Text style={styles.bannerText}>
          {t('voting.youHaveVoted', { defaultValue: 'You have already voted on this ballot.' })}
        </Text>
      </View>
    );
  }
  if (status && status !== 'active') {
    return (
      <View style={[styles.banner, styles.bannerMuted]}>
        <Text style={styles.bannerText}>
          {t('voting.notActive', { defaultValue: `Voting is ${status}.` })}
        </Text>
      </View>
    );
  }
  if (!canVote && eligibilityReason) {
    return (
      <View style={[styles.banner, styles.bannerWarning]}>
        <Text style={styles.bannerText}>{eligibilityReason}</Text>
      </View>
    );
  }
  return null;
}

interface UnitPickerProps {
  units: ApiEligibleUnit[];
  activeUnitId: string | undefined;
  onSelect: (unitId: string) => void;
}

function UnitPicker({ units, activeUnitId, onSelect }: UnitPickerProps) {
  const { t } = useTranslation();
  return (
    <View style={s.card}>
      <Text style={[s.cardTitle, { marginBottom: 8 }]}>
        {t('voting.votingAs', { defaultValue: 'Voting as' })}
      </Text>
      <View style={styles.unitRow}>
        {units.map((u) => {
          const active = u.unit_id === activeUnitId;
          return (
            <Pressable
              key={u.unit_id}
              style={[styles.unitChip, active && styles.unitChipActive]}
              onPress={() => onSelect(u.unit_id)}
            >
              <Text style={[styles.unitChipText, active && styles.unitChipTextActive]}>
                {u.unit_designation}
                {u.already_voted ? ' ✓' : ''}
              </Text>
            </Pressable>
          );
        })}
      </View>
    </View>
  );
}

interface QuestionCardProps {
  index: number;
  question: ApiVoteQuestion;
  selection: string | string[] | undefined;
  disabled: boolean;
  onSingle: (optionId: string) => void;
  onMulti: (optionId: string) => void;
  onRank: (order: string[]) => void;
}

function QuestionCard({
  index,
  question,
  selection,
  disabled,
  onSingle,
  onMulti,
  onRank,
}: QuestionCardProps) {
  // `ranked` is NOT a multi-select: it captures an ordered preference list, so
  // it gets its own reorderable renderer. Only `multiple_choice` is a genuine
  // unordered checkbox group.
  const isMulti = question.question_type === 'multiple_choice';
  const isRanked = question.question_type === 'ranked';
  const sortedOptions = [...(question.options ?? [])].sort((a, b) => a.order - b.order);

  if (isRanked) {
    return (
      <RankedQuestionCard
        index={index}
        question={question}
        sortedOptions={sortedOptions}
        selection={selection}
        disabled={disabled}
        onRank={onRank}
      />
    );
  }

  return (
    <View style={s.card}>
      <Text style={styles.questionLabel}>Q{index}</Text>
      <Text style={s.cardTitle}>
        {question.question_text}
        {question.is_required ? ' *' : ''}
      </Text>
      {question.description ? (
        <Text style={[s.cardBody, { marginBottom: 8 }]}>{question.description}</Text>
      ) : null}

      <View style={{ marginTop: 8 }}>
        {sortedOptions.map((opt) => {
          const selected = isMulti
            ? Array.isArray(selection) && selection.includes(opt.id)
            : selection === opt.id;
          return (
            <Pressable
              key={opt.id}
              style={[
                styles.optionRow,
                selected && styles.optionRowSelected,
                disabled && styles.optionRowDisabled,
              ]}
              disabled={disabled}
              onPress={() => (isMulti ? onMulti(opt.id) : onSingle(opt.id))}
            >
              <View
                style={[
                  isMulti ? styles.checkbox : styles.radio,
                  selected && styles.markerSelected,
                ]}
              >
                {selected && <View style={isMulti ? styles.checkInner : styles.radioInner} />}
              </View>
              <Text style={[styles.optionLabel, selected && styles.optionLabelSelected]}>
                {opt.text}
              </Text>
            </Pressable>
          );
        })}
      </View>
    </View>
  );
}

interface RankedQuestionCardProps {
  index: number;
  question: ApiVoteQuestion;
  sortedOptions: ApiQuestionOption[];
  selection: string | string[] | undefined;
  disabled: boolean;
  onRank: (order: string[]) => void;
}

/**
 * Ranked-choice ballot input. The voter orders every option; the array
 * position is the preference (rank 1 = top). Mirrors the ppt-web `Ranked`
 * input: the declared option order is the default ballot until the voter
 * moves a row, and the full ordering is what gets submitted so the backend
 * can weight by rank position (repositories/vote.rs).
 */
function RankedQuestionCard({
  index,
  question,
  sortedOptions,
  selection,
  disabled,
  onRank,
}: RankedQuestionCardProps) {
  const { t } = useTranslation();
  const order =
    Array.isArray(selection) && selection.length > 0 ? selection : sortedOptions.map((o) => o.id);
  const labelFor = (id: string) => sortedOptions.find((o) => o.id === id)?.text ?? id;

  const move = (from: number, delta: number) => {
    const to = from + delta;
    if (to < 0 || to >= order.length) return;
    const next = [...order];
    [next[from], next[to]] = [next[to], next[from]];
    onRank(next);
  };

  return (
    <View style={s.card}>
      <Text style={styles.questionLabel}>Q{index}</Text>
      <Text style={s.cardTitle}>
        {question.question_text}
        {question.is_required ? ' *' : ''}
      </Text>
      <Text style={[s.cardBody, { marginBottom: 8 }]}>
        {question.description ??
          t('voting.rankHint', {
            defaultValue: 'Order the options by preference (1 = most preferred).',
          })}
      </Text>

      <View style={{ marginTop: 8 }}>
        {order.map((id, i) => (
          <View
            key={id}
            style={[styles.optionRow, styles.rankRow, disabled && styles.optionRowDisabled]}
          >
            <View style={styles.rankBadge}>
              <Text style={styles.rankBadgeText}>{i + 1}</Text>
            </View>
            <Text style={[styles.optionLabel, { flex: 1 }]}>{labelFor(id)}</Text>
            <Pressable
              style={[styles.rankButton, (disabled || i === 0) && styles.rankButtonDisabled]}
              disabled={disabled || i === 0}
              onPress={() => move(i, -1)}
              accessibilityRole="button"
              accessibilityLabel={t('voting.moveUp', { defaultValue: 'Move up' })}
            >
              <Text style={styles.rankButtonText}>↑</Text>
            </Pressable>
            <Pressable
              style={[
                styles.rankButton,
                (disabled || i === order.length - 1) && styles.rankButtonDisabled,
              ]}
              disabled={disabled || i === order.length - 1}
              onPress={() => move(i, 1)}
              accessibilityRole="button"
              accessibilityLabel={t('voting.moveDown', { defaultValue: 'Move down' })}
            >
              <Text style={styles.rankButtonText}>↓</Text>
            </Pressable>
          </View>
        ))}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  backLink: { marginBottom: 8 },
  backLinkText: { color: colors.accent, fontSize: 14 },
  banner: {
    flexDirection: 'row',
    alignItems: 'center',
    padding: 12,
    borderRadius: 8,
    marginBottom: 12,
    gap: 8,
  },
  bannerSuccess: { backgroundColor: '#dcfce7' },
  bannerWarning: { backgroundColor: '#fef3c7' },
  bannerMuted: { backgroundColor: colors.surfaceMuted },
  bannerIcon: { fontSize: 14, color: colors.success, fontWeight: '700' },
  bannerText: { fontSize: 13, color: colors.text, flex: 1 },
  unitRow: { flexDirection: 'row', flexWrap: 'wrap', gap: 8 },
  unitChip: {
    paddingHorizontal: 12,
    paddingVertical: 6,
    borderRadius: 16,
    backgroundColor: colors.surfaceMuted,
  },
  unitChipActive: { backgroundColor: colors.accent },
  unitChipText: { fontSize: 13, color: colors.textMuted, fontWeight: '500' },
  unitChipTextActive: { color: '#fff' },
  questionLabel: {
    fontSize: 11,
    fontWeight: '700',
    color: colors.textSubtle,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: 4,
  },
  optionRow: {
    flexDirection: 'row',
    alignItems: 'center',
    padding: 12,
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.border,
    marginBottom: 8,
  },
  optionRowSelected: { backgroundColor: '#eff6ff', borderColor: colors.accent },
  optionRowDisabled: { opacity: 0.7 },
  rankRow: { gap: 8 },
  rankBadge: {
    width: 24,
    height: 24,
    borderRadius: 12,
    backgroundColor: colors.accent,
    alignItems: 'center',
    justifyContent: 'center',
  },
  rankBadgeText: { fontSize: 12, fontWeight: '700', color: '#fff' },
  rankButton: {
    width: 32,
    height: 32,
    borderRadius: 6,
    backgroundColor: colors.surfaceMuted,
    alignItems: 'center',
    justifyContent: 'center',
  },
  rankButtonDisabled: { opacity: 0.3 },
  rankButtonText: { fontSize: 16, color: colors.text, fontWeight: '600' },
  optionLabel: { fontSize: 14, color: colors.text, flex: 1 },
  optionLabelSelected: { color: colors.accent, fontWeight: '500' },
  radio: {
    width: 20,
    height: 20,
    borderRadius: 10,
    borderWidth: 2,
    borderColor: '#d1d5db',
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: 10,
  },
  checkbox: {
    width: 20,
    height: 20,
    borderRadius: 4,
    borderWidth: 2,
    borderColor: '#d1d5db',
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: 10,
  },
  markerSelected: { borderColor: colors.accent },
  radioInner: { width: 10, height: 10, borderRadius: 5, backgroundColor: colors.accent },
  checkInner: { width: 10, height: 10, borderRadius: 2, backgroundColor: colors.accent },
  buttonPending: { opacity: 0.6 },
});
