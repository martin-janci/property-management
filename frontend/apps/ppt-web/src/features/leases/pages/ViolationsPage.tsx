/**
 * ViolationsPage - List all lease violations with filtering.
 * UC-34: Lease Violations Tracking
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ViolationCard } from '../components/ViolationCard';
import type { LeaseSummary, ViolationSeverity, ViolationStatus, ViolationSummary } from '../types';

interface ViolationsPageProps {
  violations: ViolationSummary[];
  leases: LeaseSummary[];
  isLoading?: boolean;
  isManager?: boolean;
  onViewViolation: (id: string) => void;
  onResolveViolation: (id: string) => void;
  onDisputeViolation: (id: string) => void;
  onCreateViolation: () => void;
  /** Omit to hide the export button (no export endpoint). */
  onExportReport?: () => void;
}

const violationStatuses: ViolationStatus[] = [
  'open',
  'resolved',
  'disputed',
  'escalated',
  'dismissed',
];
const violationSeverities: ViolationSeverity[] = ['minor', 'moderate', 'severe'];

export function ViolationsPage({
  violations,
  leases,
  isLoading,
  isManager,
  onViewViolation,
  onResolveViolation,
  onDisputeViolation,
  onCreateViolation,
  onExportReport,
}: ViolationsPageProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedLease, setSelectedLease] = useState<string>('');
  const [selectedStatus, setSelectedStatus] = useState<string>('');
  const [selectedSeverity, setSelectedSeverity] = useState<string>('');

  // Filter violations
  const filteredViolations = violations.filter((violation) => {
    const matchesSearch =
      !searchQuery ||
      violation.tenantName.toLowerCase().includes(searchQuery.toLowerCase()) ||
      violation.unitNumber.toLowerCase().includes(searchQuery.toLowerCase()) ||
      violation.buildingName.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesLease = !selectedLease || violation.leaseId === selectedLease;
    const matchesStatus = !selectedStatus || violation.status === selectedStatus;
    const matchesSeverity = !selectedSeverity || violation.severity === selectedSeverity;
    return matchesSearch && matchesLease && matchesStatus && matchesSeverity;
  });

  const clearFilters = () => {
    setSearchQuery('');
    setSelectedLease('');
    setSelectedStatus('');
    setSelectedSeverity('');
  };

  const hasFilters = searchQuery || selectedLease || selectedStatus || selectedSeverity;

  // Statistics
  const openCount = violations.filter((v) => v.status === 'open').length;
  const resolvedCount = violations.filter((v) => v.status === 'resolved').length;
  const disputedCount = violations.filter((v) => v.status === 'disputed').length;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{t('leases.violations.title')}</h1>
          <p className="text-gray-600 mt-1">{t('leases.violations.subtitle')}</p>
        </div>
        <div className="flex gap-2">
          {onExportReport && (
            <button
              type="button"
              onClick={onExportReport}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
            >
              <span className="flex items-center gap-2">
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                  />
                </svg>
                {t('leases.violations.exportReport')}
              </span>
            </button>
          )}
          {isManager && (
            <button
              type="button"
              onClick={onCreateViolation}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md"
            >
              <span className="flex items-center gap-2">
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 4v16m8-8H4"
                  />
                </svg>
                {t('leases.violations.createNew')}
              </span>
            </button>
          )}
        </div>
      </div>

      {/* Statistics */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
        <div className="bg-white rounded-lg shadow p-4">
          <p className="text-sm text-gray-500">{t('leases.violations.stats.total')}</p>
          <p className="text-2xl font-bold text-gray-900">{violations.length}</p>
        </div>
        <div className="bg-white rounded-lg shadow p-4">
          <p className="text-sm text-gray-500">{t('leases.violations.stats.open')}</p>
          <p className="text-2xl font-bold text-red-600">{openCount}</p>
        </div>
        <div className="bg-white rounded-lg shadow p-4">
          <p className="text-sm text-gray-500">{t('leases.violations.stats.resolved')}</p>
          <p className="text-2xl font-bold text-green-600">{resolvedCount}</p>
        </div>
        <div className="bg-white rounded-lg shadow p-4">
          <p className="text-sm text-gray-500">{t('leases.violations.stats.disputed')}</p>
          <p className="text-2xl font-bold text-yellow-600">{disputedCount}</p>
        </div>
      </div>

      {/* Filters */}
      <div className="bg-white rounded-lg shadow p-4 mb-6">
        <div className="grid grid-cols-1 md:grid-cols-5 gap-4">
          {/* Search */}
          <div className="md:col-span-2">
            <label htmlFor="search" className="sr-only">
              {t('common.search')}
            </label>
            <div className="relative">
              <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                <svg
                  className="h-5 w-5 text-gray-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                  />
                </svg>
              </div>
              <input
                id="search"
                type="text"
                placeholder={t('leases.violations.searchPlaceholder')}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="block w-full pl-10 pr-3 py-2 border border-gray-300 rounded-md leading-5 bg-white placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
              />
            </div>
          </div>

          {/* Lease Filter */}
          <div>
            <label htmlFor="lease-filter" className="sr-only">
              {t('leases.violations.filterByLease')}
            </label>
            <select
              id="lease-filter"
              value={selectedLease}
              onChange={(e) => setSelectedLease(e.target.value)}
              className="block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
            >
              <option value="">{t('leases.violations.allLeases')}</option>
              {leases.map((lease) => (
                <option key={lease.id} value={lease.id}>
                  {lease.buildingName} - {lease.unitNumber}
                </option>
              ))}
            </select>
          </div>

          {/* Status Filter */}
          <div>
            <label htmlFor="status-filter" className="sr-only">
              {t('leases.violations.filterByStatus')}
            </label>
            <select
              id="status-filter"
              value={selectedStatus}
              onChange={(e) => setSelectedStatus(e.target.value)}
              className="block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
            >
              <option value="">{t('leases.violations.allStatuses')}</option>
              {violationStatuses.map((status) => (
                <option key={status} value={status}>
                  {t(`leases.violations.status.${status}`)}
                </option>
              ))}
            </select>
          </div>

          {/* Severity Filter */}
          <div>
            <label htmlFor="severity-filter" className="sr-only">
              {t('leases.violations.filterBySeverity')}
            </label>
            <select
              id="severity-filter"
              value={selectedSeverity}
              onChange={(e) => setSelectedSeverity(e.target.value)}
              className="block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
            >
              <option value="">{t('leases.violations.allSeverities')}</option>
              {violationSeverities.map((severity) => (
                <option key={severity} value={severity}>
                  {t(`leases.violations.severity.${severity}`)}
                </option>
              ))}
            </select>
          </div>
        </div>

        {hasFilters && (
          <div className="mt-3 flex items-center justify-between">
            <p className="text-sm text-gray-600">
              {t('leases.violations.showing', {
                count: filteredViolations.length,
                total: violations.length,
              })}
            </p>
            <button
              type="button"
              onClick={clearFilters}
              className="text-sm text-blue-600 hover:text-blue-800"
            >
              {t('leases.violations.clearFilters')}
            </button>
          </div>
        )}
      </div>

      {/* Violations List */}
      {filteredViolations.length === 0 ? (
        <div className="bg-white rounded-lg shadow p-8 text-center">
          <svg
            className="mx-auto h-12 w-12 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          <h3 className="mt-2 text-sm font-medium text-gray-900">
            {t('leases.violations.noViolations')}
          </h3>
          <p className="mt-1 text-sm text-gray-500">
            {t('leases.violations.noViolationsDescription')}
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {filteredViolations.map((violation) => (
            <ViolationCard
              key={violation.id}
              violation={violation}
              onView={onViewViolation}
              onResolve={onResolveViolation}
              onDispute={onDisputeViolation}
            />
          ))}
        </div>
      )}
    </div>
  );
}
