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
import { buildListingMetadata } from './metadata';

function inferApiBaseFromHost(host: string): string | null {
  const bareHost = host.split(':')[0]?.toLowerCase();
  if (!bareHost) return null;
  if (bareHost === 'rlt.sk' || bareHost.endsWith('.rlt.sk')) {
    if (bareHost === 'staging.rlt.sk' || bareHost.endsWith('.staging.rlt.sk')) {
      return 'https://api.staging.rlt.sk';
    }
    return 'https://api.rlt.sk';
  }
  return null;
}

function resolveApiBase(host: string): string {
  // SSR path: prefer an internal URL (Docker network) when set, then host
  // inference for known *.rlt.sk topology, then the public env, then a
  // dev-only localhost fallback. Inference comes before NEXT_PUBLIC_API_URL
  // so a baked-in/misconfigured `http://localhost:8081` can't take down
  // SSR on staging or prod.
  if (process.env.API_INTERNAL_URL) return process.env.API_INTERNAL_URL;
  const inferred = inferApiBaseFromHost(host);
  if (inferred) return inferred;
  return process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081';
}

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
    const apiBase = resolveApiBase(host);
    const response = await fetch(`${apiBase}/api/v1/listings/${slug}`, {
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

  return buildListingMetadata(listing);
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
