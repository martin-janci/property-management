/**
 * Resolve the listing price for submission from the edit wizard.
 *
 * Returns the numeric price only when a genuinely-entered positive number is
 * present; returns `null` for an empty field or any non-finite / non-positive
 * value. Callers MUST reject a `null` result with validation rather than
 * persisting a fallback — the previous `Number(form.price) || 0` coercion
 * silently turned an empty/invalid field into a €0 listing.
 */
export function resolveListingPrice(price: number | ''): number | null {
  const value = typeof price === 'number' ? price : Number(price);
  if (price === '' || !Number.isFinite(value) || value <= 0) return null;
  return value;
}
