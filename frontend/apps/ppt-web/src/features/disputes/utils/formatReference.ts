/**
 * Format a dispute's UUID into a short, human-friendly reference code.
 *
 * The backend identifies disputes by UUID and does not (yet) expose a
 * dedicated short code, so we derive a compact `DSP-XXXXXXXX` label from
 * the first 8 characters of the UUID rather than uppercasing the whole
 * (unreadable) UUID. Hyphens are stripped first so the 8 characters are
 * always hex digits regardless of where the first UUID hyphen falls.
 */
export function formatDisputeReference(id: string): string {
  const compact = id.replace(/-/g, '');
  return `DSP-${compact.slice(0, 8).toUpperCase()}`;
}
