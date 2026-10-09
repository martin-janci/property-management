import { NextResponse } from 'next/server';
import { versionPayload } from '@/lib/version-payload';

// GET /version — the shared runtime version surface (T6).
//
// This is the path the acceptance criterion uses (`curl https://<host>/version`)
// and the one ppt-web and admin-web serve from nginx, so reality-web must answer
// on it too: Caddy routes the whole reality apex to this container, so without
// this route the reality apex was the one service of the five where /version
// 404'd and only /api/version worked.
//
// `/api/version` is kept as an alias so anything already pointing at it keeps
// working; both handlers return the identical body from `versionPayload()`.
//
// NOTE: `/version` must also be excluded from the next-intl middleware matcher
// (src/middleware.ts) — otherwise the locale middleware rewrites/redirects it to
// `/<locale>/version`, where no route exists. `/api/**` is excluded there
// already, which is why the alias never needed this.
//
// force-dynamic so Next does not prerender the body at build time into a static
// response that would outlive an env change.
export const dynamic = 'force-dynamic';

export async function GET() {
  return NextResponse.json(versionPayload());
}
