/// <reference types="vitest/globals" />
/**
 * IntegrationMarketplacePage tests (Epic 150, Story 150.1 — dx-integration-marketplace-stubs).
 *
 * The page previously rendered a hard-coded `sampleIntegrations` array. These
 * tests pin the new behaviour: the page now sources its data from the backend
 * via `useMarketplaceIntegrations`, and covers the loading, error, populated,
 * empty, and filter paths.
 */

import type { MarketplaceIntegrationSummary } from '@ppt/api-client';
import { fireEvent, render, screen } from '@testing-library/react';
import { IntegrationMarketplacePage } from './IntegrationMarketplacePage';

vi.mock('@ppt/api-client', () => ({
  useMarketplaceIntegrations: vi.fn(),
}));

import { useMarketplaceIntegrations } from '@ppt/api-client';

const mockUseMarketplace = vi.mocked(useMarketplaceIntegrations);

function summary(
  overrides: Partial<MarketplaceIntegrationSummary> = {}
): MarketplaceIntegrationSummary {
  return {
    id: 'id-1',
    slug: 'quickbooks',
    name: 'QuickBooks',
    description: 'Sync invoices and payments.',
    category: 'accounting',
    icon_url: null,
    vendor_name: 'Intuit',
    status: 'available',
    rating_average: 4.5,
    rating_count: 128,
    install_count: 1542,
    is_featured: true,
    is_premium: false,
    ...overrides,
  };
}

// biome-ignore lint/suspicious/noExplicitAny: test stub intentionally partial
function queryResult(overrides: Record<string, unknown> = {}): any {
  return {
    data: undefined,
    isLoading: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
    ...overrides,
  };
}

afterEach(() => {
  vi.clearAllMocks();
});

it('shows a loading state while integrations are being fetched', () => {
  mockUseMarketplace.mockReturnValue(queryResult({ isLoading: true }));
  render(<IntegrationMarketplacePage />);
  expect(screen.getByText(/loading integrations/i)).toBeInTheDocument();
});

it('shows an error state with a retry button when the request fails', () => {
  const refetch = vi.fn();
  mockUseMarketplace.mockReturnValue(
    queryResult({ isError: true, error: new Error('boom'), refetch })
  );
  render(<IntegrationMarketplacePage />);

  expect(screen.getByText(/failed to load integrations/i)).toBeInTheDocument();
  expect(screen.getByText('boom')).toBeInTheDocument();

  fireEvent.click(screen.getByRole('button', { name: /retry/i }));
  expect(refetch).toHaveBeenCalledTimes(1);
});

it('renders integrations returned by the backend', () => {
  mockUseMarketplace.mockReturnValue(
    queryResult({
      data: [
        summary({ id: 'a', name: 'QuickBooks', vendor_name: 'Intuit' }),
        summary({ id: 'b', name: 'Salesforce', vendor_name: 'Salesforce', category: 'crm' }),
      ],
    })
  );
  render(<IntegrationMarketplacePage />);

  expect(screen.getByText('QuickBooks')).toBeInTheDocument();
  expect(screen.getByText('Salesforce')).toBeInTheDocument();
  expect(screen.getByText(/showing 2 integrations/i)).toBeInTheDocument();
});

it('filters the rendered integrations by the search box', () => {
  mockUseMarketplace.mockReturnValue(
    queryResult({
      data: [
        summary({ id: 'a', name: 'QuickBooks', vendor_name: 'Intuit' }),
        summary({ id: 'b', name: 'Salesforce', vendor_name: 'Salesforce' }),
      ],
    })
  );
  render(<IntegrationMarketplacePage />);

  fireEvent.change(screen.getByPlaceholderText(/search integrations/i), {
    target: { value: 'salesforce' },
  });

  expect(screen.queryByText('QuickBooks')).not.toBeInTheDocument();
  expect(screen.getByText('Salesforce')).toBeInTheDocument();
  expect(screen.getByText(/showing 1 integration\b/i)).toBeInTheDocument();
});

it('shows the empty state when the backend returns no integrations', () => {
  mockUseMarketplace.mockReturnValue(queryResult({ data: [] }));
  render(<IntegrationMarketplacePage />);
  expect(screen.getByText(/no integrations found/i)).toBeInTheDocument();
});
