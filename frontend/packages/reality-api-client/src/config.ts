/**
 * Runtime API configuration for @ppt/reality-api-client.
 *
 * Returns the URL at call time so it reads window.__ENV__ (injected by the
 * /env.js route handler in reality-web) rather than the build-time baked
 * value from process.env.NEXT_PUBLIC_API_URL.
 */

function getRuntimeApiUrl(): string | undefined {
  if (typeof window === 'undefined') return undefined;
  const env = (window as { __ENV__?: Record<string, string> }).__ENV__;
  return env?.NEXT_PUBLIC_API_URL;
}

export function getApiBase(): string {
  return getRuntimeApiUrl() ?? process.env.NEXT_PUBLIC_API_URL ?? 'http://localhost:8081';
}
