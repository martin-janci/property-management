/**
 * TriageFaultDialog component - modal dialog for triaging a fault.
 * Epic 4: Fault Reporting & Resolution (UC-03.6, UC-03.8, UC-03.9)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { FaultCategory, FaultPriority } from './FaultCard';

export interface TriageData {
  priority: FaultPriority;
  category?: FaultCategory;
  assignedTo?: string;
}

interface TriageFaultDialogProps {
  isOpen: boolean;
  faultTitle: string;
  currentCategory: FaultCategory;
  aiSuggestion?: {
    category: string;
    priority?: string;
    confidence: number;
  };
  technicians?: Array<{ id: string; name: string }>;
  isSubmitting?: boolean;
  onSubmit: (data: TriageData) => void;
  onClose: () => void;
}

const priorityOptions: { value: FaultPriority; label: string; description: string }[] = [
  { value: 'low', label: 'Low', description: 'Minor issue, no urgency' },
  { value: 'medium', label: 'Medium', description: 'Standard priority' },
  { value: 'high', label: 'High', description: 'Needs prompt attention' },
  { value: 'urgent', label: 'Urgent', description: 'Immediate action required' },
];

const categoryOptions: { value: FaultCategory; label: string }[] = [
  { value: 'plumbing', label: 'Plumbing' },
  { value: 'electrical', label: 'Electrical' },
  { value: 'heating', label: 'Heating' },
  { value: 'structural', label: 'Structural' },
  { value: 'exterior', label: 'Exterior' },
  { value: 'elevator', label: 'Elevator' },
  { value: 'common_area', label: 'Common Area' },
  { value: 'security', label: 'Security' },
  { value: 'cleaning', label: 'Cleaning' },
  { value: 'other', label: 'Other' },
];

export function TriageFaultDialog({
  isOpen,
  faultTitle,
  currentCategory,
  aiSuggestion,
  technicians = [],
  isSubmitting,
  onSubmit,
  onClose,
}: TriageFaultDialogProps) {
  const { t } = useTranslation();
  const [priority, setPriority] = useState<FaultPriority>(
    (aiSuggestion?.priority as FaultPriority) || 'medium'
  );
  const [category, setCategory] = useState<FaultCategory>(
    (aiSuggestion?.category as FaultCategory) || currentCategory
  );
  const [assignedTo, setAssignedTo] = useState<string>('');

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      priority,
      category: category !== currentCategory ? category : undefined,
      assignedTo: assignedTo || undefined,
    });
  };

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      {/* Backdrop */}
      <button
        type="button"
        className="fixed inset-0 bg-black bg-opacity-50 transition-opacity cursor-default"
        onClick={onClose}
        onKeyDown={(e) => e.key === 'Escape' && onClose()}
        aria-label={t('aria.closeDialog')}
      />

      {/* Dialog */}
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="relative w-full max-w-md bg-white rounded-lg shadow-xl">
          {/* Header */}
          <div className="px-6 py-4 border-b">
            <h2 className="text-lg font-semibold text-gray-900">Triage Fault</h2>
            <p className="text-sm text-gray-500 mt-1 truncate">{faultTitle}</p>
          </div>

          {/* AI Suggestion */}
          {aiSuggestion && aiSuggestion.confidence > 0.7 && (
            <div className="px-6 py-3 bg-blue-50 border-b border-blue-100">
              <div className="flex items-center gap-2">
                <span className="text-blue-600">🤖</span>
                <span className="text-sm text-blue-800">
                  AI suggests: <strong>{aiSuggestion.category}</strong>
                  {aiSuggestion.priority && (
                    <>
                      {' '}
                      with <strong>{aiSuggestion.priority}</strong> priority
                    </>
                  )}
                  <span className="ml-1 text-blue-600">
                    ({Math.round(aiSuggestion.confidence * 100)}% confidence)
                  </span>
                </span>
              </div>
            </div>
          )}

          {/* Form */}
          <form onSubmit={handleSubmit} className="px-6 py-4 space-y-4">
            {/* Priority */}
            <fieldset>
              <legend className="block text-sm font-medium text-gray-700 mb-2">Priority *</legend>
              <div className="space-y-2">
                {priorityOptions.map((opt) => (
                  <label
                    key={opt.value}
                    className={`flex items-start p-3 border rounded-lg cursor-pointer ${
                      priority === opt.value
                        ? 'border-blue-500 bg-blue-50'
                        : 'border-gray-200 hover:border-gray-300'
                    }`}
                  >
                    <input
                      type="radio"
                      name="priority"
                      value={opt.value}
                      checked={priority === opt.value}
                      onChange={(e) => setPriority(e.target.value as FaultPriority)}
                      className="mt-0.5"
                    />
                    <div className="ml-3">
                      <div className="text-sm font-medium text-gray-900">{opt.label}</div>
                      <div className="text-xs text-gray-500">{opt.description}</div>
                    </div>
                  </label>
                ))}
              </div>
            </fieldset>

            {/* Category */}
            <div>
              <label htmlFor="category" className="block text-sm font-medium text-gray-700">
                Category
              </label>
              <select
                id="category"
                value={category}
                onChange={(e) => setCategory(e.target.value as FaultCategory)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                {categoryOptions.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
            </div>

            {/* Assign To */}
            {technicians.length > 0 && (
              <div>
                <label htmlFor="assignedTo" className="block text-sm font-medium text-gray-700">
                  Assign to (optional)
                </label>
                <select
                  id="assignedTo"
                  value={assignedTo}
                  onChange={(e) => setAssignedTo(e.target.value)}
                  className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  <option value="">Unassigned</option>
                  {technicians.map((t) => (
                    <option key={t.id} value={t.id}>
                      {t.name}
                    </option>
                  ))}
                </select>
              </div>
            )}
          </form>

          {/* Actions */}
          <div className="px-6 py-4 border-t flex justify-end gap-3">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-4 py-2 text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={handleSubmit}
              disabled={isSubmitting}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 flex items-center gap-2"
            >
              {isSubmitting && (
                <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
              )}
              {isSubmitting ? 'Saving...' : 'Triage Fault'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
