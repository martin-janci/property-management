/**
 * BuildingLocationMap - renders a building's geocoded location on a map.
 *
 * Story 3.1 AC3: the system geocodes the building address and displays the
 * building on a map. Coordinates are resolved server-side (see BIT-200) and
 * surfaced via `Building.location`; this component renders them.
 *
 * Uses the keyless OpenStreetMap embed (no API key, no extra dependency) so it
 * works in CI and offline dev without configuration. Renders nothing when the
 * building has no resolved coordinates, so it degrades gracefully while
 * geocoding is unconfigured or pending.
 */

import type { GeoLocation } from '@ppt/api-client';

interface BuildingLocationMapProps {
  location?: GeoLocation;
  /** Building name, used for the marker's accessible label. */
  name?: string;
}

/** Clamp + round a coordinate for stable, URL-safe embed parameters. */
function fmt(value: number): string {
  return value.toFixed(6);
}

export function BuildingLocationMap({ location, name }: BuildingLocationMapProps) {
  if (!location || !Number.isFinite(location.latitude) || !Number.isFinite(location.longitude)) {
    return null;
  }

  const { latitude, longitude } = location;

  // A small bounding box around the point keeps the marker centred at a
  // street-level zoom in the keyless embed.
  const delta = 0.0045;
  const bbox = [
    fmt(longitude - delta),
    fmt(latitude - delta),
    fmt(longitude + delta),
    fmt(latitude + delta),
  ].join('%2C');

  const embedUrl =
    `https://www.openstreetmap.org/export/embed.html?bbox=${bbox}` +
    `&layer=mapnik&marker=${fmt(latitude)}%2C${fmt(longitude)}`;
  const viewUrl = `https://www.openstreetmap.org/?mlat=${fmt(latitude)}&mlon=${fmt(longitude)}#map=16/${fmt(latitude)}/${fmt(longitude)}`;

  return (
    <div className="bg-white rounded-lg shadow-md p-6 mb-6">
      <div className="flex items-start justify-between mb-4">
        <h2 className="text-lg font-semibold text-gray-900">Location</h2>
        <a
          href={viewUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="text-sm text-blue-600 hover:text-blue-800"
        >
          Open in OpenStreetMap
        </a>
      </div>
      <div className="overflow-hidden rounded-lg border border-gray-200">
        <iframe
          title={name ? `Map showing ${name}` : 'Building location map'}
          src={embedUrl}
          className="w-full h-72 border-0"
          loading="lazy"
          referrerPolicy="no-referrer"
        />
      </div>
      <p className="mt-3 text-sm text-gray-500">
        {fmt(latitude)}, {fmt(longitude)}
      </p>
    </div>
  );
}

BuildingLocationMap.displayName = 'BuildingLocationMap';
