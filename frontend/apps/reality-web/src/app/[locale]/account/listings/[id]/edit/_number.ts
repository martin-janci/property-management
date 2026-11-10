/**
 * Resolve an optional numeric listing field (area / rooms / floor) for a PATCH
 * submission from the edit wizard.
 *
 * The form's numeric fields are typed `number | ''`, but the backend serializes
 * Decimals as EITHER a number or a string depending on the serializer (see the
 * `MyListing` contract note in `lib/realtor-api.ts` — "callers must coerce
 * before formatting"), so at runtime a loaded value can arrive as a string.
 * This helper collapses all of those shapes to a single optional number for the
 * update payload: a finite number when one is present, or `undefined` when the
 * field is empty / non-numeric so the PATCH omits it and the backend preserves
 * the stored value.
 *
 * This logic lived inline in `page.tsx`'s `handleSave`, duplicated across three
 * fields and rewritten on each save/i18n fix. It is extracted here behind one
 * small unit-tested function, mirroring the sibling `_price.ts` / `_status.ts`
 * helpers that already de-churned this page.
 */
export function resolveOptionalNumber(
  value: number | string | null | undefined
): number | undefined {
  // Empty / missing fields are omitted from the PATCH so the backend keeps the
  // stored value.
  if (value === '' || value === null || value === undefined) return undefined;
  // Coerce number OR string the same way (the previous inline
  // `Number(x) || undefined` dropped a genuinely-entered zero that arrived as
  // the string '0'); omit only genuinely non-numeric values.
  const n = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(n) ? n : undefined;
}
