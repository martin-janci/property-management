/**
 * Voting API module exports (Epic 5: Building Voting & Decisions, FR37-44).
 */

// API functions.
//
// The comment-domain helpers are re-exported under vote-scoped aliases
// (`addComment` → `addVoteComment`, `listComments` → `listVoteComments`, …)
// because the bare names collide with the `faults` and `news` modules in the
// top-level `@ppt/api-client` barrel (TS2308). UIs consume the React Query
// hooks anyway; the raw aliases stay available for non-React callers.
export {
  addComment as addVoteComment,
  addQuestion,
  cancelVote,
  castVote,
  closeVote,
  createVote,
  deleteQuestion,
  deleteVote,
  getAuditLog,
  getEligibility,
  getMyResponse,
  getResults,
  getVote,
  hideComment as hideVoteComment,
  listActiveVotesForBuilding,
  listCommentReplies as listVoteCommentReplies,
  listComments as listVoteComments,
  listQuestions,
  listVotes,
  publishVote,
  updateQuestion,
  updateVote,
} from './api';

// React Query hooks
export {
  useActiveVotesForBuilding,
  useAddQuestion,
  useAddVoteComment,
  useCancelVote,
  useCastVote,
  useCloseVote,
  useCreateVote,
  useDeleteQuestion,
  useDeleteVote,
  useMyVoteResponse,
  usePublishVote,
  useUpdateQuestion,
  useUpdateVote,
  useVote,
  useVoteAuditLog,
  useVoteComments,
  useVoteEligibility,
  useVoteQuestions,
  useVoteResults,
  useVotes,
  votingKeys,
} from './hooks';

// Types. `AddCommentRequest`/`HideCommentRequest` are vote-scoped aliases to
// avoid the same `faults`-module collision as the functions above.
export type {
  AddCommentRequest as AddVoteCommentRequest,
  AddQuestionRequest,
  AnswerMap,
  AnswerValue,
  CancelVoteRequest,
  CastVoteRequest,
  CreateVoteRequest,
  EligibleUnit,
  HideCommentRequest as HideVoteCommentRequest,
  ListVotesQuery,
  OptionResult,
  PublishVoteRequest,
  QuestionOption,
  QuestionResult,
  QuestionType,
  QuorumType,
  UpdateQuestionRequest,
  UpdateVoteRequest,
  Vote,
  VoteAuditAction,
  VoteAuditLog,
  VoteComment,
  VoteCommentWithUser,
  VoteEligibility,
  VoteQuestion,
  VoteReceipt,
  VoteResponse,
  VoteResults,
  VoteStatus,
  VoteSummary,
  VoteWithDetails,
} from './types';
