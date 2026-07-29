import { createHmac, timingSafeEqual } from 'node:crypto';
import { revalidateTag } from 'next/cache';
import { NextResponse } from 'next/server';
import { trackEvent } from '@/lib/analytics';

// ---------------------------------------------------------------------------
// Analytics: layout_revalidate_received (issue #2532).
//
// The receiver emits one operational event on every return path so the
// layout publish → webhook → revalidate pipeline is observable end to end
// (docs/data/layout-publish-event-tracking.md §5.3). Emission is fire-and-
// forget via the shared `trackEvent` bus — it never throws and never changes
// the HTTP outcome. `delivery_id` is echoed from the webhook body (gap D) when
// present so this event joins the api-server `layout_change_published` /
// `layout_webhook_dispatched` events for the same mutation.
// ---------------------------------------------------------------------------

/** Receiver-side revalidate outcomes (events doc §4.4). */
type RevalidateOutcome =
  | 'revalidated'
  | 'disabled'
  | 'invalid_signature'
  | 'invalid_body'
  | 'invalid_screen';

function emitRevalidateReceived(props: {
  outcome: RevalidateOutcome;
  httpStatus: number;
  targetTenant: string;
  screen?: string | null;
  tags?: string[];
  deliveryId?: string | null;
}): void {
  trackEvent('layout_revalidate_received', {
    revalidate_outcome: props.outcome,
    http_status: props.httpStatus,
    target_tenant: props.targetTenant,
    screen: props.screen ?? null,
    tags: props.tags ?? [],
    delivery_id: props.deliveryId ?? null,
  });
}

/** Extract a string `delivery_id` from an already-parsed body, else null. */
function deliveryIdOf(body: unknown): string | null {
  if (typeof body === 'object' && body !== null) {
    const raw = (body as Record<string, unknown>).delivery_id;
    if (typeof raw === 'string') return raw;
  }
  return null;
}

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
  // Tenant scope on the receiver is expressed as the request host (events doc
  // §4.3); fall back to the global sentinel when absent.
  const targetTenant = request.headers.get('host') ?? '*';

  // 1. Secret must be configured
  const secret = process.env.LAYOUT_WEBHOOK_SECRET;
  if (!secret) {
    emitRevalidateReceived({ outcome: 'disabled', httpStatus: 503, targetTenant });
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
  // These auth rejections fold into the `invalid_signature` analytics outcome.
  const timestamp = parseWebhookTimestamp(request.headers.get('X-Webhook-Timestamp'));
  if (timestamp === null) {
    emitRevalidateReceived({ outcome: 'invalid_signature', httpStatus: 401, targetTenant });
    return NextResponse.json({ error: 'missing timestamp' }, { status: 401 });
  }
  const nowSecs = Math.floor(Date.now() / 1000);
  if (!isTimestampFresh(timestamp, nowSecs)) {
    emitRevalidateReceived({ outcome: 'invalid_signature', httpStatus: 401, targetTenant });
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
    emitRevalidateReceived({ outcome: 'invalid_signature', httpStatus: 401, targetTenant });
    return NextResponse.json({ error: 'invalid signature' }, { status: 401 });
  }

  // 5. Parse body and validate shape
  let body: unknown;
  try {
    body = JSON.parse(rawBody);
  } catch {
    emitRevalidateReceived({ outcome: 'invalid_body', httpStatus: 422, targetTenant });
    return NextResponse.json({ error: 'invalid body' }, { status: 422 });
  }

  // Echo the correlation id from the (now-verified, HMAC-covered) body when
  // present — the api-server adds it to the webhook payload (gap D).
  const deliveryId = deliveryIdOf(body);

  if (
    typeof body !== 'object' ||
    body === null ||
    typeof (body as Record<string, unknown>).screen !== 'string'
  ) {
    emitRevalidateReceived({ outcome: 'invalid_body', httpStatus: 422, targetTenant, deliveryId });
    return NextResponse.json({ error: 'invalid body' }, { status: 422 });
  }

  const screen = (body as { screen: string }).screen;

  // 6. Validate screen contains a slash and has non-empty second segment
  if (!screen.includes('/')) {
    emitRevalidateReceived({
      outcome: 'invalid_screen',
      httpStatus: 422,
      targetTenant,
      screen,
      deliveryId,
    });
    return NextResponse.json({ error: 'invalid screen' }, { status: 422 });
  }

  // 7. Revalidate tags
  const tags = layoutTagsFor(screen);
  if (!tags) {
    emitRevalidateReceived({
      outcome: 'invalid_screen',
      httpStatus: 422,
      targetTenant,
      screen,
      deliveryId,
    });
    return NextResponse.json({ error: 'invalid screen' }, { status: 422 });
  }

  for (const tag of tags) {
    revalidateTag(tag, 'default');
  }

  emitRevalidateReceived({
    outcome: 'revalidated',
    httpStatus: 200,
    targetTenant,
    screen,
    tags,
    deliveryId,
  });
  return NextResponse.json({ revalidated: true, tags });
}
