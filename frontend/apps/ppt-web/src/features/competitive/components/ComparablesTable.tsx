/**
 * Comparables Table Component
 * Story 70.4: Comparable Sales/Rentals
 *
 * Displays comparable properties in a sortable table format.
 */

import { useMemo, useState } from 'react';

export interface ComparableProperty {
  id: string;
  propertyType: string;
  transactionType: string;
  street?: string;
  city: string;
  postalCode?: string;
  sizeSqm: number;
  rooms?: number;
  floor?: number;
  price: number;
  pricePerSqm: number;
  currency: string;
  distanceMeters: number;
  similarityScore: number;
  transactionDate?: string;
  isActive: boolean;
}

export interface ComparablesTableProps {
  comparables: ComparableProperty[];
  sourceProperty?: {
    sizeSqm: number;
    rooms?: number;
    price: number;
    pricePerSqm: number;
    currency: string;
  };
  onViewDetails?: (comparable: ComparableProperty) => void;
  className?: string;
}

type SortField = 'similarity' | 'price' | 'pricePerSqm' | 'distance' | 'size' | 'date';
type SortDirection = 'asc' | 'desc';

/**
 * Displays comparable properties with sorting and comparison.
 */
export function ComparablesTable({
  comparables,
  sourceProperty,
  onViewDetails,
  className = '',
}: ComparablesTableProps) {
  const [sortField, setSortField] = useState<SortField>('similarity');
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDirection((prev) => (prev === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortField(field);
      setSortDirection(field === 'similarity' ? 'desc' : 'asc');
    }
  };

  const sortedComparables = useMemo(() => {
    return [...comparables].sort((a, b) => {
      let aVal: number;
      let bVal: number;

      switch (sortField) {
        case 'similarity':
          aVal = a.similarityScore;
          bVal = b.similarityScore;
          break;
        case 'price':
          aVal = a.price;
          bVal = b.price;
          break;
        case 'pricePerSqm':
          aVal = a.pricePerSqm;
          bVal = b.pricePerSqm;
          break;
        case 'distance':
          aVal = a.distanceMeters;
          bVal = b.distanceMeters;
          break;
        case 'size':
          aVal = a.sizeSqm;
          bVal = b.sizeSqm;
          break;
        case 'date':
          aVal = a.transactionDate ? new Date(a.transactionDate).getTime() : 0;
          bVal = b.transactionDate ? new Date(b.transactionDate).getTime() : 0;
          break;
        default:
          return 0;
      }

      return sortDirection === 'asc' ? aVal - bVal : bVal - aVal;
    });
  }, [comparables, sortField, sortDirection]);

  const formatPrice = (price: number, currency: string) => {
    return new Intl.NumberFormat('sk-SK', {
      style: 'currency',
      currency,
      maximumFractionDigits: 0,
    }).format(price);
  };

  const formatDistance = (meters: number) => {
    if (meters < 1000) {
      return `${Math.round(meters)}m`;
    }
    return `${(meters / 1000).toFixed(1)}km`;
  };

  const formatDate = (dateStr?: string) => {
    if (!dateStr) return '-';
    return new Date(dateStr).toLocaleDateString('sk-SK', {
      year: 'numeric',
      month: 'short',
    });
  };

  const getComparisonClass = (value: number, sourceValue?: number, inverse = false) => {
    if (sourceValue === undefined) return '';
    const diff = ((value - sourceValue) / sourceValue) * 100;
    if (Math.abs(diff) < 5) return '';
    if (inverse) {
      return diff > 0 ? 'text-red-600' : 'text-green-600';
    }
    return diff > 0 ? 'text-green-600' : 'text-red-600';
  };

  const SortHeader = ({ field, children }: { field: SortField; children: React.ReactNode }) => (
    <th
      className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100"
      onClick={() => handleSort(field)}
    >
      <div className="flex items-center gap-1">
        {children}
        <span className="text-gray-400">
          {sortField === field && (sortDirection === 'asc' ? ' ^' : ' v')}
        </span>
      </div>
    </th>
  );

  return (
    <div className={`bg-white rounded-lg shadow-sm border ${className}`}>
      <div className="p-4 border-b">
        <h3 className="text-lg font-semibold text-gray-900">
          Comparable Properties ({comparables.length})
        </h3>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead className="bg-gray-50 border-b">
            <tr>
              <SortHeader field="similarity">Match</SortHeader>
              <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                Property
              </th>
              <SortHeader field="size">Size</SortHeader>
              <SortHeader field="price">Price</SortHeader>
              <SortHeader field="pricePerSqm">Price/sqm</SortHeader>
              <SortHeader field="distance">Distance</SortHeader>
              <SortHeader field="date">Date</SortHeader>
              <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                Status
              </th>
              {onViewDetails && (
                <th className="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase">
                  Actions
                </th>
              )}
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-100">
            {sortedComparables.map((comp) => (
              <tr key={comp.id} className="hover:bg-gray-50">
                <td className="px-4 py-3">
                  <div className="flex items-center gap-2">
                    <div className={'w-10 h-2 rounded-full overflow-hidden bg-gray-200'}>
                      <div
                        className={`h-full rounded-full ${
                          comp.similarityScore >= 80
                            ? 'bg-green-500'
                            : comp.similarityScore >= 60
                              ? 'bg-yellow-500'
                              : 'bg-red-500'
                        }`}
                        style={{ width: `${comp.similarityScore}%` }}
                      />
                    </div>
                    <span className="text-gray-900 font-medium">{comp.similarityScore}%</span>
                  </div>
                </td>
                <td className="px-4 py-3">
                  <div>
                    <div className="font-medium text-gray-900 capitalize">{comp.propertyType}</div>
                    <div className="text-gray-500">
                      {comp.city}
                      {comp.rooms && ` - ${comp.rooms} rooms`}
                    </div>
                  </div>
                </td>
                <td className="px-4 py-3">
                  <span className={getComparisonClass(comp.sizeSqm, sourceProperty?.sizeSqm)}>
                    {comp.sizeSqm} m2
                  </span>
                </td>
                <td className="px-4 py-3">
                  <span
                    className={`font-medium ${getComparisonClass(comp.price, sourceProperty?.price, true)}`}
                  >
                    {formatPrice(comp.price, comp.currency)}
                  </span>
                </td>
                <td className="px-4 py-3">
                  <span
                    className={getComparisonClass(
                      comp.pricePerSqm,
                      sourceProperty?.pricePerSqm,
                      true
                    )}
                  >
                    {formatPrice(comp.pricePerSqm, comp.currency)}
                  </span>
                </td>
                <td className="px-4 py-3 text-gray-500">{formatDistance(comp.distanceMeters)}</td>
                <td className="px-4 py-3 text-gray-500">{formatDate(comp.transactionDate)}</td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex px-2 py-1 text-xs font-medium rounded-full ${
                      comp.isActive ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-700'
                    }`}
                  >
                    {comp.isActive ? 'Active' : 'Sold'}
                  </span>
                </td>
                {onViewDetails && (
                  <td className="px-4 py-3 text-right">
                    <button
                      type="button"
                      onClick={() => onViewDetails(comp)}
                      className="text-blue-600 hover:text-blue-800"
                    >
                      View
                    </button>
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {comparables.length === 0 && (
        <div className="p-8 text-center text-gray-500">No comparable properties found.</div>
      )}

      {/* Summary stats */}
      {comparables.length > 0 && (
        <div className="p-4 border-t bg-gray-50">
          <div className="grid grid-cols-4 gap-4 text-center">
            <div>
              <div className="text-sm font-medium text-gray-900">
                {Math.round(
                  comparables.reduce((sum, c) => sum + c.pricePerSqm, 0) / comparables.length
                )}{' '}
                {comparables[0]?.currency ?? 'EUR'}
              </div>
              <div className="text-xs text-gray-500">Avg. Price/sqm</div>
            </div>
            <div>
              <div className="text-sm font-medium text-gray-900">
                {formatPrice(
                  Math.min(...comparables.map((c) => c.price)),
                  comparables[0]?.currency ?? 'EUR'
                )}
              </div>
              <div className="text-xs text-gray-500">Min Price</div>
            </div>
            <div>
              <div className="text-sm font-medium text-gray-900">
                {formatPrice(
                  Math.max(...comparables.map((c) => c.price)),
                  comparables[0]?.currency ?? 'EUR'
                )}
              </div>
              <div className="text-xs text-gray-500">Max Price</div>
            </div>
            <div>
              <div className="text-sm font-medium text-gray-900">
                {Math.round(
                  comparables.reduce((sum, c) => sum + c.similarityScore, 0) / comparables.length
                )}
                %
              </div>
              <div className="text-xs text-gray-500">Avg. Match</div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
