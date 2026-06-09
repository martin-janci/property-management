/**
 * Price-map presentation config — geometry and the choropleth colour scale.
 *
 * This file holds NO price data. Every number on the page comes live from
 * reality-server `GET /api/v1/price-map` (see `usePriceMap`). What lives here
 * is purely how to *draw* the map: the simplified SVG outlines for the
 * Bratislava city districts and the price→colour bucket function shared by
 * the map, legend and list.
 *
 * Districts whose live `district_name` has no geometry entry still render in
 * the list/detail panels; the SVG simply omits them (and hides entirely when
 * none of the live rows have geometry), so there is never an empty map.
 */

export interface PriceMapFilter {
  propertyType: 'all' | 'apartment' | 'house' | 'land';
  transactionType: 'sale' | 'rent';
}

/** Normalise a district/city name for geometry lookup (diacritics-insensitive). */
function normalize(name: string): string {
  return name
    .toLowerCase()
    .normalize('NFD')
    .replace(/\p{M}/gu, '') // strip combining diacritical marks
    .replace(/[^a-z0-9]/g, '');
}

/**
 * Simplified SVG outlines for Bratislava's city districts, keyed by the
 * normalised district name. Geometry only — no prices.
 */
const DISTRICT_SVG_PATHS: Record<string, string> = {
  // Bratislava I – Staré Mesto
  bratislavaistaremesto: 'M200,120 L240,100 L270,130 L260,160 L220,165 Z',
  staremesto: 'M200,120 L240,100 L270,130 L260,160 L220,165 Z',
  // Bratislava II – Ružinov
  bratislavaiiruzinov: 'M260,160 L310,140 L330,180 L300,200 L260,195 Z',
  ruzinov: 'M260,160 L310,140 L330,180 L300,200 L260,195 Z',
  // Bratislava III – Nové Mesto
  bratislavaiiinovemesto: 'M220,100 L240,80 L280,90 L270,130 L240,100 Z',
  novemesto: 'M220,100 L240,80 L280,90 L270,130 L240,100 Z',
  // Bratislava IV – Karlova Ves
  bratislavaivkarlovaves: 'M140,130 L200,120 L220,165 L180,175 L140,160 Z',
  karlovaves: 'M140,130 L200,120 L220,165 L180,175 L140,160 Z',
  // Bratislava V – Petržalka
  bratislavavpetrzalka: 'M180,175 L220,165 L260,195 L260,240 L200,245 L160,220 Z',
  petrzalka: 'M180,175 L220,165 L260,195 L260,240 L200,245 L160,220 Z',
};

/** Return the SVG outline for a district name, or null when unknown. */
export function geometryFor(districtName: string): string | null {
  return DISTRICT_SVG_PATHS[normalize(districtName)] ?? null;
}

/** Neutral fill for districts/cities with no priced listings. */
export const NO_DATA_COLOR = '#cbd5e1';

/**
 * Choropleth colour for an average €/m². Buckets mirror the on-screen legend.
 * `null` (no priced listings in the period) → neutral grey.
 */
export function colorForPrice(avgPricePerM2: number | null): string {
  if (avgPricePerM2 == null) return NO_DATA_COLOR;
  if (avgPricePerM2 < 3500) return '#65a30d';
  if (avgPricePerM2 < 5000) return '#d97706';
  if (avgPricePerM2 < 5500) return '#ea580c';
  return '#dc2626';
}

/** Legend rows (colour + Slovak label), shared with the map overlay. */
export const LEGEND_ITEMS: { color: string; label: string }[] = [
  { color: '#65a30d', label: 'do 3 500 €' },
  { color: '#d97706', label: '3 500 – 5 000 €' },
  { color: '#ea580c', label: '5 000 – 5 500 €' },
  { color: '#dc2626', label: 'nad 5 500 €' },
];
