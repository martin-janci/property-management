/// <reference types="vitest/globals" />
/**
 * MediationTimelineView component tests (fix-569).
 * Tests aria-labels on timeline event icons.
 */
import { render, screen } from '@testing-library/react';
import { MediationTimelineView } from './MediationTimelineView';

vi.mock('@ppt/api-client', () => ({
  useDisputeTimeline: vi.fn(),
  useDispute: () => ({
    data: {
      parties: [
        { userId: 'u-john', userName: 'John Doe' },
        { userId: 'u-jane', userName: 'Jane Smith' },
      ],
    },
  }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'common.loading': 'Loading',
        'disputes.mediation.noTimelineEvents': 'No timeline events yet.',
        'disputes.mediation.timeline.icon.disputeFiled': 'Dispute filed',
        'disputes.mediation.timeline.icon.statusChanged': 'Status changed',
        'disputes.mediation.timeline.icon.partyAdded': 'Party added',
        'disputes.mediation.timeline.icon.evidenceAdded': 'Evidence added',
        'disputes.mediation.timeline.icon.noteAdded': 'Note added',
        'disputes.mediation.timeline.icon.meetingScheduled': 'Meeting scheduled',
        'disputes.mediation.timeline.icon.resolutionProposed': 'Resolution proposed',
        'disputes.mediation.timeline.icon.resolutionAccepted': 'Resolution accepted',
        'disputes.mediation.timeline.icon.escalated': 'Escalated',
        'disputes.mediation.timeline.icon.closed': 'Closed',
      };
      return map[key] ?? key;
    },
    i18n: { language: 'en' },
  }),
}));

vi.mock('../utils/formatTime', () => ({
  formatTime: (_date: string, _lang: string) => '2 hours ago',
}));

import { useDisputeTimeline } from '@ppt/api-client';

const mockUseDisputeTimeline = vi.mocked(useDisputeTimeline);

describe('MediationTimelineView', () => {
  it('shows loading spinner', () => {
    mockUseDisputeTimeline.mockReturnValue({ data: [], isLoading: true } as ReturnType<
      typeof useDisputeTimeline
    >);
    render(<MediationTimelineView disputeId="dsp-1" />);
    expect(screen.getByRole('status', { name: 'Loading' })).toBeInTheDocument();
  });

  it('shows empty state message', () => {
    mockUseDisputeTimeline.mockReturnValue({ data: [], isLoading: false } as ReturnType<
      typeof useDisputeTimeline
    >);
    render(<MediationTimelineView disputeId="dsp-1" />);
    expect(screen.getByText('No timeline events yet.')).toBeInTheDocument();
  });

  it('renders timeline events with aria-labels on icons', () => {
    mockUseDisputeTimeline.mockReturnValue({
      data: [
        {
          id: 'evt-1',
          eventType: 'dispute_filed',
          actorId: 'u-john',
          description: 'Dispute was filed',
          createdAt: '2026-05-01T10:00:00Z',
          metadata: null,
        },
        {
          id: 'evt-2',
          eventType: 'party_added',
          actorId: 'u-jane',
          description: 'Mediator added',
          createdAt: '2026-05-02T10:00:00Z',
          metadata: null,
        },
      ],
      isLoading: false,
    } as ReturnType<typeof useDisputeTimeline>);

    render(<MediationTimelineView disputeId="dsp-1" />);

    expect(screen.getByLabelText('Dispute filed')).toBeInTheDocument();
    expect(screen.getByLabelText('Party added')).toBeInTheDocument();
    // Actor names are resolved from the dispute parties by actor id.
    expect(screen.getByText('John Doe')).toBeInTheDocument();
    expect(screen.getByText('Jane Smith')).toBeInTheDocument();
    expect(screen.getByText('Dispute was filed')).toBeInTheDocument();
  });

  it('renders status change metadata when present', () => {
    mockUseDisputeTimeline.mockReturnValue({
      data: [
        {
          id: 'evt-1',
          eventType: 'status_changed',
          actorId: 'u-unknown',
          description: 'Status updated',
          createdAt: '2026-05-01T10:00:00Z',
          metadata: { oldStatus: 'filed', newStatus: 'under_review' },
        },
      ],
      isLoading: false,
    } as ReturnType<typeof useDisputeTimeline>);

    render(<MediationTimelineView disputeId="dsp-1" />);
    expect(screen.getByLabelText('Status changed')).toBeInTheDocument();
    expect(screen.getByText(/oldStatus/)).toBeInTheDocument();
    expect(screen.getByText('filed')).toBeInTheDocument();
  });
});
