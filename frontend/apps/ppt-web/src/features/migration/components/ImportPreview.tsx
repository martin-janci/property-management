/**
 * Import Preview Component (Story 66.4).
 *
 * Displays validation results and preview before committing an import.
 */

import type * as React from 'react';
import { useState } from 'react';

export type ValidationSeverity = 'error' | 'warning' | 'info';

export interface ValidationIssue {
  rowNumber?: number;
  column?: string;
  severity: ValidationSeverity;
  code: string;
  message: string;
  originalValue?: string;
  suggestedValue?: string;
}

export interface ColumnMappingStatus {
  sourceColumn: string;
  targetField?: string;
  isMapped: boolean;
  isRequired: boolean;
  sampleValues: string[];
}

export interface RecordTypeCounts {
  newRecords: number;
  updates: number;
  skipped: number;
}

export interface ImportPreviewData {
  jobId: string;
  isValid: boolean;
  totalRows: number;
  importableRows: number;
  errorRows: number;
  warningRows: number;
  recordCounts: RecordTypeCounts;
  issues: ValidationIssue[];
  totalIssueCount: number;
  sampleRecords: Record<string, unknown>[];
  columnMapping: ColumnMappingStatus[];
}

interface ImportPreviewProps {
  preview: ImportPreviewData;
  onApprove: (acknowledgeWarnings: boolean) => void;
  onCancel: () => void;
  isLoading?: boolean;
}

const SEVERITY_COLORS: Record<ValidationSeverity, string> = {
  error: 'bg-red-100 text-red-800 border-red-200',
  warning: 'bg-yellow-100 text-yellow-800 border-yellow-200',
  info: 'bg-blue-100 text-blue-800 border-blue-200',
};

const SEVERITY_ICONS: Record<ValidationSeverity, React.JSX.Element> = {
  error: (
    <svg className="h-4 w-4 text-red-500" fill="currentColor" viewBox="0 0 20 20">
      <path
        fillRule="evenodd"
        d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
        clipRule="evenodd"
      />
    </svg>
  ),
  warning: (
    <svg className="h-4 w-4 text-yellow-500" fill="currentColor" viewBox="0 0 20 20">
      <path
        fillRule="evenodd"
        d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
        clipRule="evenodd"
      />
    </svg>
  ),
  info: (
    <svg className="h-4 w-4 text-blue-500" fill="currentColor" viewBox="0 0 20 20">
      <path
        fillRule="evenodd"
        d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
        clipRule="evenodd"
      />
    </svg>
  ),
};

export function ImportPreview({
  preview,
  onApprove,
  onCancel,
  isLoading = false,
}: ImportPreviewProps) {
  const [activeTab, setActiveTab] = useState<'summary' | 'issues' | 'mapping' | 'sample'>(
    'summary'
  );
  const [acknowledgeWarnings, setAcknowledgeWarnings] = useState(false);

  const errorCount = preview.issues.filter((i) => i.severity === 'error').length;
  const warningCount = preview.issues.filter((i) => i.severity === 'warning').length;
  const infoCount = preview.issues.filter((i) => i.severity === 'info').length;

  const canApprove = preview.isValid && (warningCount === 0 || acknowledgeWarnings);

  return (
    <div className="space-y-6">
      {/* Header with Status */}
      <div className="flex items-center justify-between border-b border-gray-200 pb-4">
        <div>
          <h2 className="text-lg font-medium text-gray-900">Import Preview</h2>
          <p className="mt-1 text-sm text-gray-500">
            Review validation results before importing {preview.totalRows.toLocaleString()} rows
          </p>
        </div>
        <div
          className={`inline-flex items-center rounded-full px-3 py-1 text-sm font-medium ${
            preview.isValid ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
          }`}
        >
          {preview.isValid ? 'Validation Passed' : 'Validation Failed'}
        </div>
      </div>

      {/* Summary Cards */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <div className="rounded-lg bg-gray-50 p-4">
          <p className="text-sm text-gray-500">Total Rows</p>
          <p className="mt-1 text-2xl font-semibold text-gray-900">
            {preview.totalRows.toLocaleString()}
          </p>
        </div>
        <div className="rounded-lg bg-green-50 p-4">
          <p className="text-sm text-green-600">Importable</p>
          <p className="mt-1 text-2xl font-semibold text-green-700">
            {preview.importableRows.toLocaleString()}
          </p>
        </div>
        <div className="rounded-lg bg-red-50 p-4">
          <p className="text-sm text-red-600">With Errors</p>
          <p className="mt-1 text-2xl font-semibold text-red-700">
            {preview.errorRows.toLocaleString()}
          </p>
        </div>
        <div className="rounded-lg bg-yellow-50 p-4">
          <p className="text-sm text-yellow-600">With Warnings</p>
          <p className="mt-1 text-2xl font-semibold text-yellow-700">
            {preview.warningRows.toLocaleString()}
          </p>
        </div>
      </div>

      {/* Record Type Breakdown */}
      <div className="rounded-lg border border-gray-200 bg-white p-4">
        <h3 className="text-sm font-medium text-gray-900">Import Actions</h3>
        <div className="mt-3 flex gap-6">
          <div className="flex items-center gap-2">
            <div className="h-3 w-3 rounded-full bg-green-500" />
            <span className="text-sm text-gray-600">
              New records: {preview.recordCounts.newRecords.toLocaleString()}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <div className="h-3 w-3 rounded-full bg-blue-500" />
            <span className="text-sm text-gray-600">
              Updates: {preview.recordCounts.updates.toLocaleString()}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <div className="h-3 w-3 rounded-full bg-gray-400" />
            <span className="text-sm text-gray-600">
              Skipped: {preview.recordCounts.skipped.toLocaleString()}
            </span>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200">
        <nav className="-mb-px flex gap-6">
          {[
            { id: 'summary', label: 'Summary' },
            { id: 'issues', label: `Issues (${preview.totalIssueCount})` },
            { id: 'mapping', label: 'Column Mapping' },
            { id: 'sample', label: 'Sample Data' },
          ].map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => setActiveTab(tab.id as typeof activeTab)}
              className={`border-b-2 px-1 py-3 text-sm font-medium ${
                activeTab === tab.id
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </nav>
      </div>

      {/* Tab Content */}
      <div className="min-h-[200px]">
        {activeTab === 'summary' && (
          <div className="space-y-4">
            {/* Issue Summary */}
            <div className="flex gap-4">
              {errorCount > 0 && (
                <span className="inline-flex items-center gap-1 rounded-full bg-red-100 px-3 py-1 text-sm text-red-800">
                  {SEVERITY_ICONS.error}
                  {errorCount} errors
                </span>
              )}
              {warningCount > 0 && (
                <span className="inline-flex items-center gap-1 rounded-full bg-yellow-100 px-3 py-1 text-sm text-yellow-800">
                  {SEVERITY_ICONS.warning}
                  {warningCount} warnings
                </span>
              )}
              {infoCount > 0 && (
                <span className="inline-flex items-center gap-1 rounded-full bg-blue-100 px-3 py-1 text-sm text-blue-800">
                  {SEVERITY_ICONS.info}
                  {infoCount} info
                </span>
              )}
              {preview.totalIssueCount === 0 && (
                <span className="inline-flex items-center gap-1 rounded-full bg-green-100 px-3 py-1 text-sm text-green-800">
                  No issues found
                </span>
              )}
            </div>

            {/* Top Issues */}
            {preview.issues.length > 0 && (
              <div className="space-y-2">
                <h4 className="text-sm font-medium text-gray-700">Top Issues</h4>
                {preview.issues.slice(0, 5).map((issue, index) => (
                  <div
                    key={index}
                    className={`rounded-lg border p-3 ${SEVERITY_COLORS[issue.severity]}`}
                  >
                    <div className="flex items-start gap-2">
                      {SEVERITY_ICONS[issue.severity]}
                      <div className="flex-1">
                        <p className="text-sm font-medium">{issue.message}</p>
                        <p className="mt-1 text-xs opacity-75">
                          {issue.rowNumber && `Row ${issue.rowNumber}`}
                          {issue.column && ` - ${issue.column}`}
                          {issue.originalValue && `: "${issue.originalValue}"`}
                        </p>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {activeTab === 'issues' && (
          <div className="space-y-2">
            {preview.issues.length === 0 ? (
              <p className="py-8 text-center text-sm text-gray-500">No validation issues found.</p>
            ) : (
              preview.issues.map((issue, index) => (
                <div
                  key={index}
                  className={`rounded-lg border p-3 ${SEVERITY_COLORS[issue.severity]}`}
                >
                  <div className="flex items-start gap-2">
                    {SEVERITY_ICONS[issue.severity]}
                    <div className="flex-1">
                      <div className="flex items-center justify-between">
                        <p className="text-sm font-medium">{issue.message}</p>
                        <span className="text-xs opacity-75">{issue.code}</span>
                      </div>
                      <p className="mt-1 text-xs opacity-75">
                        {issue.rowNumber && `Row ${issue.rowNumber}`}
                        {issue.column && ` - Column: ${issue.column}`}
                      </p>
                      {issue.originalValue && (
                        <p className="mt-1 text-xs">
                          Original:{' '}
                          <code className="rounded bg-white/50 px-1">{issue.originalValue}</code>
                        </p>
                      )}
                      {issue.suggestedValue && (
                        <p className="text-xs">
                          Suggested:{' '}
                          <code className="rounded bg-white/50 px-1">{issue.suggestedValue}</code>
                        </p>
                      )}
                    </div>
                  </div>
                </div>
              ))
            )}
            {preview.totalIssueCount > preview.issues.length && (
              <p className="pt-2 text-center text-sm text-gray-500">
                Showing {preview.issues.length} of {preview.totalIssueCount} issues
              </p>
            )}
          </div>
        )}

        {activeTab === 'mapping' && (
          <div className="overflow-hidden rounded-lg border border-gray-200">
            <table className="min-w-full divide-y divide-gray-200">
              <thead className="bg-gray-50">
                <tr>
                  <th className="px-4 py-3 text-left text-xs font-medium uppercase text-gray-500">
                    Source Column
                  </th>
                  <th className="px-4 py-3 text-left text-xs font-medium uppercase text-gray-500">
                    Target Field
                  </th>
                  <th className="px-4 py-3 text-left text-xs font-medium uppercase text-gray-500">
                    Status
                  </th>
                  <th className="px-4 py-3 text-left text-xs font-medium uppercase text-gray-500">
                    Sample Values
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-200 bg-white">
                {preview.columnMapping.map((mapping, index) => (
                  <tr key={index}>
                    <td className="whitespace-nowrap px-4 py-3 text-sm font-medium text-gray-900">
                      {mapping.sourceColumn}
                    </td>
                    <td className="whitespace-nowrap px-4 py-3 text-sm text-gray-600">
                      {mapping.targetField ?? '-'}
                    </td>
                    <td className="whitespace-nowrap px-4 py-3">
                      {mapping.isMapped ? (
                        <span className="inline-flex items-center rounded-full bg-green-100 px-2 py-0.5 text-xs font-medium text-green-800">
                          Mapped
                        </span>
                      ) : mapping.isRequired ? (
                        <span className="inline-flex items-center rounded-full bg-red-100 px-2 py-0.5 text-xs font-medium text-red-800">
                          Missing (Required)
                        </span>
                      ) : (
                        <span className="inline-flex items-center rounded-full bg-gray-100 px-2 py-0.5 text-xs font-medium text-gray-600">
                          Unmapped
                        </span>
                      )}
                    </td>
                    <td className="px-4 py-3 text-sm text-gray-500">
                      {mapping.sampleValues.slice(0, 2).join(', ')}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {activeTab === 'sample' && (
          <div className="overflow-x-auto">
            {preview.sampleRecords.length === 0 ? (
              <p className="py-8 text-center text-sm text-gray-500">No sample records available.</p>
            ) : (
              <table className="min-w-full divide-y divide-gray-200">
                <thead className="bg-gray-50">
                  <tr>
                    {Object.keys(preview.sampleRecords[0] || {}).map((key) => (
                      <th
                        key={key}
                        className="px-4 py-3 text-left text-xs font-medium uppercase text-gray-500"
                      >
                        {key}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-200 bg-white">
                  {preview.sampleRecords.map((record, index) => (
                    <tr key={index}>
                      {Object.values(record).map((value, colIndex) => (
                        <td
                          key={colIndex}
                          className="whitespace-nowrap px-4 py-3 text-sm text-gray-600"
                        >
                          {String(value ?? '-')}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        )}
      </div>

      {/* Warning Acknowledgment */}
      {warningCount > 0 && preview.isValid && (
        <div className="rounded-lg bg-yellow-50 p-4">
          <label className="flex items-start gap-3">
            <input
              type="checkbox"
              checked={acknowledgeWarnings}
              onChange={(e) => setAcknowledgeWarnings(e.target.checked)}
              className="mt-0.5 h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
            />
            <div>
              <span className="text-sm font-medium text-yellow-800">
                I acknowledge the {warningCount} warnings and want to proceed
              </span>
              <p className="text-xs text-yellow-700">
                Warnings indicate potential issues but will not prevent the import.
              </p>
            </div>
          </label>
        </div>
      )}

      {/* Actions */}
      <div className="flex justify-end gap-3 border-t border-gray-200 pt-4">
        <button
          type="button"
          onClick={onCancel}
          disabled={isLoading}
          className="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50"
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={() => onApprove(acknowledgeWarnings)}
          disabled={!canApprove || isLoading}
          className="inline-flex items-center rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50"
        >
          {isLoading ? 'Starting Import...' : 'Approve & Import'}
        </button>
      </div>
    </div>
  );
}
