/**
 * layout-tags — maps a screen id to its Next.js cache tags.
 *
 * Lives outside the route module: Next's build-time typegen rejects extra
 * (non-HTTP) exports from app router route files, so the helper is imported
 * by both the /api/layout-revalidate route and its tests from here.
 *
 * Only screens that contain a slash with a non-empty second segment are valid
 * (e.g. "reality/listing-detail" → ["layout:listing-detail"]).
 */
export function layoutTagsFor(screen: string): string[] | null {
  const parts = screen.split('/');
  const segment = parts[1];
  // Require non-empty second segment (e.g., reject 'foo/')
  if (!segment) return null;
  return [`layout:${segment}`];
}
