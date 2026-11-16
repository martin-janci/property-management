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
import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { RealtorApiError } from '@/lib/realtor-api';
import skMessages from '../../../../../messages/sk.json';
import AgentProfilePage from './page';

vi.mock('next/navigation', () => ({
  useParams: () => ({ id: 'realtor-abc' }),
}));

// Resolve translations against the real sk.json so the assertions below pin
// the actual rendered copy (not raw keys) — this guards that the page is wired
// through next-intl AND that the sk message bundle carries every key used.
vi.mock('next-intl', () => ({
  useLocale: () => 'sk',
  useTranslations: (namespace: string) => {
    const node = namespace
      .split('.')
      .reduce<unknown>((acc, part) => (acc as Record<string, unknown>)?.[part], skMessages);
    return (key: string, values?: Record<string, unknown>) => {
      let str = (node as Record<string, string> | undefined)?.[key] ?? key;
      if (values) {
        for (const [k, v] of Object.entries(values)) {
          str = str.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v));
        }
      }
      return str;
    };
  },
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

/**
 * Contact-channel honesty — the headline behaviour PR #3225 introduced when it
 * removed the fake "submit" form. The page must:
 *   - never offer a contact action when the realtor exposes no channel,
 *   - surface a real `tel:` link (not an email form) for phone-only realtors,
 *   - hand off to a `mailto:` carrying the realtor's real public email.
 * These pin the gating logic (`show_phone`/`show_email` + `email_public`) so a
 * future edit can't silently re-introduce a fabricated/dead contact path.
 */
function makeProfile(overrides: Record<string, unknown> = {}) {
  return {
    id: 'p-1',
    user_id: 'realtor-abc',
    photo_url: null,
    bio: null,
    tagline: 'Ing. Jozef Testovací',
    specializations: [],
    languages: [],
    license_number: null,
    license_verified_at: null,
    phone: null,
    email_public: null,
    show_phone: false,
    show_email: false,
    accept_inquiries: false,
    total_listings: 0,
    active_listings: 0,
    total_views: 0,
    total_inquiries: 0,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-02-01T00:00:00Z',
    ...overrides,
  };
}

describe('AgentProfilePage contact channels', () => {
  it('disables the contact button when no channel is exposed and inquiries are off', async () => {
    getRealtorPublicProfile.mockResolvedValue(makeProfile());
    getRealtorReviews.mockResolvedValue({ reviews: [], total: 0, avg_rating: null });

    renderPage();

    const contactBtn = await screen.findByRole('button', { name: 'Kontaktovať' });
    expect(contactBtn).toBeDisabled();
  });

  it('opens a phone-only contact modal with a tel: link and no email form', async () => {
    getRealtorPublicProfile.mockResolvedValue(
      makeProfile({ show_phone: true, phone: '+421 900 111 222', accept_inquiries: true })
    );
    getRealtorReviews.mockResolvedValue({ reviews: [], total: 0, avg_rating: null });

    renderPage();

    const contactBtn = await screen.findByRole('button', { name: 'Kontaktovať' });
    expect(contactBtn).not.toBeDisabled();
    fireEvent.click(contactBtn);

    const dialog = await screen.findByRole('dialog', { name: 'Kontaktovať makléra' });
    // Phone-only realtors get a real tel: hand-off, never the email textarea.
    expect(within(dialog).queryByRole('textbox')).toBeNull();
    const telLink = within(dialog).getByRole('link');
    expect(telLink).toHaveAttribute('href', 'tel:+421 900 111 222');
  });

  it('hands off to a mailto: carrying the realtor public email (never a fake submit)', async () => {
    getRealtorPublicProfile.mockResolvedValue(
      makeProfile({ show_email: true, email_public: 'jozef@example.sk', accept_inquiries: true })
    );
    getRealtorReviews.mockResolvedValue({ reviews: [], total: 0, avg_rating: null });

    renderPage();

    const contactBtn = await screen.findByRole('button', { name: 'Kontaktovať' });
    fireEvent.click(contactBtn);

    const dialog = await screen.findByRole('dialog', { name: 'Kontaktovať makléra' });
    // Typing a message must flow into the mailto body (encoded), not a POST.
    fireEvent.change(within(dialog).getByRole('textbox'), { target: { value: 'Ahoj & test' } });

    const sendLink = within(dialog).getByRole('link', { name: /Odoslať e-mail/ });
    const href = sendLink.getAttribute('href') ?? '';
    expect(href.startsWith('mailto:jozef@example.sk')).toBe(true);
    expect(href).toContain(`body=${encodeURIComponent('Ahoj & test')}`);
  });
});

/**
 * Reviews error honesty — regression for the code-review finding that the
 * `realtor-reviews` useQuery error branch was swallowed: when the reviews
 * request failed, `reviewsData` stayed undefined and the Reviews section
 * silently rendered the "no reviews yet" empty state, masking the failure.
 * The page must instead surface the error (an alert) while the rest of the
 * already-loaded profile keeps rendering.
 *
 * Fails against the pre-fix page (which destructured only `data` from the
 * reviews query): no `role="alert"` is rendered and the empty-state copy
 * ("Zatiaľ žiadne hodnotenia.") is shown for a *failed* request.
 */
describe('AgentProfilePage reviews error', () => {
  it('surfaces the reviews load error instead of a silent empty state', async () => {
    getRealtorPublicProfile.mockResolvedValue(makeProfile());
    getRealtorReviews.mockRejectedValue(new RealtorApiError('Reviews unavailable', 500));

    renderPage();

    // The already-loaded profile still renders.
    expect(await screen.findByText('Ing. Jozef Testovací')).toBeInTheDocument();

    // The reviews failure is surfaced as an alert carrying the real i18n copy…
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent(skMessages.pages.agentProfile.reviewsError);

    // …and must NOT masquerade as a successful "no reviews yet" empty state.
    expect(screen.queryByText(skMessages.pages.agentProfile.noReviews)).toBeNull();
  });
});
