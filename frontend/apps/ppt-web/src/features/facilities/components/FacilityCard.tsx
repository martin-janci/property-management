/**
 * Facility Card Component (Epic 56: Facility Booking).
 *
 * Displays a facility summary with booking actions.
 */

import type { FacilitySummary, FacilityType } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface FacilityCardProps {
  facility: FacilitySummary;
  onView?: (id: string) => void;
  onBook?: (id: string) => void;
  onEdit?: (id: string) => void;
  isManager?: boolean;
}

const facilityTypeIcons: Record<FacilityType, string> = {
  gym: 'M10 2a8 8 0 100 16 8 8 0 000-16zm0 14a6 6 0 110-12 6 6 0 010 12z',
  laundry: 'M3 3h14v14H3V3zm2 2v10h10V5H5z',
  meeting_room: 'M4 4h12v12H4V4zm2 2v8h8V6H6z',
  party_room: 'M10 2l2 6h6l-5 4 2 6-5-4-5 4 2-6-5-4h6z',
  sauna: 'M10 2a8 8 0 100 16 8 8 0 000-16z',
  pool: 'M2 10c2-2 4-2 6 0s4 2 6 0 4-2 6 0v4c-2 2-4 2-6 0s-4-2-6 0-4 2-6 0v-4z',
  playground: 'M10 2a8 8 0 100 16 8 8 0 000-16z',
  parking: 'M4 2h8a4 4 0 010 8H8v8H4V2z',
  storage: 'M2 4h16v12H2V4zm2 2v8h12V6H4z',
  garden: 'M10 2c-4 0-8 4-8 8s4 8 8 8 8-4 8-8-4-8-8-8z',
  bbq: 'M3 14h14v4H3v-4zM5 2h10v10H5V2z',
  bike_storage: 'M5 10a5 5 0 100 10 5 5 0 000-10zm10 0a5 5 0 100 10 5 5 0 000-10z',
  other: 'M10 2a8 8 0 100 16 8 8 0 000-16z',
};

export function FacilityCard({ facility, onView, onBook, onEdit, isManager }: FacilityCardProps) {
  const { t } = useTranslation();
  return (
    <div className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
      {/* Header with photo or icon */}
      <div className="flex items-start gap-4">
        {facility.photos && facility.photos.length > 0 ? (
          <img
            src={facility.photos[0]}
            alt={facility.name}
            className="w-20 h-20 object-cover rounded-lg"
          />
        ) : (
          <div className="w-20 h-20 bg-gray-100 rounded-lg flex items-center justify-center">
            <svg
              className="w-10 h-10 text-gray-400"
              fill="currentColor"
              viewBox="0 0 20 20"
              aria-hidden="true"
            >
              <path d={facilityTypeIcons[facility.facility_type]} />
            </svg>
          </div>
        )}

        <div className="flex-1">
          <h3 className="text-lg font-semibold text-gray-900">{facility.name}</h3>
          <div className="mt-1 flex items-center gap-2 flex-wrap">
            <span className="px-2 py-1 text-xs font-medium rounded bg-blue-100 text-blue-800">
              {t(`facilities.types.${facility.facility_type}`)}
            </span>
            {!facility.is_active && (
              <span className="px-2 py-1 text-xs font-medium rounded bg-red-100 text-red-800">
                {t('facilities.card.inactive')}
              </span>
            )}
            {facility.requires_approval && (
              <span className="px-2 py-1 text-xs font-medium rounded bg-yellow-100 text-yellow-800">
                {t('facilities.card.requiresApproval')}
              </span>
            )}
          </div>
          {facility.description && (
            <p className="mt-2 text-sm text-gray-600 line-clamp-2">{facility.description}</p>
          )}
        </div>
      </div>

      {/* Details */}
      <div className="mt-4 flex items-center gap-4 text-sm text-gray-500">
        {facility.capacity && (
          <span className="flex items-center gap-1">
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M10 9a3 3 0 100-6 3 3 0 000 6zm-7 9a7 7 0 1114 0H3z" />
            </svg>
            {t('facilities.card.capacity', { count: facility.capacity })}
          </span>
        )}
        {facility.location && (
          <span className="flex items-center gap-1">
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
              <path
                fillRule="evenodd"
                d="M5.05 4.05a7 7 0 119.9 9.9L10 18.9l-4.95-4.95a7 7 0 010-9.9zM10 11a2 2 0 100-4 2 2 0 000 4z"
                clipRule="evenodd"
              />
            </svg>
            {facility.location}
          </span>
        )}
        {facility.hourly_fee && (
          <span className="flex items-center gap-1">
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M8.433 7.418c.155-.103.346-.196.567-.267v1.698a2.305 2.305 0 01-.567-.267C8.07 8.34 8 8.114 8 8c0-.114.07-.34.433-.582zM11 12.849v-1.698c.22.071.412.164.567.267.364.243.433.468.433.582 0 .114-.07.34-.433.582a2.305 2.305 0 01-.567.267z" />
              <path
                fillRule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-13a1 1 0 10-2 0v.092a4.535 4.535 0 00-1.676.662C6.602 6.234 6 7.009 6 8c0 .99.602 1.765 1.324 2.246.48.32 1.054.545 1.676.662v1.941c-.391-.127-.68-.317-.843-.504a1 1 0 10-1.51 1.31c.562.649 1.413 1.076 2.353 1.253V15a1 1 0 102 0v-.092a4.535 4.535 0 001.676-.662C13.398 13.766 14 12.991 14 12c0-.99-.602-1.765-1.324-2.246A4.535 4.535 0 0011 9.092V7.151c.391.127.68.317.843.504a1 1 0 101.511-1.31c-.563-.649-1.413-1.076-2.354-1.253V5z"
                clipRule="evenodd"
              />
            </svg>
            {t('facilities.card.perHour', { fee: facility.hourly_fee })}
          </span>
        )}
      </div>

      {/* Actions */}
      <div className="mt-4 flex items-center gap-2 border-t pt-3">
        {onView && (
          <button
            type="button"
            onClick={() => onView(facility.id)}
            className="text-sm text-blue-600 hover:text-blue-800"
          >
            {t('facilities.card.viewDetails')}
          </button>
        )}
        {facility.is_bookable && facility.is_active && (
          <button
            type="button"
            onClick={() => onBook?.(facility.id)}
            className="text-sm px-3 py-1 bg-green-600 text-white rounded hover:bg-green-700"
          >
            {t('facilities.card.bookNow')}
          </button>
        )}
        {isManager && (
          <button
            type="button"
            onClick={() => onEdit?.(facility.id)}
            className="text-sm text-gray-600 hover:text-gray-800 ml-auto"
          >
            {t('facilities.card.edit')}
          </button>
        )}
      </div>
    </div>
  );
}
