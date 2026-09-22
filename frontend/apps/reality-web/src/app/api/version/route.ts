import { NextResponse } from 'next/server';
import { versionPayload } from '@/lib/version-payload';

// GET /api/version — alias of `/version` (see src/app/version/route.ts, which
// is the canonical path shared with ppt-web and admin-web). Kept so the route
// that shipped first keeps answering; the body is identical.
export const dynamic = 'force-dynamic';

export async function GET() {
  return NextResponse.json(versionPayload());
}
