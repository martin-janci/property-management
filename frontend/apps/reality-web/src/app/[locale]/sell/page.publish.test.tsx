/**
 * Regression test for the sell-wizard persistence gap
 * (code-review-reality-web-sell-wizard-no-persist).
 *
 * Before the fix, the Publish button flipped `submitted` to the success
 * screen WITHOUT ever calling the API — every seller submission was silently
 * discarded. These tests assert the Publish handler now POSTs via
 * `createListing`, only shows the success screen on a resolved 2xx, surfaces a
 * form-level error banner on failure (keeping the wizard open), and disables
 * the button while the request is in flight.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createListing, RealtorApiError } from '@/lib/realtor-api';
import SellPage from './page';

// Keep the real RealtorApiError (handlePublish uses `instanceof`); stub only
// the network call.
vi.mock('@/lib/realtor-api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/realtor-api')>();
  return { ...actual, createListing: vi.fn() };
});

// Override the global next-intl mock (src/test/setup.tsx) to add `t.rich`,
// which the step-5 terms label uses — the setup mock's `t` has no `.rich`.
vi.mock('next-intl', () => {
  const t = Object.assign((key: string) => key, { rich: (key: string) => key });
  return {
    useTranslations: () => t,
    useLocale: () => 'en',
    useMessages: () => ({}),
    NextIntlClientProvider: ({ children }: { children: React.ReactNode }) => children,
  };
});

// Header/Footer pull in the auth context (useAuth → AuthProvider), which is
// irrelevant to the wizard submit flow under test — stub them out.
vi.mock('@/components/ui', () => ({
  Header: () => null,
  Footer: () => null,
}));

const mockCreateListing = vi.mocked(createListing);

/** Drive the wizard through all 5 steps with valid data, stopping on step 5. */
function fillWizardToPublish() {
  // Step 1 — type + location (defaults: sale / apartment)
  fireEvent.change(screen.getByLabelText('fields.address'), { target: { value: 'Hlavna 1' } });
  fireEvent.change(screen.getByLabelText('fields.city'), { target: { value: 'Bratislava' } });
  fireEvent.click(screen.getByRole('button', { name: 'next' }));

  // Step 2 — details
  fireEvent.change(screen.getByLabelText('fields.area'), { target: { value: '65' } });
  fireEvent.change(screen.getByLabelText('fields.rooms'), { target: { value: '3' } });
  fireEvent.click(screen.getByRole('button', { name: 'next' }));

  // Step 3 — photos (no required fields)
  fireEvent.click(screen.getByRole('button', { name: 'next' }));

  // Step 4 — price
  fireEvent.change(screen.getByLabelText('fields.price'), { target: { value: '120000' } });
  fireEvent.click(screen.getByRole('button', { name: 'next' }));

  // Step 5 — contact + terms
  fireEvent.change(screen.getByLabelText('fields.contactName'), { target: { value: 'Jane Doe' } });
  fireEvent.change(screen.getByLabelText('fields.contactPhone'), {
    target: { value: '+421 900 000 000' },
  });
  fireEvent.change(screen.getByLabelText('fields.contactEmail'), {
    target: { value: 'jane@example.com' },
  });
  fireEvent.click(screen.getByRole('checkbox'));
}

describe('SellPage — Publish persists the listing', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('POSTs the mapped payload via createListing exactly once', async () => {
    mockCreateListing.mockResolvedValue({ id: 'lst_1' } as never);
    render(<SellPage />);
    fillWizardToPublish();

    fireEvent.click(screen.getByRole('button', { name: 'publish' }));

    await waitFor(() => expect(mockCreateListing).toHaveBeenCalledTimes(1));
    expect(mockCreateListing).toHaveBeenCalledWith(
      expect.objectContaining({
        transactionType: 'sale',
        propertyType: 'apartment',
        street: 'Hlavna 1',
        city: 'Bratislava',
        area: 65,
        rooms: 3,
        price: 120000,
        currency: 'EUR',
        isNegotiable: false,
      })
    );
  });

  it('shows the success screen only after a resolved 2xx', async () => {
    mockCreateListing.mockResolvedValue({ id: 'lst_1' } as never);
    render(<SellPage />);
    fillWizardToPublish();

    // Success screen is not shown before the request resolves.
    expect(screen.queryByText('submittedTitle')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'publish' }));

    expect(await screen.findByText('submittedTitle')).toBeInTheDocument();
  });

  it('renders an error banner and stays on the wizard when the POST fails', async () => {
    mockCreateListing.mockRejectedValue(new RealtorApiError('nope', 500));
    render(<SellPage />);
    fillWizardToPublish();

    fireEvent.click(screen.getByRole('button', { name: 'publish' }));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('nope');
    // Wizard remains — the success screen was NOT shown.
    expect(screen.queryByText('submittedTitle')).not.toBeInTheDocument();
  });

  it('disables the Publish button while the request is in flight', async () => {
    let resolve: (v: unknown) => void = () => {};
    mockCreateListing.mockReturnValue(
      new Promise((r) => {
        resolve = r;
      }) as never
    );
    render(<SellPage />);
    fillWizardToPublish();

    fireEvent.click(screen.getByRole('button', { name: 'publish' }));

    await waitFor(() => {
      const btn = screen.getByRole('button', { name: 'publishing' });
      expect(btn).toBeDisabled();
    });

    resolve({ id: 'lst_1' }); // let the pending request settle for cleanup
  });
});
