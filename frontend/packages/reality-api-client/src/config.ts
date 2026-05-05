/**
 * Runtime API configuration for @ppt/reality-api-client.
 *
 * Returns the URL at call time so it reads window.__ENV__ (injected by the
 * reality-web server layout) rather than the build-time baked value from
 * process.env.NEXT_PUBLIC_API_URL.
 */
export function getApiBase(): string {
  if (
    typeof window !== 'undefined' &&
    (window as { __ENV__?: { NEXT_PUBLIC_API_URL?: string } }).__ENV__?.NEXT_PUBLIC_API_URL
  ) {
    return (window as { __ENV__?: { NEXT_PUBLIC_API_URL?: string } }).__ENV__!.NEXT_PUBLIC_API_URL!;
  }
  return process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081';
}
