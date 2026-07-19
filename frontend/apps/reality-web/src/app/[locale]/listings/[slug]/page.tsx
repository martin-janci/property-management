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
import { notFound } from 'next/navigation';
import { ListingDetailContent } from '@/components/listings';
import { buildListingJsonLd } from './jsonLd';
import { parseListingDetail } from './listingSchema';
import { buildListingMetadata } from './metadata';
import { getResolvedLayout } from '@/lib/layout';

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
    // A 200 does not guarantee a well-formed body: an upstream/proxy partial
    // response or a malformed listing is truthy but may be missing required
    // nested fields (`address.city`, …) or carry wrong-typed collection fields
    // (`features: "x"`, `photos: {}`). Rendering such a body crashes SSR with a
    // 500. `parseListingDetail` is the single normalizer that validates the
    // required shape once and coerces `features`/`photos` — a bad body falls
    // back to `ListingNotFound` (404-style), and a good one is guaranteed-shape
    // for every consumer downstream (metadata, JSON-LD, ListingDetailContent).
    const body: unknown = await response.json();
    return parseListingDetail(body);
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
  const [listing, layout] = await Promise.all([getListing(slug, host), getResolvedLayout(host)]);

  // A missing / malformed listing must emit a real HTTP 404, not a 200 with
  // "not found" markup: this is a public, SEO-indexed portal, and a soft-404
  // (200 body) lets crawlers index dead listings. `notFound()` renders the
  // locale-aware `app/[locale]/not-found.tsx` with a 404 status. `notFound()`
  // returns `never`, so `listing` is non-null below (#2341).
  if (!listing) {
    notFound();
  }

  // JSON-LD structured data. `listing` is a guaranteed-shape `ListingDetail`
  // (normalized by `parseListingDetail` in `getListing`), so `buildListingJsonLd`
  // can dereference required fields directly.
  const jsonLd = buildListingJsonLd(listing);

  return <ListingDetailContent listing={listing} jsonLd={jsonLd} layout={layout} />;
}
