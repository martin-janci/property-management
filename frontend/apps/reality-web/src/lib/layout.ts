/**
 * Reality-web server-side resolved-layout library.
 *
 * Provides types, the hard-coded default layout, and `getResolvedLayout` —
 * a fetch wrapper that NEVER throws and NEVER gates a page render.  Any
 * failure (network error, non-2xx, malformed body) returns the default
 * (spec §4 — fail-safe default).
 *
 * ADAPT (duplication note): `resolveApiBase` / `inferApiBaseFromHost` are
 * replicated verbatim from `src/app/[locale]/listings/[slug]/page.tsx`.
 * That file's helpers are module-private (not exported).  If they are ever
 * exported from a shared location, remove the copies here and import instead.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ResolvedSection {
  type: string;
  mode?: string;
  props?: Record<string, unknown>;
  presentation: 'visible' | 'placeholder';
}

export interface ResolvedScreen {
  screen: string;
  version: number;
  sections: ResolvedSection[];
}

// ---------------------------------------------------------------------------
// Default layout
// ---------------------------------------------------------------------------

const SCREEN = 'reality/listing-detail';

export const DEFAULT_LISTING_DETAIL_LAYOUT: ResolvedScreen = {
  screen: SCREEN,
  version: 0,
  sections: [
    { type: 'gallery.v1', presentation: 'visible' },
    { type: 'listing-header.v1', presentation: 'visible' },
    { type: 'key-details.v1', presentation: 'visible' },
    { type: 'description.v1', presentation: 'visible' },
    { type: 'features.v1', presentation: 'visible' },
    { type: 'additional-info.v1', presentation: 'visible' },
    { type: 'resources.v1', presentation: 'visible' },
    { type: 'agent-contact.v1', presentation: 'visible' },
  ],
};

// ---------------------------------------------------------------------------
// API-base resolution (replicated from listings/[slug]/page.tsx — see ADAPT)
// ---------------------------------------------------------------------------

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

function resolveApiBase(host: string | null): string {
  if (process.env.API_INTERNAL_URL) return process.env.API_INTERNAL_URL;
  if (host) {
    const inferred = inferApiBaseFromHost(host);
    if (inferred) return inferred;
  }
  return process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081';
}

// ---------------------------------------------------------------------------
// getResolvedLayout
// ---------------------------------------------------------------------------

/** Hard cap on the number of sections accepted from the server. */
const MAX_SECTIONS = 100;

/** True when `s` is an object with a string `type` — the minimum shape the
 *  renderer can handle without throwing. */
function isRenderableSection(s: unknown): s is ResolvedSection {
  return s !== null && typeof s === 'object' && typeof (s as { type?: unknown }).type === 'string';
}

/**
 * Fetch the resolved layout for the listing-detail screen.
 *
 * NEVER throws and NEVER gates the page — any failure returns
 * `DEFAULT_LISTING_DETAIL_LAYOUT` (spec §4). Element-level defense: a payload
 * like `sections: [null]` must not be able to SSR-crash the page later, so
 * non-object elements / missing string `type` are dropped and the section
 * list is clamped to MAX_SECTIONS.
 *
 * Cache tags: only the global `layout:listing-detail` tag — the per-host
 * `host:<host>:layout:...` tag family was never revalidated by anything
 * (the /api/layout-revalidate route only knows `layout:<segment>`), so it
 * was dead weight in the tag index.
 */
export async function getResolvedLayout(host: string | null): Promise<ResolvedScreen> {
  try {
    const tags = ['layout:listing-detail'];
    const response = await fetch(
      `${resolveApiBase(host)}/api/v1/layout/resolved/${SCREEN}?platform=web`,
      { headers: host ? { Host: host } : {}, next: { revalidate: 60, tags } }
    );
    if (!response.ok) return DEFAULT_LISTING_DETAIL_LAYOUT;
    const body = (await response.json()) as ResolvedScreen;
    if (!body || body.screen !== SCREEN || !Array.isArray(body.sections)) {
      return DEFAULT_LISTING_DETAIL_LAYOUT;
    }
    return {
      ...body,
      sections: body.sections.filter(isRenderableSection).slice(0, MAX_SECTIONS),
    };
  } catch {
    return DEFAULT_LISTING_DETAIL_LAYOUT;
  }
}
