/**
 * Layout API Client
 *
 * Fetches resolved screen layouts from the layout endpoint.
 * The layout endpoint is additive, never gating — callers must fall back
 * to their own DEFAULT_LAYOUT when this throws.
 */

import { getOrg, getToken } from '../auth';

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

const API_BASE = '/api/v1/layout';

/** Hard cap on the number of sections accepted from the server. */
const MAX_SECTIONS = 100;

/** True when `s` is an object with a string `type` — the minimum shape the
 *  renderer can handle without throwing. */
function isRenderableSection(s: unknown): s is ResolvedSection {
  return s !== null && typeof s === 'object' && typeof (s as { type?: unknown }).type === 'string';
}

/** Fetch the resolved layout for a screen. Throws on any failure — callers
 *  are expected to fall back to their DEFAULT_LAYOUT (spec §4: the layout
 *  endpoint is additive, never gating).
 *
 *  Uses `tenantRequest` (not `authenticatedFetchJson`) so the request carries
 *  X-Tenant-ID — the backend TenantExtractor 400s without it on non-pinned
 *  hosts. Element-level defense: non-object elements / missing string `type`
 *  are dropped, and the section list is clamped to MAX_SECTIONS. */
export async function fetchResolvedLayout(
  screen: string,
  platform: 'web' | 'mobile' = 'web'
): Promise<ResolvedScreen> {
  const data = await tenantRequest<ResolvedScreen>(
    `${API_BASE}/resolved/${screen}?platform=${platform}`
  );
  if (!data || !Array.isArray(data.sections)) {
    throw new Error('layout: malformed ResolvedScreen payload');
  }
  return {
    ...data,
    sections: data.sections.filter(isRenderableSection).slice(0, MAX_SECTIONS),
  };
}

// ---------------------------------------------------------------------------
// Tenant-override domain
// ---------------------------------------------------------------------------

export interface TenantSectionPatch {
  visible?: boolean;
  mode?: string;
  props?: Record<string, unknown>;
}

export interface TenantOverride {
  order?: string[];
  sections?: Record<string, TenantSectionPatch>;
}

export interface LayoutRails {
  hideable: string[];
  mode_editable: string[];
  reorderable: boolean;
  prop_whitelist: Record<string, string[]>;
}

export interface ManifestComponent {
  required?: boolean;
  supported_modes?: string[];
  default_mode?: string;
}

export interface LayoutManifest {
  platform: string;
  components: Record<string, ManifestComponent>;
}

export interface BaseSection {
  type: string;
  visible?: boolean;
  mode?: string;
  props?: Record<string, unknown>;
}

export interface TenantLayoutEnvelope {
  override: { override_config: TenantOverride } | null;
  rails: LayoutRails | Record<string, never>;
  published: { sections: BaseSection[] } | null;
  manifest: LayoutManifest | null;
}

export class TenantLayoutError extends Error {
  status: number;
  errors: string[];

  constructor(status: number, errors: string[]) {
    super(`TenantLayoutError: HTTP ${status}`);
    this.name = 'TenantLayoutError';
    this.status = status;
    this.errors = errors;
  }
}

/**
 * Raw fetch helper for tenant-override endpoints.
 * Adds Authorization + X-Tenant-ID headers; parses {errors} on non-2xx
 * into TenantLayoutError. Must NOT use authenticatedFetchJson — it lacks
 * the org header and swallows error bodies.
 */
async function tenantRequest<T>(url: string, options?: RequestInit): Promise<T> {
  const token = getToken();
  const org = getOrg();

  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(org ? { 'X-Tenant-ID': org } : {}),
      ...options?.headers,
    },
  });

  if (!response.ok) {
    let errors: string[] = [];
    try {
      const body = await response.json();
      if (Array.isArray(body?.errors)) {
        errors = body.errors as string[];
      }
    } catch {
      // body unparseable — leave errors as empty array
    }
    throw new TenantLayoutError(response.status, errors);
  }

  return response.json() as Promise<T>;
}

/** Fetch the tenant layout envelope for a screen (GET). */
export async function fetchTenantLayout(screen: string): Promise<TenantLayoutEnvelope> {
  const params = new URLSearchParams({ screen });
  return tenantRequest<TenantLayoutEnvelope>(`${API_BASE}/tenant-override?${params}`);
}

/** Save (PUT) a tenant layout override for a screen. Throws TenantLayoutError on non-2xx. */
export async function saveTenantLayoutOverride(
  screen: string,
  override: TenantOverride
): Promise<unknown> {
  return tenantRequest<unknown>(`${API_BASE}/tenant-override`, {
    method: 'PUT',
    body: JSON.stringify({ screen, override_config: override }),
  });
}
