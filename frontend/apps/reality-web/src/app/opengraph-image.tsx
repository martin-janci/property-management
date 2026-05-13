import { ImageResponse } from 'next/og';

/**
 * Default Open Graph card (1200×630) for the entire site.
 *
 * Next 14 file convention — Next renders this on demand at /opengraph-image
 * and serves it with proper cache headers. Per-route OG cards (e.g. listing
 * detail with the property photo) can override by dropping a sibling
 * opengraph-image.tsx in their route directory.
 *
 * Layout: brand mark + "Reality Portal" wordmark + a single English
 * tagline. The tagline is intentionally kept in English so we don't have
 * to maintain six locale-specific PNGs — share previews for non-English
 * users still get the locale-correct OG `description` below the image
 * from the per-route metadata. (When traffic from a specific non-English
 * locale justifies it, drop a sibling `[locale]/opengraph-image.tsx`
 * with localized copy.)
 */
export const runtime = 'edge';
export const size = { width: 1200, height: 630 };
export const contentType = 'image/png';

export default async function OgImage() {
  return new ImageResponse(
    <div
      style={{
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'linear-gradient(135deg, #1f4cd1 0%, #3b82f6 100%)',
        color: '#fff',
        fontFamily: 'sans-serif',
      }}
    >
      <div
        style={{
          width: 96,
          height: 96,
          borderRadius: 24,
          background: 'rgba(255,255,255,0.15)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          marginBottom: 36,
        }}
      >
        <svg viewBox="0 0 32 32" width="56" height="56">
          <title>Reality Portal</title>
          <path d="M8 22V12l8-6 8 6v10h-5v-6h-6v6z" fill="#fff" />
        </svg>
      </div>
      <div style={{ fontSize: 84, fontWeight: 800, letterSpacing: -1 }}>Reality Portal</div>
      <div style={{ fontSize: 32, opacity: 0.8, marginTop: 18 }}>Find your perfect property</div>
    </div>,
    size
  );
}
