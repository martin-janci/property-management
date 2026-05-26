/**
 * MediationResolutionForm - form for recording dispute resolution.
 * Epic 80: Dispute Mediation (Story 80.3)
 *
 * Wraps the ResolveDisputeRequest payload with a structured form:
 *   - Resolution type selector (mutual_agreement / favor_filer / favor_respondent / withdrawn / dismissed)
 *   - Details textarea
 *   - Terms textarea (optional)
 *   - Requires confirmation checkbox
 *
 * The form uses controlled state via useState to stay lightweight.
 * On submit it calls the onResolve callback; the parent owns the mutation.
 */

import type { ResolutionType, ResolveDisputeRequest } from '@ppt/api-client';
import { useState } from 'react';

const resolutionTypeOptions: Array<{ value: ResolutionType; label: string; description: string }> =
  [
    {
      value: 'mutual_agreement',
      label: 'Mutual Agreement',
      description: 'Both parties reached a mutually acceptable agreement',
    },
    {
      value: 'favor_filer',
      label: 'In Favour of Filer',
      description: 'Resolution decided in favour of the party who filed the dispute',
    },
    {
      value: 'favor_respondent',
      label: 'In Favour of Respondent',
      description: 'Resolution decided in favour of the responding party',
    },
    {
      value: 'withdrawn',
      label: 'Withdrawn',
      description: 'The filer has withdrawn the dispute',
    },
    {
      value: 'dismissed',
      label: 'Dismissed',
      description: 'Dispute dismissed — insufficient grounds or procedural issue',
    },
  ];

interface MediationResolutionFormProps {
  disputeId: string;
  isSubmitting?: boolean;
  onResolve: (data: ResolveDisputeRequest) => void;
  onCancel: () => void;
}

export function MediationResolutionForm({
  isSubmitting,
  onResolve,
  onCancel,
}: MediationResolutionFormProps) {
  const [resolutionType, setResolutionType] = useState<ResolutionType>('mutual_agreement');
  const [resolutionDetails, setResolutionDetails] = useState('');
  const [terms, setTerms] = useState('');
  const [requiresConfirmation, setRequiresConfirmation] = useState(false);

  const canSubmit = resolutionDetails.trim().length >= 10;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit || isSubmitting) return;

    onResolve({
      resolutionType,
      resolutionDetails: resolutionDetails.trim(),
      terms: terms.trim() || undefined,
      requiresConfirmation,
    });
  };

  const selectedOption = resolutionTypeOptions.find((o) => o.value === resolutionType);

  return (
    <form onSubmit={handleSubmit} className="space-y-5">
      {/* Resolution type */}
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-2">
          Resolution Type <span className="text-red-500">*</span>
        </label>
        <div className="space-y-2">
          {resolutionTypeOptions.map((option) => (
            <label
              key={option.value}
              className={[
                'flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition-colors',
                resolutionType === option.value
                  ? 'border-violet-500 bg-violet-50'
                  : 'border-gray-200 hover:border-gray-300',
              ].join(' ')}
            >
              <input
                type="radio"
                name="resolutionType"
                value={option.value}
                checked={resolutionType === option.value}
                onChange={() => setResolutionType(option.value)}
                className="mt-1 text-violet-600"
              />
              <div>
                <p className="text-sm font-medium text-gray-900">{option.label}</p>
                <p className="text-xs text-gray-500 mt-0.5">{option.description}</p>
              </div>
            </label>
          ))}
        </div>
        {selectedOption && (
          <p className="mt-2 text-xs text-gray-500 italic">Selected: {selectedOption.label}</p>
        )}
      </div>

      {/* Resolution details */}
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">
          Resolution Details <span className="text-red-500">*</span>
        </label>
        <textarea
          value={resolutionDetails}
          onChange={(e) => setResolutionDetails(e.target.value)}
          rows={5}
          placeholder="Describe the full resolution, including what was agreed and any conditions…"
          className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent resize-none"
          required
        />
        <p className="mt-1 text-xs text-gray-400">
          {resolutionDetails.trim().length}/10 characters minimum
        </p>
      </div>

      {/* Terms (optional) */}
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">
          Terms / Conditions <span className="text-xs font-normal text-gray-400">(optional)</span>
        </label>
        <textarea
          value={terms}
          onChange={(e) => setTerms(e.target.value)}
          rows={3}
          placeholder="List any specific terms, deadlines, or follow-up actions…"
          className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent resize-none"
        />
      </div>

      {/* Requires confirmation */}
      <label className="flex items-start gap-2.5 cursor-pointer">
        <input
          type="checkbox"
          checked={requiresConfirmation}
          onChange={(e) => setRequiresConfirmation(e.target.checked)}
          className="mt-0.5 rounded border-gray-300 text-violet-600"
        />
        <div>
          <p className="text-sm font-medium text-gray-700">Requires party confirmation</p>
          <p className="text-xs text-gray-500 mt-0.5">
            Both parties must acknowledge the resolution before it is finalised.
          </p>
        </div>
      </label>

      {/* Actions */}
      <div className="flex justify-end gap-3 pt-2">
        <button
          type="button"
          onClick={onCancel}
          className="px-4 py-2 text-sm border border-gray-300 rounded-lg hover:bg-gray-50"
        >
          Cancel
        </button>
        <button
          type="submit"
          disabled={!canSubmit || isSubmitting}
          className="px-5 py-2 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed font-medium"
        >
          {isSubmitting ? 'Recording resolution…' : 'Record Resolution'}
        </button>
      </div>
    </form>
  );
}
