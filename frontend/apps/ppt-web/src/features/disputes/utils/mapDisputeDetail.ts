/**
 * Maps the api-client dispute detail data (backend-aligned, camelCase) onto the
 * props DisputeDetailPage renders. The backend returns user ids only on
 * evidence, activities, resolutions, votes and action items, so display names
 * are resolved from the dispute's party roster (which carries names/emails).
 */
import type {
  DisputeEvidence as ApiDisputeEvidence,
  DisputeActionItem,
  DisputeWithDetails,
  ResolutionWithVotes,
  TimelineEvent,
} from '@ppt/api-client';
import type { ActionItem } from '../components/ActionItemCard';
import type { TimelineEntry } from '../components/DisputeTimeline';
import type { Resolution, ResolutionVote } from '../components/ResolutionCard';
import type { DisputeDetail, DisputeEvidence, DisputeParty } from '../pages/DisputeDetailPage';

export interface DisputeDetailUiInput {
  dispute?: DisputeWithDetails;
  timeline?: TimelineEvent[];
  evidence?: ApiDisputeEvidence[];
  resolutions?: ResolutionWithVotes[];
  actions?: DisputeActionItem[];
  currentUserId?: string;
  /** Fallback label for a user id that is not on the party roster. */
  unknownName: string;
}

export interface DisputeDetailUi {
  dispute?: DisputeDetail;
  parties: DisputeParty[];
  evidence: DisputeEvidence[];
  timeline: TimelineEntry[];
  resolutions: Array<Resolution & { votes: ResolutionVote[]; acceptanceRate: number }>;
  actionItems: ActionItem[];
  /** Caller is a complainant / respondent / witness on this dispute. */
  isParty: boolean;
  /** Caller is the dispute's mediator party. */
  isMediator: boolean;
}

export function mapDisputeDetailForUi(input: DisputeDetailUiInput): DisputeDetailUi {
  const { dispute, currentUserId, unknownName } = input;
  const parties: DisputeParty[] = (dispute?.parties ?? []).map((p) => ({
    id: p.id,
    userId: p.userId,
    userName: p.userName,
    userEmail: p.userEmail,
    role: p.role,
    notifiedAt: p.notifiedAt,
    respondedAt: p.respondedAt,
  }));
  const nameByUser = new Map(parties.map((p) => [p.userId, p.userName]));
  const nameByParty = new Map(parties.map((p) => [p.id, p.userName]));
  const userName = (id?: string) => (id ? (nameByUser.get(id) ?? unknownName) : unknownName);

  const mine = parties.filter((p) => !!currentUserId && p.userId === currentUserId);
  const mediator = parties.find((p) => p.role === 'mediator');

  const uiDispute: DisputeDetail | undefined = dispute
    ? {
        id: dispute.id,
        organizationId: dispute.organizationId,
        buildingId: dispute.buildingId,
        unitId: dispute.unitId,
        referenceNumber: dispute.referenceNumber,
        category: dispute.category,
        title: dispute.title,
        description: dispute.description,
        desiredResolution: dispute.desiredResolution,
        status: dispute.status,
        priority: dispute.priority,
        filedBy: dispute.filedBy,
        filedByName: userName(dispute.filedBy),
        assignedTo: dispute.assignedTo ?? mediator?.userId,
        assignedToName: dispute.assignedTo ? userName(dispute.assignedTo) : mediator?.userName,
        createdAt: dispute.createdAt,
        updatedAt: dispute.updatedAt,
      }
    : undefined;

  return {
    dispute: uiDispute,
    parties,
    evidence: (input.evidence ?? []).map((e) => ({
      id: e.id,
      uploadedBy: e.uploadedBy,
      uploaderName: userName(e.uploadedBy),
      filename: e.filename,
      originalFilename: e.originalFilename,
      contentType: e.contentType,
      sizeBytes: e.sizeBytes,
      storageUrl: e.storageUrl,
      description: e.description,
      createdAt: e.createdAt,
    })),
    timeline: (input.timeline ?? []).map((ev) => ({
      id: ev.id,
      actorId: ev.actorId,
      actorName: userName(ev.actorId),
      activityType: ev.eventType,
      description: ev.description,
      metadata: ev.metadata,
      createdAt: ev.createdAt,
    })),
    resolutions: (input.resolutions ?? []).map(({ resolution: r, votes, acceptanceRate }) => ({
      id: r.id,
      disputeId: r.disputeId,
      proposedBy: r.proposedBy,
      proposedByName: userName(r.proposedBy),
      resolutionText: r.resolutionText,
      terms: r.terms.map((term) => ({
        id: term.id,
        description: term.description,
        responsiblePartyId: term.responsiblePartyId,
        // responsiblePartyId references a dispute_parties row id (same referent
        // as a vote's partyId — issue #3312), so resolve it via the party-id
        // map, not the user-id map.
        responsiblePartyName: term.responsiblePartyId
          ? (nameByParty.get(term.responsiblePartyId) ?? unknownName)
          : undefined,
        deadline: term.deadline,
        completed: term.completed,
        completedAt: term.completedAt,
      })),
      status: r.status,
      proposedAt: r.proposedAt,
      acceptedAt: r.acceptedAt,
      implementedAt: r.implementedAt,
      votes: votes.map((v) => ({
        partyId: v.partyId,
        partyName: nameByParty.get(v.partyId) ?? unknownName,
        accepted: v.accepted,
        comments: v.comments,
        votedAt: v.votedAt,
      })),
      acceptanceRate,
    })),
    actionItems: (input.actions ?? []).map((a) => ({
      id: a.id,
      disputeId: a.disputeId,
      resolutionId: a.resolutionId,
      resolutionTermId: a.resolutionTermId,
      assignedTo: a.assignedTo,
      assignedToName: userName(a.assignedTo),
      title: a.title,
      description: a.description,
      dueDate: a.dueDate,
      status: a.status,
      completedAt: a.completedAt,
      completionNotes: a.completionNotes,
      reminderSentAt: a.reminderSentAt,
      escalatedAt: a.escalatedAt,
    })),
    isParty: mine.some((p) => p.role !== 'mediator'),
    isMediator: mine.some((p) => p.role === 'mediator'),
  };
}
