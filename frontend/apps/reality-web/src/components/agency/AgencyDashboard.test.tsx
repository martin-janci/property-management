/**
 * AgencyDashboard Component Tests
 *
 * Regression for Issue #2277: on an API failure `useMyAgency` settles with
 * `isLoading=false` and `data=undefined`. The dashboard used to fall through
 * to <NoAgencyMessage /> ("No Agency Found / Create Agency"), showing an actual
 * agency owner a misleading empty state. It must now render a distinct
 * error/retry state instead, and only show "No Agency Found" when the query
 * genuinely succeeds with no agency.
 */

import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock @ppt/reality-api-client before importing the component.
vi.mock('@ppt/reality-api-client', () => ({
  useMyAgency: vi.fn(),
  useAgencyStats: vi.fn(() => ({ data: undefined, isLoading: false, isError: false })),
  useAgencyPerformance: vi.fn(() => ({ data: undefined, isError: false })),
  useRealtors: vi.fn(() => ({ data: [], isError: false })),
}));

import { useMyAgency } from '@ppt/reality-api-client';
import { AgencyDashboard } from './AgencyDashboard';

const mockUseMyAgency = vi.mocked(useMyAgency);

describe('AgencyDashboard — agency load error handling (Issue #2277)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the error/retry state (not "No Agency Found") when the agency query fails', () => {
    mockUseMyAgency.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      refetch: vi.fn(),
    } as unknown as ReturnType<typeof useMyAgency>);

    render(<AgencyDashboard />);

    // The distinct error state is present…
    expect(screen.getByRole('alert')).toHaveTextContent('loadErrorTitle');
    // …and the misleading empty-state must NOT be shown on a fetch failure.
    expect(screen.queryByText(/No Agency Found/i)).not.toBeInTheDocument();
  });

  it('calls refetch when the retry button is clicked', () => {
    const refetch = vi.fn();
    mockUseMyAgency.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      refetch,
    } as unknown as ReturnType<typeof useMyAgency>);

    render(<AgencyDashboard />);

    fireEvent.click(screen.getByRole('button', { name: 'retry' }));
    expect(refetch).toHaveBeenCalledOnce();
  });

  it('still shows the "No Agency Found" empty state when the query succeeds with no agency', () => {
    mockUseMyAgency.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    } as unknown as ReturnType<typeof useMyAgency>);

    render(<AgencyDashboard />);

    expect(screen.getByText(/No Agency Found/i)).toBeInTheDocument();
    expect(screen.queryByText('loadErrorTitle')).not.toBeInTheDocument();
  });

  it('renders the dashboard when the agency loads successfully', () => {
    mockUseMyAgency.mockReturnValue({
      data: { id: 'agency-1', name: 'Acme Realty' },
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    } as unknown as ReturnType<typeof useMyAgency>);

    render(<AgencyDashboard />);

    expect(screen.getByRole('heading', { name: 'Acme Realty' })).toBeInTheDocument();
    expect(screen.queryByText(/No Agency Found/i)).not.toBeInTheDocument();
  });
});
