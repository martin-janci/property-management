/**
 * Competitive Analysis Page
 * Epic 70: Competitive Feature Enhancements
 *
 * Main page for viewing all competitive features for a listing.
 */

import { useState } from 'react';
import { AMENITY_CATEGORIES, AmenitiesMap, type NearbyAmenity } from '../components/AmenitiesMap';
import { type ComparableProperty, ComparablesTable } from '../components/ComparablesTable';
import {
  ComparisonChart,
  type ComparisonEntry,
  type PriceRange,
  PriceRangeDisplay,
} from '../components/ComparisonChart';
import {
  type CompetitiveFeaturesStatus,
  CompetitiveStatusBadge,
} from '../components/CompetitiveStatusBadge';
import {
  type NeighborhoodInsights,
  NeighborhoodInsightsCard,
} from '../components/NeighborhoodInsightsCard';
import { type PricingAnalysis, PricingAnalysisPanel } from '../components/PricingAnalysisPanel';
import { type PricingSuggestion, PricingSuggestionCard } from '../components/PricingSuggestionCard';
import { VirtualTourForm } from '../components/VirtualTourForm';
import { TOUR_TYPES, type VirtualTour, VirtualTourViewer } from '../components/VirtualTourViewer';

export interface CompetitiveAnalysisPageProps {
  listingId: string;
}

// Mock data for demonstration
const mockTour: VirtualTour = {
  id: 'tour-1',
  listingId: 'listing-1',
  tourType: TOUR_TYPES.MATTERPORT,
  title: 'Full Property Tour',
  description: 'Explore the entire property in 3D',
  embedUrl: 'https://my.matterport.com/show/?m=example',
  thumbnailUrl: 'https://via.placeholder.com/400x300',
  displayOrder: 1,
  isFeatured: true,
  hotspots: [
    {
      id: 'hotspot-1',
      label: 'Living Room',
      description: 'Spacious living area with natural light',
      positionX: 45,
      positionY: 30,
    },
    {
      id: 'hotspot-2',
      label: 'Kitchen',
      description: 'Modern kitchen with appliances',
      positionX: 60,
      positionY: 40,
    },
  ],
};

const mockPricingSuggestion: PricingSuggestion = {
  id: 'pricing-1',
  listingId: 'listing-1',
  suggestedPriceLow: 185000,
  suggestedPriceMid: 195000,
  suggestedPriceHigh: 210000,
  currency: 'EUR',
  confidenceLevel: 'high',
  confidenceScore: 85,
  comparablesCount: 8,
  marketTrend: 'stable',
  seasonalAdjustment: 1.02,
  calculatedAt: new Date().toISOString(),
  validUntil: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString(),
};

const mockPricingAnalysis: PricingAnalysis = {
  suggestion: mockPricingSuggestion,
  factors: [
    {
      id: 'factor-1',
      factorType: 'location',
      factorName: 'Prime Location',
      impact: 5.0,
      explanation: 'Located in a desirable neighborhood with good amenities',
    },
    {
      id: 'factor-2',
      factorType: 'size',
      factorName: 'Above Average Size',
      impact: 3.5,
      explanation: 'Property size is larger than average for the area',
    },
    {
      id: 'factor-3',
      factorType: 'condition',
      factorName: 'Recently Renovated',
      impact: 4.0,
      explanation: 'Recent renovations add value to the property',
    },
  ],
  priceHistory: [
    {
      id: 'history-1',
      price: 185000,
      pricePerSqm: 2720,
      currency: 'EUR',
      recordedAt: new Date(Date.now() - 90 * 24 * 60 * 60 * 1000).toISOString(),
    },
    {
      id: 'history-2',
      price: 192000,
      pricePerSqm: 2825,
      currency: 'EUR',
      recordedAt: new Date(Date.now() - 60 * 24 * 60 * 60 * 1000).toISOString(),
    },
  ],
  comparablesUsed: [
    {
      id: 'comp-1',
      propertyType: 'apartment',
      city: 'Bratislava',
      sizeSqm: 68,
      price: 188000,
      pricePerSqm: 2765,
      similarityScore: 92,
    },
  ],
};

const mockNeighborhoodInsights: NeighborhoodInsights = {
  id: 'insights-1',
  listingId: 'listing-1',
  latitude: 48.1486,
  longitude: 17.1077,
  walkScore: 85,
  transitScore: 78,
  bikeScore: 72,
  population: 450000,
  medianAge: 38.5,
  medianIncome: 1850,
  crimeIndex: 25,
  safetyRating: 'good',
  dataSources: {
    walk_score: 'Walk Score API',
    amenities: 'OpenStreetMap',
    demographics: 'Statistical Office SR',
  },
  fetchedAt: new Date().toISOString(),
  validUntil: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString(),
};

const mockAmenities: NearbyAmenity[] = [
  {
    id: 'amenity-1',
    insightsId: 'insights-1',
    category: AMENITY_CATEGORIES.SUPERMARKET,
    name: 'Tesco Express',
    address: 'Obchodna 15',
    distanceMeters: 150,
    latitude: 48.149,
    longitude: 17.108,
    rating: 4.2,
  },
  {
    id: 'amenity-2',
    insightsId: 'insights-1',
    category: AMENITY_CATEGORIES.TRANSIT_STOP,
    name: 'Hodzovo namestie (tram)',
    distanceMeters: 200,
    latitude: 48.148,
    longitude: 17.107,
  },
  {
    id: 'amenity-3',
    insightsId: 'insights-1',
    category: AMENITY_CATEGORIES.SCHOOL,
    name: 'Zakladna skola Grosslingova',
    address: 'Grosslingova 18',
    distanceMeters: 450,
    latitude: 48.146,
    longitude: 17.11,
    rating: 4.5,
  },
  {
    id: 'amenity-4',
    insightsId: 'insights-1',
    category: AMENITY_CATEGORIES.PARK,
    name: 'Medicka zahrada',
    distanceMeters: 600,
    latitude: 48.145,
    longitude: 17.112,
    rating: 4.7,
  },
];

const mockComparables: ComparableProperty[] = [
  {
    id: 'comp-1',
    propertyType: 'apartment',
    transactionType: 'sale',
    city: 'Bratislava',
    sizeSqm: 72,
    rooms: 3,
    price: 198000,
    pricePerSqm: 2750,
    currency: 'EUR',
    distanceMeters: 350,
    similarityScore: 95,
    transactionDate: new Date(Date.now() - 20 * 24 * 60 * 60 * 1000).toISOString(),
    isActive: false,
  },
  {
    id: 'comp-2',
    propertyType: 'apartment',
    transactionType: 'sale',
    city: 'Bratislava',
    sizeSqm: 68,
    rooms: 2,
    price: 185000,
    pricePerSqm: 2720,
    currency: 'EUR',
    distanceMeters: 500,
    similarityScore: 88,
    transactionDate: new Date(Date.now() - 45 * 24 * 60 * 60 * 1000).toISOString(),
    isActive: false,
  },
  {
    id: 'comp-3',
    propertyType: 'apartment',
    transactionType: 'sale',
    city: 'Bratislava',
    sizeSqm: 75,
    rooms: 3,
    price: 210000,
    pricePerSqm: 2800,
    currency: 'EUR',
    distanceMeters: 800,
    similarityScore: 82,
    isActive: true,
  },
];

const mockComparisonEntries: ComparisonEntry[] = [
  {
    feature: 'Size (sqm)',
    sourceValue: '70',
    comparableValues: ['72', '68', '75'],
  },
  {
    feature: 'Rooms',
    sourceValue: '3',
    comparableValues: ['3', '2', '3'],
  },
  {
    feature: 'Price/sqm',
    sourceValue: '2785 EUR',
    comparableValues: ['2750 EUR', '2720 EUR', '2800 EUR'],
  },
  {
    feature: 'Floor',
    sourceValue: '4',
    comparableValues: ['3', '5', '2'],
  },
];

const mockPriceRange: PriceRange = {
  min: 185000,
  max: 210000,
  median: 198000,
  currency: 'EUR',
};

const mockStatus: CompetitiveFeaturesStatus = {
  listingId: 'listing-1',
  hasVirtualTours: true,
  virtualTourCount: 2,
  hasPricingAnalysis: true,
  pricingAnalysisValid: true,
  hasNeighborhoodInsights: true,
  neighborhoodInsightsValid: true,
  hasComparables: true,
  comparablesCount: 5,
};

/**
 * Main competitive analysis page component.
 */
export function CompetitiveAnalysisPage({ listingId }: CompetitiveAnalysisPageProps) {
  const [activeTab, setActiveTab] = useState<'tours' | 'pricing' | 'neighborhood' | 'comparables'>(
    'tours'
  );
  const [showTourForm, setShowTourForm] = useState(false);
  const [showPricingDetails, setShowPricingDetails] = useState(false);
  const [showAmenities, setShowAmenities] = useState(false);

  const tabs = [
    { key: 'tours', label: 'Virtual Tours', count: 2 },
    { key: 'pricing', label: 'Pricing Analysis', badge: 'High' },
    { key: 'neighborhood', label: 'Neighborhood' },
    { key: 'comparables', label: 'Comparables', count: 5 },
  ] as const;

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <div className="bg-white border-b">
        <div className="max-w-7xl mx-auto px-4 py-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">Competitive Analysis</h1>
              <p className="text-gray-500 mt-1">Listing: {listingId}</p>
            </div>
            <CompetitiveStatusBadge status={mockStatus} compact />
          </div>

          {/* Tabs */}
          <div className="mt-6 border-b -mb-px">
            <nav className="flex gap-4">
              {tabs.map((tab) => (
                <button
                  key={tab.key}
                  type="button"
                  onClick={() => setActiveTab(tab.key)}
                  className={`px-4 py-3 text-sm font-medium border-b-2 transition-colors ${
                    activeTab === tab.key
                      ? 'border-blue-600 text-blue-600'
                      : 'border-transparent text-gray-500 hover:text-gray-700'
                  }`}
                >
                  {tab.label}
                  {'count' in tab && tab.count && (
                    <span className="ml-2 px-2 py-0.5 text-xs bg-gray-100 rounded-full">
                      {tab.count}
                    </span>
                  )}
                  {'badge' in tab && tab.badge && (
                    <span className="ml-2 px-2 py-0.5 text-xs bg-green-100 text-green-700 rounded-full">
                      {tab.badge}
                    </span>
                  )}
                </button>
              ))}
            </nav>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="max-w-7xl mx-auto px-4 py-6">
        {/* Virtual Tours Tab */}
        {activeTab === 'tours' && (
          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold text-gray-900">Virtual Tours</h2>
              <button
                type="button"
                onClick={() => setShowTourForm(true)}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
              >
                Add Tour
              </button>
            </div>

            {showTourForm && (
              <div className="bg-white rounded-lg shadow-sm border p-6">
                <h3 className="text-lg font-medium mb-4">Add Virtual Tour</h3>
                <VirtualTourForm
                  onSubmit={async (_data) => {
                    // Virtual tour API not yet available - will be implemented in competitive features epic
                    setShowTourForm(false);
                  }}
                  onCancel={() => setShowTourForm(false)}
                />
              </div>
            )}

            <VirtualTourViewer
              tour={mockTour}
              onHotspotClick={(_hotspot) => {
                // Hotspot info panel not yet implemented - competitive features epic
              }}
              className="h-[500px]"
            />
          </div>
        )}

        {/* Pricing Tab */}
        {activeTab === 'pricing' && (
          <div className="space-y-6">
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <div className="lg:col-span-1">
                <PricingSuggestionCard
                  suggestion={mockPricingSuggestion}
                  currentPrice={195000}
                  onAnalyze={() => setShowPricingDetails(true)}
                />
              </div>
              <div className="lg:col-span-2">
                {showPricingDetails ? (
                  <PricingAnalysisPanel
                    analysis={mockPricingAnalysis}
                    onRefresh={() => {
                      // Pricing analysis refresh API not yet available
                    }}
                  />
                ) : (
                  <div className="bg-white rounded-lg shadow-sm border p-6 flex items-center justify-center h-full">
                    <div className="text-center">
                      <p className="text-gray-500 mb-4">
                        Click "View Full Analysis" to see detailed pricing breakdown
                      </p>
                      <button
                        type="button"
                        onClick={() => setShowPricingDetails(true)}
                        className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
                      >
                        View Analysis
                      </button>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Neighborhood Tab */}
        {activeTab === 'neighborhood' && (
          <div className="space-y-6">
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <div className="lg:col-span-1">
                <NeighborhoodInsightsCard
                  insights={mockNeighborhoodInsights}
                  onRefresh={() => {
                    // Neighborhood insights refresh API not yet available
                  }}
                  onViewAmenities={() => setShowAmenities(true)}
                />
              </div>
              <div className="lg:col-span-2">
                {showAmenities ? (
                  <AmenitiesMap
                    amenities={mockAmenities}
                    centerLatitude={mockNeighborhoodInsights.latitude}
                    centerLongitude={mockNeighborhoodInsights.longitude}
                  />
                ) : (
                  <div className="bg-white rounded-lg shadow-sm border p-6 flex items-center justify-center h-full min-h-[400px]">
                    <div className="text-center">
                      <svg
                        className="w-12 h-12 text-gray-400 mx-auto mb-4"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"
                        />
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M15 11a3 3 0 11-6 0 3 3 0 016 0z"
                        />
                      </svg>
                      <p className="text-gray-500 mb-4">
                        Click "View Nearby Amenities" to explore the neighborhood
                      </p>
                      <button
                        type="button"
                        onClick={() => setShowAmenities(true)}
                        className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
                      >
                        View Amenities
                      </button>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Comparables Tab */}
        {activeTab === 'comparables' && (
          <div className="space-y-6">
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <div className="lg:col-span-2">
                <ComparablesTable
                  comparables={mockComparables}
                  sourceProperty={{
                    sizeSqm: 70,
                    rooms: 3,
                    price: 195000,
                    pricePerSqm: 2785,
                    currency: 'EUR',
                  }}
                  onViewDetails={(_comp) => {
                    // Comparable property detail view not yet implemented
                  }}
                />
              </div>
              <div className="lg:col-span-1 space-y-6">
                <PriceRangeDisplay range={mockPriceRange} currentPrice={195000} />
                <ComparisonChart entries={mockComparisonEntries} />
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
