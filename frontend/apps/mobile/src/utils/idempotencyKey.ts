/**
 * Client-generated idempotency keys for offline-safe fault creation
 * (Story 4.1 / 2B.7, gap #970.6).
 *
 * The api-server `POST /api/v1/faults` create is idempotent on a body
 * `idempotency_key`: replaying the same key returns the existing fault instead
 * of inserting a duplicate (backend gap #970 / PR #989). The offline queue can
 * retry a create several times across reconnects, so each report must carry a
 * stable key generated once, at submit time, and reused on every replay.
 */

/**
 * Generate an RFC-4122-ish v4 UUID string.
 *
 * `Math.random` is sufficient here: the key only needs to be unique per device
 * per report so the server can dedupe replays — it is not a security token.
 * React Native does not ship `crypto.randomUUID` on all engines, so we avoid
 * relying on it.
 */
export function generateIdempotencyKey(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (char) => {
    const rand = (Math.random() * 16) | 0;
    const value = char === 'x' ? rand : (rand & 0x3) | 0x8;
    return value.toString(16);
  });
}
