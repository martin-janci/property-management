/**
 * Listing Detail Page
 *
 * Property detail page with SSR (Epic 44, Story 44.3).
 */

import type { ListingDetail } from '@ppt/reality-api-client';
import type { Metadata } from 'next';
import { ListingDetailContent, ListingNotFound } from '@/components/listings';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081';

interface PageProps {
  params: Promise<{ slug: string }>;
}

async function getListing(slug: string): Promise<ListingDetail | null> {
  try {
    const response = await fetch(`${API_BASE}/api/v1/listings/${slug}`, {
      next: { revalidate: 60 }, // Revalidate every minute
    });
    if (!response.ok) return null;
    return response.json();
  } catch {
    return null;
  }
}

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { slug } = await params;
  const listing = await getListing(slug);

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
  const listing = await getListing(slug);

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
