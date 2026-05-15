/**
 * Server-side tenant config fetcher (Phase 3: Hosting & Theming).
 *
 * Reality-web runs the same Next.js deployment for the global portal AND every
 * tenant subdomain / custom domain. This module is the single point that asks
 * the api-server "given the host this request landed on, what is the tenant
 * config?" — and caches the answer per-request so the layout, page, and any
 * server components share one fetch.
 *
 * # Why server-side only
 *
 * The browser cannot reliably send the original `Host` it landed on (a SPA
 * fetch from `acme.example.com` would still hit `api.example.com`). The
 * server-side request, by contrast, IS that host. We forward it explicitly
 * so api-server's host_tenant_middleware sees the same value Caddy did.
 *
 * # Caching
 *
 * - Per-request memoization via React's `cache()` — multiple components in
 *   one render share one fetch.
 * - Next.js `revalidate: 60` so ISR pages get a fresh config every minute
 *   without hammering api-server. Tenant deletes / brand flips need at
 *   most ~60s to propagate.
 * - The `tags: [`tenant:${host}`]` cache key tag lets the admin panel call
 *   `revalidateTag('tenant:acme.example.com')` to force-refresh after a
 *   branding change.
 */

import type React from 'react';
import { cache } from 'react';
import { headers } from 'next/headers';

export type FeatureFlagState = {
  enabled: boolean;
  value?: unknown;
};

export type TenantBranding = {
  logo_url: string | null;
  primary_color: string | null;
  secondary_color: string | null;
  accent_color: string | null;
  font_family: string | null;
  /** Open-ended `--ppt-*` token overrides. */
  css_vars: Record<string, string>;
};

export type TenantConfig = {
  /** `null` for the platform host (the global Reality Portal). */
  tenant_id: string | null;
  name: string;
  branding: TenantBranding;
  feature_flags: Record<string, FeatureFlagState>;
  locales: string[];
};

/**
 * Default config used when api-server is unreachable or returns malformed
 * data. We never want to throw out of the layout — a degraded experience
 * (no custom branding) beats a 500 page.
 */
const PLATFORM_DEFAULT: TenantConfig = {
  tenant_id: null,
  name: 'Reality Portal',
  branding: {
    logo_url: null,
    primary_color: null,
    secondary_color: null,
    accent_color: null,
    font_family: null,
    css_vars: {},
  },
  feature_flags: {},
  locales: ['en', 'sk', 'cs', 'de', 'pl', 'hu'],
};

/**
 * Resolve the api-server URL the SSR layer should call.
 *
 * Priority:
 * 1. `API_INTERNAL_URL` — explicit. In docker-compose this is the
 *    cluster-internal name, e.g. `http://api-server:8080`. This avoids
 *    the SSR layer making a public-internet round-trip back to itself.
 * 2. `NEXT_PUBLIC_API_URL` — fall back if the operator only set the
 *    public URL. Works at the cost of a public hop.
 * 3. `http://localhost:8080` — local dev default.
 */
function getApiInternalUrl(): string {
  return (
    process.env.API_INTERNAL_URL ||
    process.env.NEXT_PUBLIC_API_URL ||
    'http://localhost:8080'
  ).replace(/\/+$/, '');
}

/**
 * Fetch the tenant config for the inbound request's host.
 *
 * Memoized per-request via React's `cache()`. The `Host` header is
 * forwarded verbatim so api-server resolves the same tenant Caddy did.
 *
 * Errors (network / non-2xx / malformed JSON) all degrade to the platform
 * default — never throw out of the SSR layer.
 */
export const getTenantConfig = cache(async (): Promise<TenantConfig> => {
  let host: string | null = null;
  try {
    const h = await headers();
    host = h.get('x-forwarded-host') || h.get('host');
  } catch {
    // `headers()` throws when called outside a request scope (during
    // generateStaticParams etc). Falling back to platform default is
    // safe in that branch — those pages render the global portal shell.
  }

  const apiUrl = `${getApiInternalUrl()}/tenant-config`;

  try {
    const response = await fetch(apiUrl, {
      method: 'GET',
      headers: host ? { Host: host } : {},
      // Include host in the Next data-cache key so two tenants don't
      // collide. Tag is also exposed for selective `revalidateTag`.
      next: {
        revalidate: 60,
        tags: host ? [`tenant:${host}`] : ['tenant:platform'],
      },
    });

    if (!response.ok) {
      // Non-2xx — degrade gracefully. The api-server only 4xx/5xxes
      // /tenant-config under genuine failure (the soft-resolve middleware
      // path always returns 200 for the platform host).
      return PLATFORM_DEFAULT;
    }

    const json = (await response.json()) as Partial<TenantConfig>;

    return {
      tenant_id: json.tenant_id ?? null,
      name: json.name ?? PLATFORM_DEFAULT.name,
      branding: {
        logo_url: json.branding?.logo_url ?? null,
        primary_color: json.branding?.primary_color ?? null,
        secondary_color: json.branding?.secondary_color ?? null,
        accent_color: json.branding?.accent_color ?? null,
        font_family: json.branding?.font_family ?? null,
        css_vars: (json.branding?.css_vars as Record<string, string>) ?? {},
      },
      feature_flags: (json.feature_flags as Record<string, FeatureFlagState>) ?? {},
      locales: json.locales ?? PLATFORM_DEFAULT.locales,
    };
  } catch {
    return PLATFORM_DEFAULT;
  }
});

/**
 * Convenience: server-side feature flag check.
 *
 * Reads `building_disabled` (and any other per-tenant kill switch) from
 * the resolved tenant config. A flag with no row counts as `enabled: false`.
 */
export async function getFeatureFlag(key: string): Promise<FeatureFlagState> {
  const cfg = await getTenantConfig();
  return cfg.feature_flags[key] ?? { enabled: false };
}

/**
 * Build a React style object from branding. CSS custom properties (`--ppt-*`)
 * are first-class style keys in React, so we just collect them into an object.
 * Sanitization rejects keys / values that could break out of the style
 * attribute (a tenant-controlled CSS var must not become an injection vector).
 *
 * Returns `undefined` when there is nothing to set, so callers can safely
 * spread without emitting `style=""` on every page render.
 */
export function brandingToStyleObject(
  branding: TenantBranding,
): React.CSSProperties | undefined {
  const style: Record<string, string> = {};
  if (branding.font_family) {
    const safeFont = sanitizeCssValue(branding.font_family, '--ppt-font-family');
    if (safeFont) style['--ppt-font-family'] = safeFont;
  }
  if (branding.primary_color) {
    const safe = sanitizeCssValue(branding.primary_color, '--ppt-color-primary');
    if (safe) style['--ppt-color-primary'] = safe;
  }
  if (branding.secondary_color) {
    const safe = sanitizeCssValue(branding.secondary_color, '--ppt-color-secondary');
    if (safe) style['--ppt-color-secondary'] = safe;
  }
  if (branding.accent_color) {
    const safe = sanitizeCssValue(branding.accent_color, '--ppt-color-accent');
    if (safe) style['--ppt-color-accent'] = safe;
  }
  for (const [k, v] of Object.entries(branding.css_vars)) {
    if (!isSafeCssVarName(k)) continue;
    const safe = sanitizeCssValue(String(v), k);
    if (safe) style[k] = safe;
  }
  if (Object.keys(style).length === 0) return undefined;
  return style as React.CSSProperties;
}

/**
 * Legacy raw-string version. Kept for tests / non-React consumers.
 * Sanitization is identical to `brandingToStyleObject`.
 */
export function brandingToStyleString(branding: TenantBranding): string {
  const obj = brandingToStyleObject(branding);
  if (!obj) return '';
  return Object.entries(obj as Record<string, string>)
    .map(([k, v]) => `${k}: ${v}`)
    .join('; ');
}

export function isSafeCssVarName(name: string): boolean {
  return /^--[a-z0-9-]+$/.test(name) && name.length <= 64;
}

/**
 * Sanitize a CSS custom-property value before it lands in inline `style=`.
 *
 * Defense layers (N7):
 * - Reject characters that break out of the style attribute or chain a
 *   second declaration: `<`, `>`, `"`, newlines, `{`, `}`, `;`.
 * - Length cap (200 chars) — bigger than any legit color / font value.
 * - Denylist tokens that resolve to a network fetch or arbitrary script
 *   when the var ends up in a property that evaluates URLs (background-image,
 *   list-style-image, mask, cursor, etc.): `url(`, `expression(`,
 *   `javascript:`, `data:`. Logo URLs flow through `branding.logo_url`
 *   (a typed string the layout renders into a real `<link rel="icon">`),
 *   never through `css_vars`.
 * - Positive allowlist for color-shaped vars: when the var name suggests
 *   a color (matches `/color|background|border/`), the value MUST look
 *   like a CSS color literal (hex, rgb/rgba, hsl/hsla, or a bare CSS
 *   color keyword). Anything else (e.g. `var(--evil)`, `inherit`,
 *   `attr(...)`) is rejected.
 *
 * Returns `null` when the value cannot be made safe — callers drop the
 * declaration rather than emit a partial / suspicious value.
 */
export function sanitizeCssValue(value: string, varName?: string): string | null {
  if (typeof value !== 'string') return null;
  if (value.length > 200) return null;
  // Structural breakouts.
  if (/[<>"\n\r{};]/.test(value)) return null;

  // Token denylist (case-insensitive). `data:` is denied wholesale even
  // though `data:image/...` is theoretically harmless — branding rows do
  // not need it (logos go through `logo_url`), and allow-listing the
  // `image/png` subset would reopen the door to `data:text/html` via
  // case / whitespace tricks.
  const lower = value.toLowerCase().replace(/\s+/g, '');
  if (
    lower.includes('url(') ||
    lower.includes('expression(') ||
    lower.includes('javascript:') ||
    lower.includes('data:')
  ) {
    return null;
  }

  // Color-shaped var: enforce a CSS color literal. The allowed shapes are
  // intentionally narrow — no `var(...)` indirection (would let a tenant
  // chain to an unsanitized var), no `currentColor` / `inherit` (low
  // value, easy to bypass intent), no functional notations beyond rgb/hsl
  // families.
  if (varName && /color|background|border/i.test(varName)) {
    const trimmed = value.trim();
    const isColorLiteral =
      /^#[0-9a-f]{3,8}$/i.test(trimmed) ||
      /^rgba?\([^()]*\)$/i.test(trimmed) ||
      /^hsla?\([^()]*\)$/i.test(trimmed) ||
      /^[a-z]+$/i.test(trimmed); // bare keyword: red, transparent, etc.
    if (!isColorLiteral) return null;
  }

  return value;
}
