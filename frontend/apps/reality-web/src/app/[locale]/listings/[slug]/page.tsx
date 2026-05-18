/**
 * Listing Detail Page
 *
 * Property detail page with SSR (Epic 44, Story 44.3).
 *
 * C2 (Phase 6): fetch cache is tenant-keyed via `tags` so a
 * `revalidateTag('host:<host>:listing:<slug>')` call only busts the cache
 * for that tenant — not for other tenants serving the same slug.
 */

import type { ListingDetail } from '@ppt/reality-api-client';
import type { Metadata } from 'next';
import { headers } from 'next/headers';
import { ListingDetailContent, ListingNotFound } from '@/components/listings';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081';

interface PageProps {
  params: Promise<{ slug: string }>;
}

/**
 * Resolve the inbound host from the `x-tenant-host` header set by middleware.
 * Falls back to the `host` header, then to the empty string (build time).
 */
async function resolveHost(): Promise<string> {
  try {
    const h = await headers();
    return h.get('x-tenant-host') || h.get('host') || '';
  } catch {
    // Outside a request scope (generateStaticParams / build time).
    return '';
  }
}

async function getListing(slug: string, host: string): Promise<ListingDetail | null> {
  try {
    // C2: include host in cache tags so per-tenant ISR invalidation works:
    //   revalidateTag('host:agency.example.com:listing:my-flat-slug')
    // busts only that tenant's cached entry.  The `host` part of the
    // cache key is implicit in the fetch URL (Host header) but Next.js
    // data-cache does NOT automatically key on request headers — explicit
    // `tags` are required.
    const tags = host
      ? [`host:${host}:listings`, `host:${host}:listing:${slug}`]
      : ['listings', `listing:${slug}`];
    const response = await fetch(`${API_BASE}/api/v1/listings/${slug}`, {
      headers: host ? { Host: host } : {},
      next: { revalidate: 60, tags },
    });
    if (!response.ok) return null;
    return response.json();
  } catch {
    return null;
  }
}

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { slug } = await params;
  const host = await resolveHost();
  const listing = await getListing(slug, host);

  if (!listing) {
    return {
      title: 'Listing Not Found - Reality Portal',
    };
  }

  const title = `${listing.title} - ${listing.address.city} | Reality Portal`;
  const description = listing.description.slice(0, 160);

  return {
    title,
    description,
    openGraph: {
      title,
      description,
      type: 'website',
      images: listing.primaryPhoto ? [listing.primaryPhoto.url] : [],
    },
  };
}

export default async function ListingDetailPage({ params }: PageProps) {
  const { slug } = await params;
  const host = await resolveHost();
  const listing = await getListing(slug, host);

  if (!listing) {
    return <ListingNotFound />;
  }

  // JSON-LD structured data
  const jsonLd = {
    '@context': 'https://schema.org',
    '@type': 'RealEstateListing',
    name: listing.title,
    description: listing.description,
    url: `${process.env.NEXT_PUBLIC_SITE_URL || ''}/listings/${listing.slug}`,
    image: listing.photos.map((p) => p.url),
    address: {
      '@type': 'PostalAddress',
      streetAddress: listing.address.street,
      addressLocality: listing.address.city,
      addressRegion: listing.address.district,
      postalCode: listing.address.postalCode,
      addressCountry: listing.address.country,
    },
    offers: {
      '@type': 'Offer',
      price: listing.price,
      priceCurrency: listing.currency,
      availability: listing.status === 'active' ? 'InStock' : 'OutOfStock',
    },
    numberOfRooms: listing.rooms,
    floorSize: {
      '@type': 'QuantitativeValue',
      value: listing.area,
      unitCode: 'MTK',
    },
  };

  return <ListingDetailContent listing={listing} jsonLd={jsonLd} />;
}
