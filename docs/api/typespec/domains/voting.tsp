import "../shared/models.tsp";
import "../shared/errors.tsp";
import "../shared/pagination.tsp";
import "../shared/tenant.tsp";

namespace PropertyManagement.Voting;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("Vote/Poll (UC-04)")
model Vote {
  id: uuid;
  buildingId: uuid;
  title: string;
  description: string;
  type: VoteType;
  status: VoteStatus;
  startDate: dateTime;
  endDate: dateTime;
  quorumPercentage: float64;
  options: VoteOption[];
  allowDelegation: boolean;
  isAnonymous: boolean;
  ...AuditableEntity;
}

enum VoteType {
  Simple: "simple",
  Weighted: "weighted",
  RankedChoice: "ranked_choice",
}

enum VoteStatus {
  Draft: "draft",
  Scheduled: "scheduled",
  Active: "active",
  Closed: "closed",
  Cancelled: "cancelled",
}

model VoteOption {
  id: uuid;
  text: string;
  description?: string;
  order: int32;
}

model VoteResult {
  voteId: uuid;
  totalEligibleVoters: int32;
  totalVotes: int32;
  participationRate: float64;
  quorumReached: boolean;
  optionResults: VoteOptionResult[];
  winningOptionId?: uuid;
}

model VoteOptionResult {
  optionId: uuid;
  text: string;
  voteCount: int32;
  weightedVotes: float64;
  percentage: float64;
}

model Ballot {
  id: uuid;
  voteId: uuid;
  voterId: uuid;
  delegatedFrom?: uuid;
  selectedOptionIds: uuid[];
  weight: float64;
  submittedAt: dateTime;
}

@route("/api/v1/votes")
@tag("Voting")
interface VotingApi {
  @get list(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    ...PaginationQuery,
    @query buildingId?: uuid,
    @query status?: VoteStatus
  ): PaginatedResponse<Vote>;

  @post create(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @body body: {
      buildingId: uuid;
      title: string;
      description: string;
      type: VoteType;
      startDate: dateTime;
      endDate: dateTime;
      quorumPercentage?: float64;
      options: { text: string; description?: string; }[];
      allowDelegation?: boolean;
      isAnonymous?: boolean;
    }
  ): { @statusCode statusCode: 201; @body body: Vote; };

  @get @route("/{id}") get(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Vote | NotFoundError;

  @patch @route("/{id}") update(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { title?: string; description?: string; startDate?: dateTime; endDate?: dateTime; }
  ): Vote;

  @post @route("/{id}/publish") publish(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Vote;

  @post @route("/{id}/cancel") cancel(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { reason: string; }
  ): Vote;

  @post @route("/{id}/cast") castVote(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { selectedOptionIds: uuid[]; }
  ): { @statusCode statusCode: 201; @body body: Ballot; };

  @get @route("/{id}/results") getResults(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): VoteResult | NotFoundError;

  @get @route("/{id}/my-ballot") getMyBallot(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Ballot | NotFoundError;
}
