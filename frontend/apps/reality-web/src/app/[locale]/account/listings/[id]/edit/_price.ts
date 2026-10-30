/**
 * Resolve the listing price for submission from the edit wizard.
 *
 * NOTE: pre-fix implementation — mirrors the old inline
 * `Number(form.price) || 0` coercion in `handleSave`, which silently turned an
 * empty / invalid field into a €0 listing. The accompanying test asserts the
 * intended (post-fix) contract and therefore fails against this version.
 */
export function resolveListingPrice(price: number | ''): number | null {
  return typeof price === 'number' ? price : Number(price) || 0;
}
