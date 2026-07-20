/**
 * Layout API Client
 *
 * Fetches resolved screen layouts from the layout endpoint.
 * The layout endpoint is additive, never gating — callers must fall back
 * to their own DEFAULT_LAYOUT when this throws.
 */

import { authenticatedFetchJson } from '../lib/fetch';

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

/** Fetch the resolved layout for a screen. Throws on any failure — callers
 *  are expected to fall back to their DEFAULT_LAYOUT (spec §4: the layout
 *  endpoint is additive, never gating). */
export async function fetchResolvedLayout(
  screen: string,
  platform: 'web' | 'mobile' = 'web'
): Promise<ResolvedScreen> {
  const data = await authenticatedFetchJson<ResolvedScreen>(
    `${API_BASE}/resolved/${screen}?platform=${platform}`
  );
  if (!data || !Array.isArray(data.sections)) {
    throw new Error('layout: malformed ResolvedScreen payload');
  }
  return data;
}
