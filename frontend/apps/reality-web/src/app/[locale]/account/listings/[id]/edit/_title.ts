/**
 * Resolve the listing title sent by the edit wizard.
 *
 * The wizard has no title field, and it used to rebuild the title from the
 * first 80 characters of the description on every save, overwriting whatever
 * title the listing already had. Keep the loaded title; derive one only when
 * the listing has none.
 */
export function resolveListingTitle(
  loadedTitle: string | null | undefined,
  form: { description?: string; propertyType: string; city: string }
): string {
  const existing = loadedTitle?.trim();
  if (existing) return existing;
  return form.description?.trim().slice(0, 80) || `${form.propertyType} ${form.city.trim()}`.trim();
}
