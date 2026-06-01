/**
 * BuildingCard Component
 *
 * Displays a building summary in a card format.
 *
 * @see Story 81.2 - Wire buildings page to API
 */

import type { Building, BuildingStatus, BuildingType } from '@ppt/api-client';
import { isSafeImageUrl } from '@ppt/shared';

interface BuildingCardProps {
  building: Building;
  onView: (id: string) => void;
  onEdit?: (id: string) => void;
}

const STATUS_LABELS: Record<BuildingStatus, string> = {
  active: 'Active',
  under_construction: 'Under Construction',
  renovation: 'Renovation',
  inactive: 'Inactive',
};

const STATUS_COLORS: Record<BuildingStatus, string> = {
  active: 'bg-green-100 text-green-800',
  under_construction: 'bg-yellow-100 text-yellow-800',
  renovation: 'bg-orange-100 text-orange-800',
  inactive: 'bg-gray-100 text-gray-800',
};

const TYPE_LABELS: Record<BuildingType, string> = {
  residential: 'Residential',
  commercial: 'Commercial',
  mixed: 'Mixed Use',
  industrial: 'Industrial',
};

export function BuildingCard({ building, onView, onEdit }: BuildingCardProps) {
  const { id, name, address, type, status, unitCount, floorCount, photoUrl } = building;

  // Only render server-provided photo URLs that pass the http(s)/relative
  // allowlist — guards against injected javascript:/data: image sources.
  const safePhotoUrl = isSafeImageUrl(photoUrl) ? photoUrl : undefined;

  const addressLine = [address.street, address.city, address.postalCode].filter(Boolean).join(', ');

  return (
    <div className="bg-white rounded-lg shadow-md overflow-hidden hover:shadow-lg transition-shadow">
      {/* Building Image */}
      <div className="h-48 bg-gray-200 relative">
        {safePhotoUrl ? (
          <img src={safePhotoUrl} alt={name} className="w-full h-full object-cover" />
        ) : (
          <div className="w-full h-full flex items-center justify-center text-gray-400">
            <svg
              className="w-16 h-16"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"
              />
            </svg>
          </div>
        )}
        {/* Status Badge */}
        <span
          className={`absolute top-2 right-2 px-2 py-1 rounded-full text-xs font-medium ${STATUS_COLORS[status]}`}
        >
          {STATUS_LABELS[status]}
        </span>
      </div>

      {/* Building Info */}
      <div className="p-4">
        <h3 className="text-lg font-semibold text-gray-900 mb-1">{name}</h3>
        <p className="text-sm text-gray-500 mb-3">{addressLine}</p>

        {/* Building Details */}
        <div className="flex items-center gap-4 text-sm text-gray-600 mb-4">
          <span className="flex items-center gap-1">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"
              />
            </svg>
            {unitCount} units
          </span>
          <span className="flex items-center gap-1">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 6h16M4 10h16M4 14h16M4 18h16"
              />
            </svg>
            {floorCount} floors
          </span>
        </div>

        {/* Type Badge */}
        <div className="flex items-center justify-between">
          <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
            {TYPE_LABELS[type]}
          </span>

          {/* Actions */}
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => onView(id)}
              className="text-blue-600 hover:text-blue-800 text-sm font-medium"
            >
              View
            </button>
            {onEdit && (
              <button
                type="button"
                onClick={() => onEdit(id)}
                className="text-gray-600 hover:text-gray-800 text-sm font-medium"
              >
                Edit
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

BuildingCard.displayName = 'BuildingCard';
