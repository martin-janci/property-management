/**
 * Agent / realtor public profile page — real-data regression test.
 *
 * The page used to render a hardcoded `MOCK_AGENT` for every `[id]` (and a
 * contact form that faked a successful submit). This test pins the corrected
 * behaviour: the page fetches the realtor by id from the real reality-server
 * endpoint (`GET /api/v1/realtors/{id}/profile` via `getRealtorPublicProfile`)
 * and:
 *   1. renders the fetched profile's real data, and
 *   2. shows a not-found state (never fabricated data) when the API 404s.
 *
 * It fails against the old mock-only implementation — which ignored the id and
 * always rendered "Ing. Petra Horáčková" — because that page neither fetches
 * nor surfaces a not-found state.
 */
/// <reference types="vitest/globals" />

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { RealtorApiError } from '@/lib/realtor-api';
import AgentProfilePage from './page';

vi.mock('next/navigation', () => ({
  useParams: () => ({ id: 'realtor-abc' }),
}));

vi.mock('next-intl', () => ({
  useLocale: () => 'sk',
}));

vi.mock('@/components/ui', () => ({
  Header: () => null,
  Footer: () => null,
}));

const getRealtorPublicProfile = vi.fn();
const getRealtorReviews = vi.fn();

vi.mock('@/lib/realtor-api', async () => {
  const actual = await vi.importActual<typeof import('@/lib/realtor-api')>('@/lib/realtor-api');
  return {
    ...actual,
    getRealtorPublicProfile: (id: string) => getRealtorPublicProfile(id),
    getRealtorReviews: (id: string) => getRealtorReviews(id),
  };
});

function renderPage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <AgentProfilePage />
    </QueryClientProvider>
  );
}

describe('AgentProfilePage real-data fetch', () => {
  it('renders the fetched realtor profile instead of a hardcoded mock', async () => {
    getRealtorPublicProfile.mockResolvedValue({
      id: 'p-1',
      user_id: 'realtor-abc',
      photo_url: null,
      bio: 'Špecializujem sa na byty v Košiciach.',
      tagline: 'Ing. Jozef Testovací',
      specializations: ['Rezidenčné'],
      languages: ['Slovenčina'],
      license_number: 'SK-REA-2024-0001',
      license_verified_at: '2024-02-01T00:00:00Z',
      phone: '+421 900 000 000',
      email_public: 'jozef@example.sk',
      show_phone: true,
      show_email: true,
      accept_inquiries: true,
      total_listings: 5,
      active_listings: 3,
      total_views: 120,
      total_inquiries: 7,
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-02-01T00:00:00Z',
    });
    getRealtorReviews.mockResolvedValue({ reviews: [], total: 0, avg_rating: null });

    renderPage();

    // Real fetched identity is shown; the old hardcoded mock name is not.
    expect(await screen.findByText('Ing. Jozef Testovací')).toBeInTheDocument();
    expect(screen.queryByText(/Horáčková/)).toBeNull();
    expect(screen.getByText('jozef@example.sk')).toBeInTheDocument();
  });

  it('shows a not-found state when the realtor does not exist (no fabricated data)', async () => {
    getRealtorPublicProfile.mockRejectedValue(new RealtorApiError('Profile not found', 404));
    getRealtorReviews.mockResolvedValue({ reviews: [], total: 0, avg_rating: null });

    renderPage();

    expect(await screen.findByText('Maklér sa nenašiel')).toBeInTheDocument();
    expect(screen.queryByText(/Horáčková/)).toBeNull();
  });
});
