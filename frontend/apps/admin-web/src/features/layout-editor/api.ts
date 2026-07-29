// Layout Editor API layer
// Consumes: /api/v1/platform-admin/layout/*
// Token is passed as a parameter — pages own the useAdminAuth() call.

import type { ResolvedScreenLike } from '@ppt/shared';

const BASE = '/api/v1/platform-admin/layout';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SectionConfig {
  type: string;
  visible?: boolean;
  mode?: string;
  props?: Record<string, unknown>;
  overrides?: Record<string, unknown>;
}

export interface ScreenConfig {
  screen: string;
  version: number;
  sections: SectionConfig[];
}

export interface Rails {
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

export interface Manifest {
  platform: 'web' | 'mobile';
  components: Record<string, ManifestComponent>;
}

// LayoutConfigRow fields from backend (serialized LayoutConfigRow):
//   screen: String, draft: Value, published: Option<Value>,
//   published_version: i32, rails: Value, updated_at: DateTime<Utc>
export interface ScreenSummary {
  screen: string;
  draft: ScreenConfig;
  published: ScreenConfig | null;
  published_version: number;
  rails: Rails | Record<string, never>;
  updated_at: string;
}

export interface ScreenRow {
  screen: string;
  draft: ScreenConfig;
  published: ScreenConfig | null;
  published_version: number;
  rails: Rails | Record<string, never>;
}

export interface VersionRow {
  version: number;
  published_at: string;
  published_by: string | null;
}

export interface KillRow {
  section_type: string;
  killed_at: string;
}

export interface ConfigEnvelope {
  config: ScreenRow;
  versions: VersionRow[];
  kills: KillRow[];
}

export interface ManifestRow {
  platform: string;
  manifest: Manifest;
  updated_at: string;
}

// ---------------------------------------------------------------------------
// Error class
// ---------------------------------------------------------------------------

export class LayoutApiError extends Error {
  status: number;
  errors: string[];
  constructor(status: number, errors: string[], fallbackMessage?: string) {
    super(errors.join('; ') || fallbackMessage || `HTTP ${status}`);
    this.status = status;
    this.errors = errors;
  }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function headers(token: string | null): Record<string, string> {
  const h: Record<string, string> = { 'Content-Type': 'application/json' };
  if (token) h.Authorization = `Bearer ${token}`;
  return h;
}

async function request<T>(
  token: string | null,
  path: string,
  init: RequestInit = {},
  opts: { emptyOn404?: T } = {}
): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: { ...headers(token), ...(init.headers as Record<string, string> | undefined) },
    credentials: 'include',
  });
  if (res.status === 404 && opts.emptyOn404 !== undefined) return opts.emptyOn404;
  if (!res.ok) {
    // Read the raw text once, then try JSON — keeps the text available as the
    // last-resort message when the body has no `errors: string[]`.
    const rawText = await res.text().catch(() => '');
    let body: unknown = {};
    try {
      body = JSON.parse(rawText);
    } catch {
      // non-JSON body — fall through to rawText
    }
    const record = (typeof body === 'object' && body !== null ? body : {}) as Record<
      string,
      unknown
    >;
    const errors = Array.isArray(record.errors) ? (record.errors as string[]) : [];
    // Fallback message chain: body.message → body.detail → raw text → HTTP <status>
    const fallback =
      (typeof record.message === 'string' && record.message) ||
      (typeof record.detail === 'string' && record.detail) ||
      rawText.trim() ||
      undefined;
    throw new LayoutApiError(res.status, errors, fallback);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

// ---------------------------------------------------------------------------
// Public API functions
// ---------------------------------------------------------------------------

export function listScreens(token: string | null): Promise<ScreenSummary[]> {
  return request(token, '/screens', {}, { emptyOn404: [] as ScreenSummary[] });
}

export function getConfig(token: string | null, screen: string): Promise<ConfigEnvelope> {
  return request(token, `/config?screen=${encodeURIComponent(screen)}`);
}

export function putDraft(token: string | null, screen: string, config: ScreenConfig) {
  return request(token, '/draft', { method: 'PUT', body: JSON.stringify({ screen, config }) });
}

export function putRails(token: string | null, screen: string, rails: Rails) {
  return request(token, '/rails', { method: 'PUT', body: JSON.stringify({ screen, rails }) });
}

export function publish(token: string | null, screen: string) {
  return request(token, '/publish', { method: 'POST', body: JSON.stringify({ screen }) });
}

export function rollback(token: string | null, screen: string, version: number) {
  return request(token, '/rollback', { method: 'POST', body: JSON.stringify({ screen, version }) });
}

export function kill(token: string | null, screen: string, sectionType: string) {
  return request<void>(token, '/kill', {
    method: 'POST',
    body: JSON.stringify({ screen, section_type: sectionType }),
  });
}

export function unkill(token: string | null, screen: string, sectionType: string) {
  return request<void>(token, '/unkill', {
    method: 'POST',
    body: JSON.stringify({ screen, section_type: sectionType }),
  });
}

export function listManifests(token: string | null): Promise<ManifestRow[]> {
  return request(token, '/manifests', {}, { emptyOn404: [] as ManifestRow[] });
}

export function putManifest(token: string | null, platform: string, manifest: Manifest) {
  return request(token, '/manifests', {
    method: 'PUT',
    body: JSON.stringify({ platform, manifest }),
  });
}

// ---------------------------------------------------------------------------
// Preview resolve
// ---------------------------------------------------------------------------

export function previewResolve(
  token: string | null,
  config: ScreenConfig,
  platform: 'web' | 'mobile'
): Promise<ResolvedScreenLike> {
  return request(token, '/preview-resolve', {
    method: 'POST',
    body: JSON.stringify({ config, platform }),
  });
}
