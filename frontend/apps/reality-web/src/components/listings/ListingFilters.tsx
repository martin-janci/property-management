/**
 * ListingFilters Component
 *
 * Search filters sidebar for listings page (Epic 44, Story 44.2).
 */

'use client';

import type { ListingFilters as FilterType, PropertyType } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { useState } from 'react';

interface ListingFiltersProps {
  filters: FilterType;
  onFiltersChange: (filters: FilterType) => void;
  onClose?: () => void;
  isMobile?: boolean;
}

type PropertyTypeKey = 'apartment' | 'house' | 'land' | 'commercial' | 'office' | 'garage';

const propertyTypes: { value: PropertyType; labelKey: PropertyTypeKey }[] = [
  { value: 'apartment', labelKey: 'apartment' },
  { value: 'house', labelKey: 'house' },
  { value: 'land', labelKey: 'land' },
  { value: 'commercial', labelKey: 'commercial' },
  { value: 'office', labelKey: 'office' },
  { value: 'garage', labelKey: 'garage' },
];

type PriceRangeKey =
  | 'priceUnder50k'
  | 'price50kTo100k'
  | 'price100kTo200k'
  | 'price200kTo500k'
  | 'price500kPlus';

const priceRanges: { min?: number; max?: number; labelKey: PriceRangeKey }[] = [
  { min: undefined, max: 50000, labelKey: 'priceUnder50k' },
  { min: 50000, max: 100000, labelKey: 'price50kTo100k' },
  { min: 100000, max: 200000, labelKey: 'price100kTo200k' },
  { min: 200000, max: 500000, labelKey: 'price200kTo500k' },
  { min: 500000, max: undefined, labelKey: 'price500kPlus' },
];

type AreaRangeKey = 'areaUnder50' | 'area50To100' | 'area100To150' | 'area150To200' | 'area200Plus';

const areaRanges: { min?: number; max?: number; labelKey: AreaRangeKey }[] = [
  { min: undefined, max: 50, labelKey: 'areaUnder50' },
  { min: 50, max: 100, labelKey: 'area50To100' },
  { min: 100, max: 150, labelKey: 'area100To150' },
  { min: 150, max: 200, labelKey: 'area150To200' },
  { min: 200, max: undefined, labelKey: 'area200Plus' },
];

export function ListingFilters({
  filters,
  onFiltersChange,
  onClose,
  isMobile = false,
}: ListingFiltersProps) {
  const t = useTranslations('filters');
  const [expandedSections, setExpandedSections] = useState<Record<string, boolean>>({
    propertyType: true,
    price: true,
    area: true,
    rooms: true,
  });

  const toggleSection = (section: string) => {
    setExpandedSections((prev) => ({
      ...prev,
      [section]: !prev[section],
    }));
  };

  const handlePropertyTypeChange = (type: PropertyType) => {
    const currentTypes = filters.propertyType || [];
    const newTypes = currentTypes.includes(type)
      ? currentTypes.filter((t) => t !== type)
      : [...currentTypes, type];
    onFiltersChange({ ...filters, propertyType: newTypes.length > 0 ? newTypes : undefined });
  };

  const handlePriceRangeChange = (min?: number, max?: number) => {
    onFiltersChange({ ...filters, priceMin: min, priceMax: max });
  };

  const handleAreaRangeChange = (min?: number, max?: number) => {
    onFiltersChange({ ...filters, areaMin: min, areaMax: max });
  };

  const handleRoomsChange = (min?: number) => {
    onFiltersChange({ ...filters, roomsMin: min });
  };

  const clearAllFilters = () => {
    onFiltersChange({
      transactionType: filters.transactionType,
      query: filters.query,
    });
  };

  const hasActiveFilters =
    (filters.propertyType && filters.propertyType.length > 0) ||
    filters.priceMin !== undefined ||
    filters.priceMax !== undefined ||
    filters.areaMin !== undefined ||
    filters.areaMax !== undefined ||
    filters.roomsMin !== undefined;

  return (
    <aside className={`filters ${isMobile ? 'mobile' : ''}`}>
      {/* Header */}
      <div className="filters-header">
        <h2 className="filters-title">{t('title')}</h2>
        <div className="filters-actions">
          {hasActiveFilters && (
            <button type="button" className="clear-button" onClick={clearAllFilters}>
              {t('clearAll')}
            </button>
          )}
          {isMobile && onClose && (
            <button
              type="button"
              className="close-button"
              onClick={onClose}
              aria-label="Close filters"
            >
              <svg
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                aria-hidden="true"
              >
                <path d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          )}
        </div>
      </div>

      {/* Property Type */}
      <div className="filter-section">
        <button
          type="button"
          className="section-header"
          onClick={() => toggleSection('propertyType')}
        >
          <span>{t('propertyType')}</span>
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            className={`chevron ${expandedSections.propertyType ? 'expanded' : ''}`}
            aria-hidden="true"
          >
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>
        {expandedSections.propertyType && (
          <div className="section-content">
            {propertyTypes.map((type) => (
              <label key={type.value} className="checkbox-label">
                <input
                  type="checkbox"
                  checked={filters.propertyType?.includes(type.value) ?? false}
                  onChange={() => handlePropertyTypeChange(type.value)}
                  className="checkbox"
                />
                <span>{t(type.labelKey)}</span>
              </label>
            ))}
          </div>
        )}
      </div>

      {/* Price Range */}
      <div className="filter-section">
        <button type="button" className="section-header" onClick={() => toggleSection('price')}>
          <span>{t('priceRange')}</span>
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            className={`chevron ${expandedSections.price ? 'expanded' : ''}`}
            aria-hidden="true"
          >
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>
        {expandedSections.price && (
          <div className="section-content">
            <label className="radio-label">
              <input
                type="radio"
                name="priceRange"
                checked={filters.priceMin === undefined && filters.priceMax === undefined}
                onChange={() => handlePriceRangeChange(undefined, undefined)}
                className="radio"
              />
              <span>{t('anyPrice')}</span>
            </label>
            {priceRanges.map((range) => (
              <label key={`price-${range.min ?? 0}-${range.max ?? 'max'}`} className="radio-label">
                <input
                  type="radio"
                  name="priceRange"
                  checked={filters.priceMin === range.min && filters.priceMax === range.max}
                  onChange={() => handlePriceRangeChange(range.min, range.max)}
                  className="radio"
                />
                <span>{t(range.labelKey, { currency: '\u20AC' })}</span>
              </label>
            ))}
          </div>
        )}
      </div>

      {/* Area Range */}
      <div className="filter-section">
        <button type="button" className="section-header" onClick={() => toggleSection('area')}>
          <span>{t('area')}</span>
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            className={`chevron ${expandedSections.area ? 'expanded' : ''}`}
            aria-hidden="true"
          >
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>
        {expandedSections.area && (
          <div className="section-content">
            <label className="radio-label">
              <input
                type="radio"
                name="areaRange"
                checked={filters.areaMin === undefined && filters.areaMax === undefined}
                onChange={() => handleAreaRangeChange(undefined, undefined)}
                className="radio"
              />
              <span>{t('anySize')}</span>
            </label>
            {areaRanges.map((range) => (
              <label key={`area-${range.min ?? 0}-${range.max ?? 'max'}`} className="radio-label">
                <input
                  type="radio"
                  name="areaRange"
                  checked={filters.areaMin === range.min && filters.areaMax === range.max}
                  onChange={() => handleAreaRangeChange(range.min, range.max)}
                  className="radio"
                />
                <span>{t(range.labelKey)}</span>
              </label>
            ))}
          </div>
        )}
      </div>

      {/* Rooms */}
      <div className="filter-section">
        <button type="button" className="section-header" onClick={() => toggleSection('rooms')}>
          <span>{t('minimumRooms')}</span>
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            className={`chevron ${expandedSections.rooms ? 'expanded' : ''}`}
            aria-hidden="true"
          >
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>
        {expandedSections.rooms && (
          <div className="section-content rooms-grid">
            {[undefined, 1, 2, 3, 4, 5].map((rooms) => (
              <button
                key={rooms ?? 'any'}
                type="button"
                className={`room-button ${filters.roomsMin === rooms ? 'active' : ''}`}
                onClick={() => handleRoomsChange(rooms)}
              >
                {rooms === undefined ? t('anyRooms') : t('roomsPlus', { count: rooms })}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Mobile Apply Button */}
      {isMobile && (
        <div className="mobile-footer">
          <button type="button" className="apply-button" onClick={onClose}>
            {t('showResults')}
          </button>
        </div>
      )}

      <style jsx>{`
        .filters {
          background: var(--ppt-bg-surface);
          border-radius: 12px;
          padding: 20px;
          height: fit-content;
        }

        .filters.mobile {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          z-index: 100;
          border-radius: 0;
          overflow-y: auto;
          padding-bottom: 80px;
        }

        .filters-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 20px;
          padding-bottom: 12px;
          border-bottom: 1px solid var(--ppt-border-default);
        }

        .filters-title {
          font-size: 1.125rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
          margin: 0;
        }

        .filters-actions {
          display: flex;
          align-items: center;
          gap: 12px;
        }

        .clear-button {
          padding: 4px 8px;
          font-size: 13px;
          color: var(--ppt-color-primary);
          background: transparent;
          border: none;
          cursor: pointer;
        }

        .clear-button:hover {
          text-decoration: underline;
        }

        .close-button {
          padding: 4px;
          background: transparent;
          border: none;
          cursor: pointer;
          color: var(--ppt-fg-muted);
        }

        .filter-section {
          border-bottom: 1px solid var(--ppt-bg-subtle);
          padding-bottom: 16px;
          margin-bottom: 16px;
        }

        .filter-section:last-of-type {
          border-bottom: none;
          margin-bottom: 0;
        }

        .section-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          width: 100%;
          padding: 0;
          background: transparent;
          border: none;
          cursor: pointer;
          font-size: 14px;
          font-weight: 600;
          color: var(--ppt-fg-secondary);
          margin-bottom: 12px;
        }

        .chevron {
          transition: transform 0.2s;
        }

        .chevron.expanded {
          transform: rotate(180deg);
        }

        .section-content {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }

        .checkbox-label,
        .radio-label {
          display: flex;
          align-items: center;
          gap: 8px;
          font-size: 14px;
          color: var(--ppt-neutral-600);
          cursor: pointer;
        }

        .checkbox,
        .radio {
          width: 16px;
          height: 16px;
          accent-color: var(--ppt-color-primary);
        }

        .rooms-grid {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 8px;
        }

        .room-button {
          padding: 8px;
          border: 1px solid var(--ppt-border-default);
          background: var(--ppt-bg-surface);
          border-radius: 6px;
          font-size: 14px;
          color: var(--ppt-fg-secondary);
          cursor: pointer;
          transition: all 0.2s;
        }

        .room-button:hover {
          border-color: var(--ppt-color-primary);
        }

        .room-button.active {
          background: var(--ppt-color-primary);
          border-color: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
        }

        .mobile-footer {
          position: fixed;
          bottom: 0;
          left: 0;
          right: 0;
          padding: 16px;
          background: var(--ppt-bg-surface);
          border-top: 1px solid var(--ppt-border-default);
        }

        .apply-button {
          width: 100%;
          padding: 12px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
          border-radius: 8px;
          font-size: 16px;
          font-weight: 600;
          cursor: pointer;
        }

        .apply-button:hover {
          background: var(--ppt-color-primary-hover);
        }
      `}</style>
    </aside>
  );
}
