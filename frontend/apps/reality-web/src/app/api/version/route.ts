import { NextResponse } from 'next/server';

// Version identity (T6). reality-web is the one frontend image without nginx in
// front of it, so its /version surface is this route handler; ppt-web and
// admin-web serve the identical JSON from their nginx templates.
//
// The values are baked into the image by docker/frontend/reality-web.Dockerfile
// (ARG APP_VERSION / GIT_SHA / BUILD_DATE -> ENV), fed from the release tag and
// github.sha — no git call and no I/O at request time.
//
// force-dynamic so Next does not prerender the body at build time into a static
// response that would outlive an env change.
export const dynamic = 'force-dynamic';

export async function GET() {
  return NextResponse.json({
    version: process.env.APP_VERSION ?? 'unknown',
    commit: process.env.GIT_SHA ?? 'unknown',
    built_at: process.env.BUILD_DATE ?? 'unknown',
  });
}
