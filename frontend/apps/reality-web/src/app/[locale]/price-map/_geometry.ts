/**
 * Presentational geometry + color scale for the price-map choropleth.
 *
 * This is NOT price data. The real price/listing/trend numbers come from
 * reality-server (`/api/v1/price-map`) via `usePriceMap` in
 * `@ppt/reality-api-client`. The server aggregates by city and carries no
 * map geometry, so the SVG outlines below are static scaffolding keyed by a
 * normalized district id. A district the API returns that has no geometry
 * entry still renders in the list and insights — it just isn't drawn on the
 * SVG. Add an entry here to light up a new city/district on the map.
 */

/** Lowercase, strip diacritics, collapse non-alphanumerics to '-'. */
export function normalizeDistrictId(value: string): string {
  return value
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/(^-|-$)/g, '');
}

/** Static SVG outlines (viewBox "0 80 400 200"), keyed by normalized id. */
export const DISTRICT_GEOMETRY: Record<string, string> = {
  // Seeded city — a single Bratislava outline so the map renders against the
  // dev stack. Replace with finer district outlines once the endpoint groups
  // below city level (UX follow-up).
  bratislava: 'M150,110 L260,95 L320,140 L300,205 L210,235 L150,200 L135,150 Z',
};

/** Geometry lookup tolerant of either the raw district id or its name. */
export function geometryFor(districtId: string, districtName: string): string | undefined {
  return (
    DISTRICT_GEOMETRY[normalizeDistrictId(districtId)] ??
    DISTRICT_GEOMETRY[normalizeDistrictId(districtName)]
  );
}

/** Choropleth color from €/m²; matches the legend buckets. */
export function priceToColor(pricePerM2: number | null | undefined): string {
  if (pricePerM2 == null) return '#cbd5e1'; // no data
  if (pricePerM2 >= 5500) return '#dc2626';
  if (pricePerM2 >= 5000) return '#ea580c';
  if (pricePerM2 >= 3500) return '#d97706';
  return '#65a30d';
}
