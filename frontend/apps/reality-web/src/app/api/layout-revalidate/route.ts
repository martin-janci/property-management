import { createHmac, timingSafeEqual } from 'node:crypto';
import { revalidateTag } from 'next/cache';
import { NextResponse } from 'next/server';

// ---------------------------------------------------------------------------
// Exported helper — maps a screen id to its cache tags.
// Only screens that contain a slash are valid (e.g. "reality/listing-detail").
// ---------------------------------------------------------------------------

export function layoutTagsFor(screen: string): string[] {
  return ['layout:' + screen.split('/')[1]];
}

// ---------------------------------------------------------------------------
// POST /api/layout-revalidate
// ---------------------------------------------------------------------------

export async function POST(request: Request): Promise<NextResponse> {
  // 1. Secret must be configured
  const secret = process.env.LAYOUT_WEBHOOK_SECRET;
  if (!secret) {
    return NextResponse.json({ error: 'disabled' }, { status: 503 });
  }

  // 2. Read raw body FIRST — signature is over the raw bytes
  const rawBody = await request.text();

  // 3. Verify HMAC signature
  const header = request.headers.get('X-Webhook-Signature') ?? '';
  const expected = 'sha256=' + createHmac('sha256', secret).update(rawBody).digest('base64');

  const headerBuf = Buffer.from(header);
  const expectedBuf = Buffer.from(expected);

  // Guard unequal lengths before timingSafeEqual (it throws on length mismatch)
  if (headerBuf.length !== expectedBuf.length || !timingSafeEqual(headerBuf, expectedBuf)) {
    return NextResponse.json({ error: 'invalid signature' }, { status: 401 });
  }

  // 4. Parse body and validate shape
  let body: unknown;
  try {
    body = JSON.parse(rawBody);
  } catch {
    return NextResponse.json({ error: 'invalid body' }, { status: 422 });
  }

  if (
    typeof body !== 'object' ||
    body === null ||
    typeof (body as Record<string, unknown>).screen !== 'string'
  ) {
    return NextResponse.json({ error: 'invalid body' }, { status: 422 });
  }

  const screen = (body as { screen: string }).screen;

  // 5. Validate screen contains a slash
  if (!screen.includes('/')) {
    return NextResponse.json({ error: 'invalid screen' }, { status: 422 });
  }

  // 6. Revalidate tags
  const tags = layoutTagsFor(screen);
  for (const tag of tags) {
    revalidateTag(tag, 'default');
  }

  return NextResponse.json({ revalidated: true, tags });
}
