import { type NextRequest, NextResponse } from 'next/server';

// Always dynamic — this route's whole purpose is to expose runtime env vars.
export const dynamic = 'force-dynamic';

/**
 * Serves runtime environment configuration as a JavaScript snippet.
 *
 * Only keys that are actually set in process.env are included, so this
 * script never shadows build-time baked values with localhost fallbacks.
 * Client code reads window.__ENV__ via src/lib/env.ts with a graceful
 * fallback to process.env (build-time) and then to localhost defaults.
 *
 * Load with: <script src="/env.js" /> (synchronous, no async/defer)
 * Cache-Control is no-store so each page load gets the current values.
 */
export function GET(_req: NextRequest) {
  const env: Record<string, string> = {};

  if (process.env.NEXT_PUBLIC_API_URL) {
    env.NEXT_PUBLIC_API_URL = process.env.NEXT_PUBLIC_API_URL;
  }
  if (process.env.NEXT_PUBLIC_SITE_URL) {
    env.NEXT_PUBLIC_SITE_URL = process.env.NEXT_PUBLIC_SITE_URL;
  }

  // Escape '<' to prevent '</script>' injection in case a value is embedded
  // inside another script context (defence-in-depth; content is env vars).
  const safeJson = JSON.stringify(env).replace(/</g, '\\u003c');

  return new NextResponse(`window.__ENV__=${safeJson};`, {
    headers: {
      'Content-Type': 'application/javascript; charset=utf-8',
      'Cache-Control': 'no-store',
    },
  });
}
