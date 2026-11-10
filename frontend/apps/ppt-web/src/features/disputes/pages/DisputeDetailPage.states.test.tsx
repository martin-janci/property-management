/// <reference types="vitest/globals" />
/**
 * DisputeDetailPage renders loading / error / not-found states before it reads
 * any `dispute.*` field (round-3 audit #3: "Cannot read properties of
 * undefined (reading 'status')" when the dispute query failed).
 */
import { fireEvent, render, screen } from '@testing-library/react';
import { DisputeDetailPage } from './DisputeDetailPage';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: 'en' } }),
}));

const noop = () => {};

function renderWithoutDispute(props: {
  isLoading?: boolean;
  isError?: boolean;
  onBack?: () => void;
}) {
  return render(
    <DisputeDetailPage
      dispute={undefined}
      parties={[]}
      evidence={[]}
      timeline={[]}
      resolutions={[]}
      actionItems={[]}
      onBack={props.onBack ?? noop}
      onUpdateStatus={noop}
      onDeleteEvidence={noop}
      onProposeResolution={noop}
      onVoteResolution={noop}
      onAcceptResolution={noop}
      onImplementResolution={noop}
      onCreateAction={noop}
      onCompleteAction={noop}
      onSendReminder={noop}
      onEscalate={noop}
      onNavigateToMediation={noop}
      isLoading={props.isLoading}
      isError={props.isError}
    />
  );
}

describe('DisputeDetailPage without a dispute', () => {
  it('shows a spinner while loading', () => {
    renderWithoutDispute({ isLoading: true });
    expect(screen.getAllByRole('status').length).toBeGreaterThan(0);
    expect(screen.queryByText('errors.disputeNotFound')).toBeNull();
  });

  it('shows the load-error state when the query failed', () => {
    renderWithoutDispute({ isError: true });
    expect(screen.getByText('errors.errorLoadingDispute')).toBeTruthy();
    expect(screen.getByText('errors.disputeLoadError')).toBeTruthy();
  });

  it('shows not-found when nothing loaded and there was no error', () => {
    const onBack = vi.fn();
    renderWithoutDispute({ onBack });
    expect(screen.getByText('errors.disputeNotFound')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'common.backToDisputes' }));
    expect(onBack).toHaveBeenCalled();
  });
});
