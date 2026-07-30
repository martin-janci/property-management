/**
 * ListingDetailContent Tests
 *
 * `ListingDetailContent` renders a guaranteed-shape `ListingDetail | null`:
 * `getListing` runs every 200 body through `parseListingDetail`, which validates
 * required fields and coerces `features` / `photos`. The malformed / wrong-typed
 * body crash-class coverage (#2276 / #2281 / #2341) now lives at that single
 * normalizer — see `listingSchema.test.ts`. These tests assert the happy path
 * and the null (not-found) fallback.
 */

import type { ListingDetail } from '@ppt/reality-api-client';
import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ResolvedScreen } from '../../lib/layout';
import { ListingDetailContent } from './ListingDetailContent';

// ---------------------------------------------------------------------------
// Mock useListingPreviewLayout so we can inject a pushed layout in tests
// ---------------------------------------------------------------------------
const mockPreviewHook = vi.fn();

vi.mock('./useListingPreviewLayout', () => ({
  useListingPreviewLayout: (...args: unknown[]) => mockPreviewHook(...args),
}));

// ContactForm (rendered in the sidebar) uses a TanStack Query mutation hook.
// Stub the API client so the component tree renders without a QueryClient.
vi.mock('@ppt/reality-api-client', () => ({
  useCreateInquiry: () => ({
    mutateAsync: vi.fn(),
    isPending: false,
    isError: false,
  }),
}));

// Header/Footer pull in the i18n routing + auth-context tree, which is
// irrelevant to the partial-body render behaviour under test.
vi.mock('@/components/ui', () => ({
  Header: () => null,
  Footer: () => null,
}));

const validListing: ListingDetail = {
  id: 'listing-1',
  slug: 'beautiful-apartment',
  title: 'Beautiful Apartment',
  propertyType: 'apartment',
  transactionType: 'sale',
  status: 'active',
  price: 150000,
  currency: 'EUR',
  area: 75,
  rooms: 3,
  address: {
    street: 'Main St 1',
    city: 'Bratislava',
    district: 'Old Town',
    postalCode: '81101',
    country: 'SK',
  },
  isFeatured: false,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  description: 'A lovely place to live.',
  features: { balcony: true, parking: false },
  photos: [
    {
      id: 'p1',
      url: 'https://cdn.example.com/p1.jpg',
      thumbnailUrl: 'https://cdn.example.com/p1-thumb.jpg',
      isPrimary: true,
      order: 0,
    },
  ],
  agent: { id: 'a1', name: 'Agent', email: 'a@example.com' },
  viewCount: 0,
  favoriteCount: 0,
  primaryPhoto: {
    id: 'p1',
    url: 'https://cdn.example.com/p1.jpg',
    thumbnailUrl: 'https://cdn.example.com/p1-thumb.jpg',
    isPrimary: true,
    order: 0,
  },
};

describe('ListingDetailContent', () => {
  beforeEach(() => {
    // Default: not in preview mode
    mockPreviewHook.mockReturnValue({
      previewLayout: null,
      inPreview: false,
      sendSectionClick: vi.fn(),
    });
  });

  it('renders a valid listing', () => {
    render(<ListingDetailContent listing={validListing} />);
    expect(screen.getByText('Beautiful Apartment')).toBeInTheDocument();
  });

  it('renders ListingNotFound when listing is null', () => {
    render(<ListingDetailContent listing={null} />);
    // next-intl is mocked to echo the key.
    expect(screen.getByText('notFound')).toBeInTheDocument();
  });

  it('renders a normalized listing whose features/photos are empty', () => {
    const empty: ListingDetail = { ...validListing, features: {}, photos: [] };
    expect(() => render(<ListingDetailContent listing={empty} />)).not.toThrow();
    expect(screen.getByText('Beautiful Apartment')).toBeInTheDocument();
  });

  it('hides features.v1 and shows placeholder for gallery.v1 when layout says so', () => {
    const layout: ResolvedScreen = {
      screen: 'reality/listing-detail',
      version: 1,
      sections: [
        { type: 'gallery.v1', presentation: 'placeholder' },
        { type: 'listing-header.v1', presentation: 'visible' },
        { type: 'key-details.v1', presentation: 'visible' },
        { type: 'description.v1', presentation: 'visible' },
        // features.v1 omitted entirely
        { type: 'additional-info.v1', presentation: 'visible' },
        { type: 'resources.v1', presentation: 'visible' },
        { type: 'agent-contact.v1', presentation: 'visible' },
      ],
    };
    render(<ListingDetailContent listing={validListing} layout={layout} />);
    // features section is absent
    expect(screen.queryByText('features')).not.toBeInTheDocument();
    // placeholder renders (role="status")
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('escapes user-supplied jsonLd before inlining it into the <script> (XSS)', () => {
    // jsonLd is derived from listing data an attacker can influence. A raw
    // `</script>` (plus U+2028/U+2029) in any string must be escaped to its
    // `\uXXXX` form so it cannot break out of the inline JSON-LD script.
    // The separators are built from code points so this source stays ASCII.
    const LS = String.fromCharCode(0x2028); // U+2028 LINE SEPARATOR
    const PS = String.fromCharCode(0x2029); // U+2029 PARAGRAPH SEPARATOR
    const jsonLd = {
      '@type': 'Product',
      name: '</script><script>alert(1)</script>',
      sep: `${LS}${PS}`,
    };
    const { container } = render(<ListingDetailContent listing={validListing} jsonLd={jsonLd} />);
    const script = container.querySelector('script[type="application/ld+json"]');
    expect(script).not.toBeNull();
    const html = script?.innerHTML ?? '';
    // The break-out sequences must not appear raw...
    expect(html).not.toContain('</script>');
    expect(html).not.toContain(LS);
    expect(html).not.toContain(PS);
    // ...but the escaped forms must, and the JSON must still round-trip.
    expect(html).toContain('\\u003c');
    expect(html).toContain('\\u2028');
    expect(JSON.parse(html)).toEqual(jsonLd);
  });

  it('pushed preview layout overrides the layout prop', () => {
    // The prop has agent-contact visible; the pushed layout makes gallery a placeholder.
    // This proves previewLayout ?? layout prop is used for both mainLayout and sidebar.
    const propLayout: ResolvedScreen = {
      screen: 'reality/listing-detail',
      version: 1,
      sections: [
        { type: 'gallery.v1', presentation: 'visible' },
        { type: 'agent-contact.v1', presentation: 'visible' },
      ],
    };
    const pushedLayout: ResolvedScreen = {
      screen: 'reality/listing-detail',
      version: 1,
      sections: [
        { type: 'gallery.v1', presentation: 'placeholder' },
        // agent-contact omitted → sidebar shows nothing
      ],
    };
    mockPreviewHook.mockReturnValue({
      previewLayout: pushedLayout,
      inPreview: true,
      sendSectionClick: vi.fn(),
    });
    render(<ListingDetailContent listing={validListing} layout={propLayout} />);
    // gallery is now a placeholder — the pushed layout took effect
    expect(screen.getByRole('status')).toBeInTheDocument();
  });
});
