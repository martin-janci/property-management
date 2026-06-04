/**
 * Universal Links (iOS) / App Links (Android) configuration.
 *
 * gap-85-3: Configure HTTPS deep links so that tapping an
 * `https://<app-host>/<path>` URL on a device with the app installed opens
 * the corresponding in-app screen instead of the browser.
 *
 * The host(s) below MUST match:
 *   - iOS:     `ios.associatedDomains` in app.config.ts (`applinks:<host>`)
 *              and the `apple-app-site-association` file served at
 *              `https://<host>/.well-known/apple-app-site-association`.
 *   - Android: the `https` `intentFilters` host in app.config.ts and the
 *              `assetlinks.json` served at
 *              `https://<host>/.well-known/assetlinks.json`.
 *
 * Single source of truth for the host comes from the `APP_LINK_HOST` env var
 * (loaded by app.config.ts into `Constants.expoConfig.extra.appLinkHost`),
 * falling back to the convention placeholder domain used across the env files.
 */
import Constants from 'expo-constants';

/**
 * Convention placeholder host. Real hosts are injected per-environment via the
 * `APP_LINK_HOST` env var (see .env.<env> and app.config.ts).
 */
export const DEFAULT_APP_LINK_HOST = 'app.ppt.example.com';

/**
 * The HTTPS host this build associates universal links / App Links with.
 * Read from Expo config (single source of truth: app.config.ts → extra).
 */
export function getAppLinkHost(): string {
  const fromExtra = Constants.expoConfig?.extra?.appLinkHost as string | undefined;
  return fromExtra && fromExtra.length > 0 ? fromExtra : DEFAULT_APP_LINK_HOST;
}

/**
 * Returns the set of hosts an incoming universal link may legitimately use.
 * Includes the configured host. Kept as an array so additional hosts (e.g. a
 * `www.` alias or a short-link domain) can be appended without touching the
 * parsing logic.
 */
export function getAppLinkHosts(): string[] {
  return [getAppLinkHost()];
}

/**
 * Custom URL scheme for non-HTTPS deep links (QR codes, push payloads, etc).
 * Matches `scheme` in app.config.ts and the legacy `ppt://` links.
 */
export const APP_SCHEMES = ['ppt', 'ppt-management'] as const;

/**
 * Normalise any supported deep-link URL (custom scheme OR universal link) to
 * the canonical `<path>?<query>` form the route matcher understands.
 *
 * - `ppt://faults/123?x=1`                        -> `faults/123?x=1`
 * - `ppt-management://faults/123`                 -> `faults/123`
 * - `https://app.ppt.example.com/faults/123?x=1`  -> `faults/123?x=1`
 *
 * Returns `null` when the URL is not a deep link this app owns (wrong scheme,
 * or an HTTPS URL whose host is not an associated App-Link host).
 */
export function normalizeLinkUrl(url: string): string | null {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return null;
  }

  const scheme = parsed.protocol.replace(/:$/, '');

  if ((APP_SCHEMES as readonly string[]).includes(scheme)) {
    // Custom-scheme URLs put the first path segment in the host slot
    // (e.g. `ppt://faults/123` → host=`faults`, pathname=`/123`).
    const host = parsed.host ?? '';
    const rest = parsed.pathname.replace(/^\//, '');
    const path = [host, rest].filter(Boolean).join('/');
    return appendQuery(path, parsed.search);
  }

  if (scheme === 'https' && getAppLinkHosts().includes(parsed.host)) {
    const path = parsed.pathname.replace(/^\//, '');
    return appendQuery(path, parsed.search);
  }

  return null;
}

function appendQuery(path: string, search: string): string {
  return search && search.length > 1 ? `${path}${search}` : path;
}
