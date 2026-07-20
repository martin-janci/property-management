import { createHmac, timingSafeEqual } from 'node:crypto';
import { revalidateTag } from 'next/cache';
import { NextResponse } from 'next/server';

// ---------------------------------------------------------------------------
// Exported helper — maps a screen id to its cache tags.
// Only screens that contain a slash are valid (e.g. "reality/listing-detail").
// ---------------------------------------------------------------------------

export function layoutTagsFor(screen: string): string[] | null {
  const parts = screen.split('/');
  const segment = parts[1];
  // Require non-empty second segment (e.g., reject 'foo/')
  if (!segment) return null;
  return ['layout:' + segment];
}

// ---------------------------------------------------------------------------
// Replay protection (issue #2485).
//
// The delivery must carry an `X-Webhook-Timestamp` (unix seconds), and the
// signature is computed over the timestamped payload `"{timestamp}.{body}"`
// (matching api-server `routes/layout/webhook.rs::sign_timestamped_payload`
// and the portal-webhook convention). A captured POST is only valid within
// TOLERANCE_SECS of the receiver's clock, so it cannot be replayed
// indefinitely, and because the timestamp is folded into the HMAC it cannot be
// swapped for a fresh one without invalidating the signature.
// ---------------------------------------------------------------------------

/** Max accepted clock skew (seconds) between the signed timestamp and now. */
const TOLERANCE_SECS = 300;

/** Maximum accepted body size, in bytes (16 KiB). */
const MAX_BODY_BYTES = 16 * 1024;

/** Parse a strict base-10 integer unix-seconds timestamp, or null if malformed. */
export function parseWebhookTimestamp(raw: string | null): number | null {
  if (raw === null) return null;
  const trimmed = raw.trim();
  // Reject empty, non-integer, or anything Number.parseInt would silently
  // truncate (e.g. "123abc", "1.5", "0x10").
  if (!/^-?\d+$/.test(trimmed)) return null;
  const ts = Number.parseInt(trimmed, 10);
  return Number.isSafeInteger(ts) ? ts : null;
}

/** Whether `timestamp` is within `tolerance` seconds of `nowSecs` (inclusive). */
export function isTimestampFresh(
  timestamp: number,
  nowSecs: number,
  tolerance: number = TOLERANCE_SECS
): boolean {
  return Math.abs(nowSecs - timestamp) <= tolerance;
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

  // 2. Read raw body FIRST — signature is over the raw bytes. Cap the size
  // BEFORE any verification work so an oversized body cannot force an
  // unbounded HMAC/JSON pass on an unauthenticated request.
  const rawBody = await request.text();
  if (Buffer.byteLength(rawBody, 'utf8') > MAX_BODY_BYTES) {
    return NextResponse.json({ error: 'body too large' }, { status: 413 });
  }

  // 3. Replay protection (issue #2485): require a fresh signed timestamp.
  // Reject a missing/malformed timestamp, or one outside the ±TOLERANCE_SECS
  // window, BEFORE the HMAC check — a captured delivery is only valid briefly.
  const timestamp = parseWebhookTimestamp(request.headers.get('X-Webhook-Timestamp'));
  if (timestamp === null) {
    return NextResponse.json({ error: 'missing timestamp' }, { status: 401 });
  }
  const nowSecs = Math.floor(Date.now() / 1000);
  if (!isTimestampFresh(timestamp, nowSecs)) {
    return NextResponse.json({ error: 'stale timestamp' }, { status: 401 });
  }

  // 4. Verify HMAC signature over the timestamped payload "{timestamp}.{body}"
  // so the timestamp is bound to the signature and cannot be swapped.
  const header = request.headers.get('X-Webhook-Signature') ?? '';
  const signedPayload = `${timestamp}.${rawBody}`;
  const expected = 'sha256=' + createHmac('sha256', secret).update(signedPayload).digest('base64');

  const headerBuf = Buffer.from(header);
  const expectedBuf = Buffer.from(expected);

  // Guard unequal lengths before timingSafeEqual (it throws on length mismatch)
  if (headerBuf.length !== expectedBuf.length || !timingSafeEqual(headerBuf, expectedBuf)) {
    return NextResponse.json({ error: 'invalid signature' }, { status: 401 });
  }

  // 5. Parse body and validate shape
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

  // 6. Validate screen contains a slash and has non-empty second segment
  if (!screen.includes('/')) {
    return NextResponse.json({ error: 'invalid screen' }, { status: 422 });
  }

  // 7. Revalidate tags
  const tags = layoutTagsFor(screen);
  if (!tags) {
    return NextResponse.json({ error: 'invalid screen' }, { status: 422 });
  }

  for (const tag of tags) {
    revalidateTag(tag, 'default');
  }

  return NextResponse.json({ revalidated: true, tags });
}
