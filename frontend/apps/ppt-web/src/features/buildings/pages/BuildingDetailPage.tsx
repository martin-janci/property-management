/**
 * BuildingDetailPage - displays detailed information about a building.
 *
 * Epic 81: Frontend API Integration (Story 81.2)
 *
 * @see Story 81.2 - Wire buildings page to API
 */

import type { BuildingStatus, BuildingType } from '@ppt/api-client';
import { isSafeImageUrl } from '@ppt/shared';
import { Link } from 'react-router-dom';
import { UnitsSection } from '../components';
import { useBuilding, useBuildingCommonAreas, useBuildingFloors } from '../hooks';

interface BuildingDetailPageProps {
  buildingId: string;
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

export function BuildingDetailPage({ buildingId }: BuildingDetailPageProps) {
  const {
    data: building,
    isLoading: buildingLoading,
    error: buildingError,
  } = useBuilding(buildingId);
  const { data: floors, isLoading: floorsLoading } = useBuildingFloors(buildingId, !!building);
  const { data: commonAreas, isLoading: areasLoading } = useBuildingCommonAreas(
    buildingId,
    !!building
  );

  if (buildingLoading) {
    return (
      <div className="max-w-4xl mx-auto px-4 py-8">
        <div className="flex justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      </div>
    );
  }

  if (buildingError || !building) {
    return (
      <div className="max-w-4xl mx-auto px-4 py-8">
        <div className="bg-red-50 border border-red-200 rounded-md p-4">
          <h3 className="text-sm font-medium text-red-800">Building not found</h3>
          <p className="mt-2 text-sm text-red-700">
            {buildingError instanceof Error
              ? buildingError.message
              : 'The requested building could not be found.'}
          </p>
          <Link
            to="/buildings"
            className="mt-4 inline-block text-sm text-blue-600 hover:text-blue-800"
          >
            Back to buildings
          </Link>
        </div>
      </div>
    );
  }

  const addressLine = [
    building.address.street,
    building.address.city,
    building.address.state,
    building.address.postalCode,
    building.address.country,
  ]
    .filter(Boolean)
    .join(', ');

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Back Link */}
      <Link
        to="/buildings"
        className="inline-flex items-center text-sm text-gray-500 hover:text-gray-700 mb-6"
      >
        <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        Back to buildings
      </Link>

      {/* Header */}
      <div className="bg-white rounded-lg shadow-md overflow-hidden mb-6">
        {/* Building Image */}
        <div className="h-64 bg-gray-200 relative">
          {isSafeImageUrl(building.photoUrl) ? (
            <img
              src={building.photoUrl}
              alt={building.name}
              className="w-full h-full object-cover"
            />
          ) : (
            <div className="w-full h-full flex items-center justify-center text-gray-400">
              <svg className="w-24 h-24" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"
                />
              </svg>
            </div>
          )}
        </div>

        {/* Building Info */}
        <div className="p-6">
          <div className="flex items-start justify-between">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">{building.name}</h1>
              <p className="text-gray-500 mt-1">{addressLine}</p>
            </div>
            <span
              className={`px-3 py-1 rounded-full text-sm font-medium ${STATUS_COLORS[building.status]}`}
            >
              {STATUS_LABELS[building.status]}
            </span>
          </div>

          {building.description && <p className="mt-4 text-gray-600">{building.description}</p>}

          {/* Stats */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mt-6">
            <div className="bg-gray-50 rounded-lg p-4">
              <p className="text-sm text-gray-500">Type</p>
              <p className="text-lg font-semibold text-gray-900">{TYPE_LABELS[building.type]}</p>
            </div>
            <div className="bg-gray-50 rounded-lg p-4">
              <p className="text-sm text-gray-500">Units</p>
              <p className="text-lg font-semibold text-gray-900">{building.unitCount}</p>
            </div>
            <div className="bg-gray-50 rounded-lg p-4">
              <p className="text-sm text-gray-500">Floors</p>
              <p className="text-lg font-semibold text-gray-900">{building.floorCount}</p>
            </div>
            {building.yearBuilt && (
              <div className="bg-gray-50 rounded-lg p-4">
                <p className="text-sm text-gray-500">Year Built</p>
                <p className="text-lg font-semibold text-gray-900">{building.yearBuilt}</p>
              </div>
            )}
            {building.totalAreaM2 && (
              <div className="bg-gray-50 rounded-lg p-4">
                <p className="text-sm text-gray-500">Total Area</p>
                <p className="text-lg font-semibold text-gray-900">
                  {building.totalAreaM2.toLocaleString()} m2
                </p>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Floors Section */}
      <div className="bg-white rounded-lg shadow-md p-6 mb-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Floors</h2>
        {floorsLoading ? (
          <div className="flex justify-center py-4">
            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600" />
          </div>
        ) : floors && floors.length > 0 ? (
          <div className="space-y-2">
            {floors.map((floor) => (
              <div
                key={floor.id}
                className="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
              >
                <div>
                  <span className="font-medium">{floor.name || `Floor ${floor.number}`}</span>
                  <span className="text-gray-500 text-sm ml-2">({floor.unitCount} units)</span>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-gray-500 text-sm">No floors defined for this building.</p>
        )}
      </div>

      {/* Units Section */}
      <UnitsSection buildingId={buildingId} />

      {/* Common Areas Section */}
      <div className="bg-white rounded-lg shadow-md p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Common Areas</h2>
        {areasLoading ? (
          <div className="flex justify-center py-4">
            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600" />
          </div>
        ) : commonAreas && commonAreas.length > 0 ? (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {commonAreas.map((area) => (
              <div key={area.id} className="p-4 bg-gray-50 rounded-lg">
                <h3 className="font-medium text-gray-900">{area.name}</h3>
                <p className="text-sm text-gray-500 capitalize">{area.type.replace('_', ' ')}</p>
                {area.description && (
                  <p className="text-sm text-gray-600 mt-1">{area.description}</p>
                )}
                {area.areaM2 && <p className="text-sm text-gray-500 mt-1">{area.areaM2} m2</p>}
              </div>
            ))}
          </div>
        ) : (
          <p className="text-gray-500 text-sm">No common areas defined for this building.</p>
        )}
      </div>
    </div>
  );
}

BuildingDetailPage.displayName = 'BuildingDetailPage';
