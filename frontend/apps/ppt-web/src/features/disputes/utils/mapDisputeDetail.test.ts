/**
 * Regression tests for identity resolution in mapDisputeDetailForUi.
 *
 * Issue #3312: a resolution term's `responsiblePartyId` references a
 * `dispute_parties` row id (same referent as a vote's `partyId`), so it must be
 * resolved against the party-id map, not the user-id map. The previous code
 * resolved it via `userName` (user-id map), so every term's responsible party
 * silently fell back to the unknown-name label.
 */

import type { DisputeWithDetails, ResolutionWithVotes } from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import { mapDisputeDetailForUi } from './mapDisputeDetail';

const PARTY_ID = '00000000-0000-0000-0000-0000000000a1';
const USER_ID = '00000000-0000-0000-0000-0000000000b2';

function disputeWithOneParty(): DisputeWithDetails {
  return {
    id: 'd1',
    organizationId: 'org1',
    referenceNumber: 'DIS-1',
    category: 'noise',
    title: 'Title',
    description: 'Desc',
    status: 'open',
    priority: 'medium',
    filedBy: USER_ID,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    evidenceCount: 0,
    activityCount: 0,
    sessionCount: 0,
    parties: [
      {
        id: PARTY_ID,
        userId: USER_ID,
        role: 'respondent',
        userName: 'Pat Party',
        userEmail: 'pat@example.com',
        createdAt: '2026-01-01T00:00:00Z',
      },
    ],
  };
}

function resolutionWithTermAndVote(): ResolutionWithVotes {
  return {
    resolution: {
      id: 'r1',
      disputeId: 'd1',
      proposedBy: USER_ID,
      resolutionText: 'Text',
      terms: [
        {
          id: 't1',
          description: 'Do the thing',
          // Backend writes the dispute_parties row id here, NOT the user id.
          responsiblePartyId: PARTY_ID,
          completed: false,
        },
      ],
      status: 'proposed',
      proposedAt: '2026-01-02T00:00:00Z',
      createdAt: '2026-01-02T00:00:00Z',
      updatedAt: '2026-01-02T00:00:00Z',
    },
    votes: [
      {
        id: 'v1',
        resolutionId: 'r1',
        partyId: PARTY_ID,
        accepted: true,
        votedAt: '2026-01-03T00:00:00Z',
      },
    ],
    acceptanceRate: 1,
  };
}

describe('mapDisputeDetailForUi — resolution-term responsible party', () => {
  it('resolves responsiblePartyId via the party-id map (like votes), not the user-id map', () => {
    const ui = mapDisputeDetailForUi({
      dispute: disputeWithOneParty(),
      resolutions: [resolutionWithTermAndVote()],
      unknownName: 'Unknown',
    });

    const term = ui.resolutions[0]?.terms[0];
    expect(term?.responsiblePartyId).toBe(PARTY_ID);
    // The party row id is on the roster, so the real name must render.
    expect(term?.responsiblePartyName).toBe('Pat Party');
    expect(term?.responsiblePartyName).not.toBe('Unknown');

    // Votes resolve the same party id to the same name — the precedent.
    expect(ui.resolutions[0]?.votes[0]?.partyName).toBe('Pat Party');
  });

  it('falls back to the unknown label when the responsible party id is not on the roster', () => {
    const resolution = resolutionWithTermAndVote();
    resolution.resolution.terms[0].responsiblePartyId = 'ffffffff-ffff-ffff-ffff-ffffffffffff';

    const ui = mapDisputeDetailForUi({
      dispute: disputeWithOneParty(),
      resolutions: [resolution],
      unknownName: 'Unknown',
    });

    expect(ui.resolutions[0]?.terms[0]?.responsiblePartyName).toBe('Unknown');
  });

  it('leaves responsiblePartyName undefined when the term has no responsible party', () => {
    const resolution = resolutionWithTermAndVote();
    resolution.resolution.terms[0].responsiblePartyId = undefined;

    const ui = mapDisputeDetailForUi({
      dispute: disputeWithOneParty(),
      resolutions: [resolution],
      unknownName: 'Unknown',
    });

    expect(ui.resolutions[0]?.terms[0]?.responsiblePartyName).toBeUndefined();
  });
});
