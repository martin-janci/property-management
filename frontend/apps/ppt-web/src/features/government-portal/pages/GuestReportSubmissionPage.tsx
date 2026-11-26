/**
 * GuestReportSubmissionPage - Guest report submission to government portals.
 * Epic 41: Government Portal UI (Story 41.1)
 *
 * Allows property managers to submit guest reports to ÚHÚL/Police portals
 * with preview modal, country selector, and submission progress indicator.
 */

import type {
  CreateRegulatorySubmissionRequest,
  GovernmentPortalConnection,
  RegulatoryReportTemplate,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  buildSubmissionRequest,
  filterPortalConnections,
  filterReportTemplates,
  formatSubmissionDate,
  type GuestRecord,
  type SubmissionCountry,
} from './guestReportSubmission';

export type { GuestRecord } from './guestReportSubmission';

/** Back-arrow (chevron-left) icon used by the header and preview actions. */
function ArrowLeftIcon() {
  return (
    <svg
      className="h-4 w-4"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      aria-hidden="true"
    >
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
    </svg>
  );
}

/** Check-mark icon used by completed steps and the completion screen. */
function CheckIcon({ className = 'h-4 w-4' }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      aria-hidden="true"
    >
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
    </svg>
  );
}

interface GuestReportSubmissionPageProps {
  /** Available portal connections */
  connections: GovernmentPortalConnection[];
  /** Available report templates */
  templates: RegulatoryReportTemplate[];
  /** Guest records to submit */
  guests: GuestRecord[];
  /** Loading state */
  isLoading?: boolean;
  /** Submission in progress */
  isSubmitting?: boolean;
  /** Current submission progress (0-100) */
  submissionProgress?: number;
  /** Submission error */
  submissionError?: string | null;
  /** Handler for creating submission */
  onSubmit: (data: CreateRegulatorySubmissionRequest) => Promise<void>;
  /** Handler for navigating back */
  onBack: () => void;
}

type SubmissionStep = 'select' | 'preview' | 'submitting' | 'complete';

export function GuestReportSubmissionPage({
  connections,
  templates,
  guests,
  isLoading = false,
  isSubmitting = false,
  submissionProgress = 0,
  submissionError,
  onSubmit,
  onBack,
}: GuestReportSubmissionPageProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<SubmissionStep>('select');
  const [selectedCountry, setSelectedCountry] = useState<SubmissionCountry>('SK');
  const [selectedConnectionId, setSelectedConnectionId] = useState<string>('');
  const [selectedTemplateId, setSelectedTemplateId] = useState<string>('');
  const [selectedGuests, setSelectedGuests] = useState<string[]>(guests.map((g) => g.id));
  const [localError, setLocalError] = useState<string | null>(null);

  // Filter connections and templates by country
  const filteredConnections = filterPortalConnections(connections, selectedCountry);
  const filteredTemplates = filterReportTemplates(templates, selectedCountry);

  const selectedConnection = connections.find((c) => c.id === selectedConnectionId);
  const selectedTemplate = templates.find((tpl) => tpl.id === selectedTemplateId);
  const selectedGuestRecords = guests.filter((g) => selectedGuests.includes(g.id));

  const countryName =
    selectedCountry === 'SK'
      ? t('governmentPortal.guestReport.country.sk', { defaultValue: 'Slovakia' })
      : t('governmentPortal.guestReport.country.cz', { defaultValue: 'Czech Republic' });

  const handleSelectAllGuests = () => {
    if (selectedGuests.length === guests.length) {
      setSelectedGuests([]);
    } else {
      setSelectedGuests(guests.map((g) => g.id));
    }
  };

  const handleToggleGuest = (guestId: string) => {
    setSelectedGuests((prev) =>
      prev.includes(guestId) ? prev.filter((id) => id !== guestId) : [...prev, guestId]
    );
  };

  const handleProceedToPreview = () => {
    setLocalError(null);

    if (!selectedConnectionId) {
      setLocalError(
        t('governmentPortal.guestReport.errors.selectConnection', {
          defaultValue: 'Please select a portal connection',
        })
      );
      return;
    }
    if (!selectedTemplateId) {
      setLocalError(
        t('governmentPortal.guestReport.errors.selectTemplate', {
          defaultValue: 'Please select a report template',
        })
      );
      return;
    }
    if (selectedGuests.length === 0) {
      setLocalError(
        t('governmentPortal.guestReport.errors.selectGuest', {
          defaultValue: 'Please select at least one guest',
        })
      );
      return;
    }

    setStep('preview');
  };

  const handleSubmit = async () => {
    setLocalError(null);
    setStep('submitting');

    try {
      await onSubmit(
        buildSubmissionRequest({
          portalConnectionId: selectedConnectionId,
          templateId: selectedTemplateId,
          country: selectedCountry,
          guests: selectedGuestRecords,
        })
      );

      setStep('complete');
    } catch (err) {
      setLocalError(
        err instanceof Error
          ? err.message
          : t('governmentPortal.guestReport.errors.submissionFailed', {
              defaultValue: 'Submission failed',
            })
      );
      setStep('preview');
    }
  };

  // Static placeholder IDs for loading skeleton
  const loadingSkeletonIds = ['skeleton-row-1', 'skeleton-row-2', 'skeleton-row-3'];

  if (isLoading) {
    return (
      <div className="max-w-4xl mx-auto px-4 py-8">
        <div className="animate-pulse">
          <div className="h-8 w-64 bg-gray-200 rounded mb-6" />
          <div className="space-y-4">
            {loadingSkeletonIds.map((id) => (
              <div key={id} className="h-16 bg-gray-100 rounded-lg" />
            ))}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-8">
        <button
          type="button"
          onClick={onBack}
          className="inline-flex items-center gap-2 text-gray-600 hover:text-gray-900 mb-4"
        >
          <ArrowLeftIcon />
          {t('governmentPortal.guestReport.backToDashboard', { defaultValue: 'Back to Dashboard' })}
        </button>
        <h1 className="text-2xl font-bold text-gray-900">
          {t('governmentPortal.guestReport.title', { defaultValue: 'Submit Guest Report' })}
        </h1>
        <p className="mt-1 text-gray-500">
          {t('governmentPortal.guestReport.subtitle', {
            defaultValue: 'Submit guest registration data to government portals (ÚHÚL/Police)',
          })}
        </p>
      </div>

      {/* Progress Steps */}
      <div className="mb-8">
        <div className="flex items-center justify-between">
          {[
            t('governmentPortal.guestReport.steps.selectOptions', {
              defaultValue: 'Select Options',
            }),
            t('governmentPortal.guestReport.steps.previewData', { defaultValue: 'Preview Data' }),
            t('governmentPortal.guestReport.steps.submit', { defaultValue: 'Submit' }),
          ].map((label, index) => {
            const stepNumber = index + 1;
            const isActive =
              (step === 'select' && stepNumber === 1) ||
              (step === 'preview' && stepNumber === 2) ||
              ((step === 'submitting' || step === 'complete') && stepNumber === 3);
            const isCompleted =
              (step === 'preview' && stepNumber === 1) ||
              ((step === 'submitting' || step === 'complete') && stepNumber <= 2);

            return (
              <div key={label} className="flex items-center">
                <div
                  className={`flex items-center justify-center w-8 h-8 rounded-full text-sm font-medium ${
                    isCompleted
                      ? 'bg-green-600 text-white'
                      : isActive
                        ? 'bg-blue-600 text-white'
                        : 'bg-gray-200 text-gray-500'
                  }`}
                >
                  {isCompleted ? <CheckIcon /> : stepNumber}
                </div>
                <span
                  className={`ml-2 text-sm ${
                    isActive || isCompleted ? 'text-gray-900 font-medium' : 'text-gray-500'
                  }`}
                >
                  {label}
                </span>
                {index < 2 && (
                  <div
                    className={`w-16 h-0.5 mx-4 ${isCompleted ? 'bg-green-600' : 'bg-gray-200'}`}
                  />
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Error Display */}
      {(localError || submissionError) && (
        <div className="mb-6 rounded-lg bg-red-50 border border-red-200 p-4">
          <div className="flex items-center gap-3">
            <svg
              className="h-5 w-5 text-red-500"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <p className="text-red-700">{localError || submissionError}</p>
          </div>
        </div>
      )}

      {/* Step 1: Select Options */}
      {step === 'select' && (
        <div className="space-y-6">
          {/* Country Selector */}
          <div className="bg-white rounded-lg border border-gray-200 p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              {t('governmentPortal.guestReport.select.country', { defaultValue: 'Select Country' })}
            </h2>
            <div className="flex gap-4">
              {(['SK', 'CZ'] as const).map((country) => (
                <button
                  key={country}
                  type="button"
                  onClick={() => {
                    setSelectedCountry(country);
                    setSelectedConnectionId('');
                    setSelectedTemplateId('');
                  }}
                  className={`flex-1 p-4 rounded-lg border-2 transition-colors ${
                    selectedCountry === country
                      ? 'border-blue-600 bg-blue-50'
                      : 'border-gray-200 hover:border-gray-300'
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <span className="text-2xl">{country === 'SK' ? '🇸🇰' : '🇨🇿'}</span>
                    <div className="text-left">
                      <p className="font-medium text-gray-900">
                        {country === 'SK'
                          ? t('governmentPortal.guestReport.country.sk', {
                              defaultValue: 'Slovakia',
                            })
                          : t('governmentPortal.guestReport.country.cz', {
                              defaultValue: 'Czech Republic',
                            })}
                      </p>
                      <p className="text-sm text-gray-500">
                        {country === 'SK'
                          ? t('governmentPortal.guestReport.country.skDescription', {
                              defaultValue: 'ÚHÚL / Police Registry',
                            })
                          : t('governmentPortal.guestReport.country.czDescription', {
                              defaultValue: 'Cizinecká policie',
                            })}
                      </p>
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </div>

          {/* Portal Connection */}
          <div className="bg-white rounded-lg border border-gray-200 p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              {t('governmentPortal.guestReport.select.portalConnection', {
                defaultValue: 'Portal Connection',
              })}
            </h2>
            {filteredConnections.length === 0 ? (
              <p className="text-gray-500">
                {t('governmentPortal.guestReport.select.noConnections', {
                  defaultValue:
                    'No active portal connections found for {{country}}. Please configure a connection first.',
                  country: countryName,
                })}
              </p>
            ) : (
              <div className="space-y-3">
                {filteredConnections.map((connection) => (
                  <label
                    key={connection.id}
                    className={`flex items-center p-4 rounded-lg border cursor-pointer transition-colors ${
                      selectedConnectionId === connection.id
                        ? 'border-blue-600 bg-blue-50'
                        : 'border-gray-200 hover:border-gray-300'
                    }`}
                  >
                    <input
                      type="radio"
                      name="connection"
                      value={connection.id}
                      checked={selectedConnectionId === connection.id}
                      onChange={(e) => setSelectedConnectionId(e.target.value)}
                      className="h-4 w-4 text-blue-600 border-gray-300 focus:ring-blue-500"
                    />
                    <div className="ml-3">
                      <p className="font-medium text-gray-900">{connection.portalName}</p>
                      <p className="text-sm text-gray-500">
                        {connection.portalType.replace('_', ' ')} -{' '}
                        {connection.portalCode ||
                          t('governmentPortal.guestReport.select.noCode', {
                            defaultValue: 'No code',
                          })}
                        {connection.testMode && (
                          <span className="ml-2 text-xs bg-yellow-100 text-yellow-800 px-2 py-0.5 rounded">
                            {t('governmentPortal.guestReport.testMode', {
                              defaultValue: 'Test Mode',
                            })}
                          </span>
                        )}
                      </p>
                    </div>
                  </label>
                ))}
              </div>
            )}
          </div>

          {/* Report Template */}
          <div className="bg-white rounded-lg border border-gray-200 p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              {t('governmentPortal.guestReport.select.reportTemplate', {
                defaultValue: 'Report Template',
              })}
            </h2>
            {filteredTemplates.length === 0 ? (
              <p className="text-gray-500">
                {t('governmentPortal.guestReport.select.noTemplates', {
                  defaultValue: 'No report templates found for {{country}}.',
                  country: countryName,
                })}
              </p>
            ) : (
              <select
                value={selectedTemplateId}
                onChange={(e) => setSelectedTemplateId(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="">
                  {t('governmentPortal.guestReport.select.templatePlaceholder', {
                    defaultValue: 'Select a template...',
                  })}
                </option>
                {filteredTemplates.map((template) => (
                  <option key={template.id} value={template.id}>
                    {template.templateName} ({template.templateCode})
                  </option>
                ))}
              </select>
            )}
          </div>

          {/* Guest Selection */}
          <div className="bg-white rounded-lg border border-gray-200 p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">
                {t('governmentPortal.guestReport.select.guestsHeading', {
                  defaultValue: 'Select Guests ({{selected}} of {{total}})',
                  selected: selectedGuests.length,
                  total: guests.length,
                })}
              </h2>
              <label className="flex items-center gap-2 text-sm text-gray-600">
                <input
                  type="checkbox"
                  checked={selectedGuests.length === guests.length && guests.length > 0}
                  onChange={handleSelectAllGuests}
                  className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                />
                {t('governmentPortal.guestReport.select.selectAll', { defaultValue: 'Select all' })}
              </label>
            </div>

            {guests.length === 0 ? (
              <p className="text-gray-500">
                {t('governmentPortal.guestReport.select.noGuests', {
                  defaultValue: 'No guests registered for submission.',
                })}
              </p>
            ) : (
              <div className="max-h-64 overflow-y-auto space-y-2">
                {guests.map((guest) => (
                  <label
                    key={guest.id}
                    className={`flex items-center p-3 rounded-lg border cursor-pointer transition-colors ${
                      selectedGuests.includes(guest.id)
                        ? 'border-blue-600 bg-blue-50'
                        : 'border-gray-200 hover:border-gray-300'
                    }`}
                  >
                    <input
                      type="checkbox"
                      checked={selectedGuests.includes(guest.id)}
                      onChange={() => handleToggleGuest(guest.id)}
                      className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                    />
                    <div className="ml-3 flex-1">
                      <p className="font-medium text-gray-900">
                        {guest.firstName} {guest.lastName}
                      </p>
                      <p className="text-sm text-gray-500">
                        {guest.nationality} | {guest.documentType}: {guest.documentNumber} |{' '}
                        Check-in: {formatSubmissionDate(guest.checkInDate)}
                      </p>
                    </div>
                    <div className="text-right text-sm text-gray-500">
                      <p>{guest.unitNumber}</p>
                      <p>{guest.buildingName}</p>
                    </div>
                  </label>
                ))}
              </div>
            )}
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-3">
            <button
              type="button"
              onClick={onBack}
              className="px-4 py-2 text-gray-700 hover:text-gray-900"
            >
              {t('governmentPortal.guestReport.actions.cancel', { defaultValue: 'Cancel' })}
            </button>
            <button
              type="button"
              onClick={handleProceedToPreview}
              disabled={filteredConnections.length === 0 || filteredTemplates.length === 0}
              className="inline-flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {t('governmentPortal.guestReport.actions.previewSubmission', {
                defaultValue: 'Preview Submission',
              })}
              <svg
                className="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 5l7 7-7 7"
                />
              </svg>
            </button>
          </div>
        </div>
      )}

      {/* Step 2: Preview */}
      {step === 'preview' && (
        <div className="space-y-6">
          {/* Preview Modal/Section */}
          <div className="bg-white rounded-lg border border-gray-200 overflow-hidden">
            <div className="bg-gray-50 px-6 py-4 border-b border-gray-200">
              <h2 className="text-lg font-semibold text-gray-900">
                {t('governmentPortal.guestReport.preview.title', {
                  defaultValue: 'Submission Preview',
                })}
              </h2>
              <p className="text-sm text-gray-500">
                {t('governmentPortal.guestReport.preview.subtitle', {
                  defaultValue: 'Review the data before submitting to the portal',
                })}
              </p>
            </div>

            <div className="p-6 space-y-6">
              {/* Destination Info */}
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <p className="text-sm font-medium text-gray-500">
                    {t('governmentPortal.guestReport.preview.destinationPortal', {
                      defaultValue: 'Destination Portal',
                    })}
                  </p>
                  <p className="text-gray-900">
                    {selectedConnection?.portalName ||
                      t('governmentPortal.guestReport.preview.unknown', {
                        defaultValue: 'Unknown',
                      })}
                    {selectedConnection?.testMode && (
                      <span className="ml-2 text-xs bg-yellow-100 text-yellow-800 px-2 py-0.5 rounded">
                        {t('governmentPortal.guestReport.testMode', { defaultValue: 'Test Mode' })}
                      </span>
                    )}
                  </p>
                </div>
                <div>
                  <p className="text-sm font-medium text-gray-500">
                    {t('governmentPortal.guestReport.preview.reportTemplate', {
                      defaultValue: 'Report Template',
                    })}
                  </p>
                  <p className="text-gray-900">
                    {selectedTemplate?.templateName ||
                      t('governmentPortal.guestReport.preview.unknown', {
                        defaultValue: 'Unknown',
                      })}
                  </p>
                </div>
                <div>
                  <p className="text-sm font-medium text-gray-500">
                    {t('governmentPortal.guestReport.preview.country', { defaultValue: 'Country' })}
                  </p>
                  <p className="text-gray-900">
                    {selectedCountry === 'SK' ? '🇸🇰 ' : '🇨🇿 '}
                    {countryName}
                  </p>
                </div>
                <div>
                  <p className="text-sm font-medium text-gray-500">
                    {t('governmentPortal.guestReport.preview.guestsToSubmit', {
                      defaultValue: 'Guests to Submit',
                    })}
                  </p>
                  <p className="text-gray-900">
                    {t('governmentPortal.guestReport.preview.guestCount', {
                      defaultValue: '{{total}} guest(s)',
                      total: selectedGuestRecords.length,
                    })}
                  </p>
                </div>
              </div>

              {/* Guest Data Preview */}
              <div>
                <h3 className="text-sm font-medium text-gray-900 mb-3">
                  {t('governmentPortal.guestReport.preview.guestData', {
                    defaultValue: 'Guest Data',
                  })}
                </h3>
                <div className="overflow-x-auto">
                  <table className="min-w-full divide-y divide-gray-200">
                    <thead className="bg-gray-50">
                      <tr>
                        <th className="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                          {t('governmentPortal.guestReport.preview.table.name', {
                            defaultValue: 'Name',
                          })}
                        </th>
                        <th className="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                          {t('governmentPortal.guestReport.preview.table.dob', {
                            defaultValue: 'DOB',
                          })}
                        </th>
                        <th className="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                          {t('governmentPortal.guestReport.preview.table.nationality', {
                            defaultValue: 'Nationality',
                          })}
                        </th>
                        <th className="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                          {t('governmentPortal.guestReport.preview.table.document', {
                            defaultValue: 'Document',
                          })}
                        </th>
                        <th className="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                          {t('governmentPortal.guestReport.preview.table.checkIn', {
                            defaultValue: 'Check-in',
                          })}
                        </th>
                        <th className="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                          {t('governmentPortal.guestReport.preview.table.unit', {
                            defaultValue: 'Unit',
                          })}
                        </th>
                      </tr>
                    </thead>
                    <tbody className="bg-white divide-y divide-gray-200">
                      {selectedGuestRecords.map((guest) => (
                        <tr key={guest.id}>
                          <td className="px-3 py-2 text-sm text-gray-900">
                            {guest.firstName} {guest.lastName}
                          </td>
                          <td className="px-3 py-2 text-sm text-gray-500">
                            {formatSubmissionDate(guest.dateOfBirth)}
                          </td>
                          <td className="px-3 py-2 text-sm text-gray-500">{guest.nationality}</td>
                          <td className="px-3 py-2 text-sm text-gray-500">
                            {guest.documentType}: {guest.documentNumber}
                          </td>
                          <td className="px-3 py-2 text-sm text-gray-500">
                            {formatSubmissionDate(guest.checkInDate)}
                          </td>
                          <td className="px-3 py-2 text-sm text-gray-500">{guest.unitNumber}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          </div>

          {/* Actions */}
          <div className="flex justify-between">
            <button
              type="button"
              onClick={() => setStep('select')}
              className="inline-flex items-center gap-2 px-4 py-2 text-gray-700 hover:text-gray-900"
            >
              <ArrowLeftIcon />
              {t('governmentPortal.guestReport.actions.backToSelection', {
                defaultValue: 'Back to Selection',
              })}
            </button>
            <button
              type="button"
              onClick={handleSubmit}
              className="inline-flex items-center gap-2 px-6 py-2 bg-green-600 text-white rounded-md hover:bg-green-700"
            >
              <svg
                className="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              {t('governmentPortal.guestReport.actions.submitToPortal', {
                defaultValue: 'Submit to Portal',
              })}
            </button>
          </div>
        </div>
      )}

      {/* Step 3: Submitting */}
      {step === 'submitting' && (
        <div className="bg-white rounded-lg border border-gray-200 p-12 text-center">
          <div className="mx-auto w-16 h-16 rounded-full bg-blue-100 flex items-center justify-center mb-6">
            <svg
              className="h-8 w-8 text-blue-600 animate-spin"
              fill="none"
              viewBox="0 0 24 24"
              aria-hidden="true"
            >
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
              />
            </svg>
          </div>
          <h2 className="text-xl font-semibold text-gray-900 mb-2">
            {t('governmentPortal.guestReport.submitting.title', {
              defaultValue: 'Submitting to Portal...',
            })}
          </h2>
          <p className="text-gray-500 mb-6">
            {t('governmentPortal.guestReport.submitting.message', {
              defaultValue: 'Please wait while your guest report is being submitted.',
            })}
          </p>

          {/* Progress Bar */}
          <div className="max-w-md mx-auto">
            <div className="h-2 bg-gray-200 rounded-full overflow-hidden">
              <div
                className="h-full bg-blue-600 transition-all duration-500 ease-out"
                style={{ width: `${isSubmitting ? submissionProgress : 50}%` }}
              />
            </div>
            <p className="mt-2 text-sm text-gray-500">
              {isSubmitting
                ? t('governmentPortal.guestReport.submitting.percentComplete', {
                    defaultValue: '{{progress}}% complete',
                    progress: submissionProgress,
                  })
                : t('governmentPortal.guestReport.submitting.processing', {
                    defaultValue: 'Processing...',
                  })}
            </p>
          </div>
        </div>
      )}

      {/* Step 4: Complete */}
      {step === 'complete' && (
        <div className="bg-white rounded-lg border border-gray-200 p-12 text-center">
          <div className="mx-auto w-16 h-16 rounded-full bg-green-100 flex items-center justify-center mb-6">
            <CheckIcon className="h-8 w-8 text-green-600" />
          </div>
          <h2 className="text-xl font-semibold text-gray-900 mb-2">
            {t('governmentPortal.guestReport.complete.title', {
              defaultValue: 'Submission Complete!',
            })}
          </h2>
          <p className="text-gray-500 mb-6">
            {t('governmentPortal.guestReport.complete.message', {
              defaultValue: 'Your guest report has been successfully submitted to {{portal}}.',
              portal:
                selectedConnection?.portalName ||
                t('governmentPortal.guestReport.complete.portal', { defaultValue: 'Portal' }),
            })}
          </p>

          <div className="bg-gray-50 rounded-lg p-4 max-w-md mx-auto mb-6">
            <div className="grid grid-cols-2 gap-4 text-left">
              <div>
                <p className="text-xs font-medium text-gray-500">
                  {t('governmentPortal.guestReport.complete.guestsSubmitted', {
                    defaultValue: 'Guests Submitted',
                  })}
                </p>
                <p className="text-lg font-semibold text-gray-900">{selectedGuestRecords.length}</p>
              </div>
              <div>
                <p className="text-xs font-medium text-gray-500">
                  {t('governmentPortal.guestReport.complete.portal', { defaultValue: 'Portal' })}
                </p>
                <p className="text-sm text-gray-900">{selectedConnection?.portalName}</p>
              </div>
            </div>
          </div>

          <button
            type="button"
            onClick={onBack}
            className="px-6 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
          >
            {t('governmentPortal.guestReport.backToDashboard', {
              defaultValue: 'Back to Dashboard',
            })}
          </button>
        </div>
      )}
    </div>
  );
}
