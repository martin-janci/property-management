/**
 * FileDisputePage - page for filing a new dispute.
 * Epic 77: Dispute Resolution (Story 77.1)
 */

import { useState } from 'react';
import { categoryLabels, type DisputeCategory } from '../components/DisputeCard';

/** Form data for creating a dispute */
export interface DisputeFormData {
  category: DisputeCategory;
  title: string;
  description: string;
  desiredResolution?: string;
  respondentIds: string[];
  buildingId?: string;
  unitId?: string;
}

interface FileDisputePageProps {
  buildings?: Array<{ id: string; name: string }>;
  units?: Array<{ id: string; designation: string; buildingId: string }>;
  residents?: Array<{ id: string; name: string; unitId?: string }>;
  /** Loading state managed by parent - set to true while form is submitting */
  isSubmitting?: boolean;
  /** Supports async handlers - parent manages loading state via isSubmitting prop */
  onSubmit: (data: DisputeFormData) => void | Promise<void>;
  onCancel: () => void;
}

export function FileDisputePage({
  buildings,
  units,
  residents,
  isSubmitting,
  onSubmit,
  onCancel,
}: FileDisputePageProps) {
  const [category, setCategory] = useState<DisputeCategory>('other');
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [desiredResolution, setDesiredResolution] = useState('');
  const [selectedRespondents, setSelectedRespondents] = useState<string[]>([]);
  const [buildingId, setBuildingId] = useState<string>('');
  const [unitId, setUnitId] = useState<string>('');

  const filteredUnits = units?.filter((u) => !buildingId || u.buildingId === buildingId);
  const filteredResidents = residents?.filter((r) => !unitId || r.unitId === unitId);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      category,
      title,
      description,
      desiredResolution: desiredResolution || undefined,
      respondentIds: selectedRespondents,
      buildingId: buildingId || undefined,
      unitId: unitId || undefined,
    });
  };

  const handleRespondentToggle = (id: string) => {
    setSelectedRespondents((prev) =>
      prev.includes(id) ? prev.filter((r) => r !== id) : [...prev, id]
    );
  };

  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      <div className="mb-6">
        <button
          type="button"
          onClick={onCancel}
          className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 mb-4"
        >
          Back to Disputes
        </button>
        <h1 className="text-2xl font-bold text-gray-900">File a Dispute</h1>
        <p className="text-gray-500 mt-1">
          Submit a formal dispute for resolution through proper channels.
        </p>
      </div>

      <form onSubmit={handleSubmit} className="bg-white rounded-lg shadow p-6 space-y-6">
        {/* Category */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Category *</label>
          <select
            value={category}
            onChange={(e) => setCategory(e.target.value as DisputeCategory)}
            className="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
            required
          >
            {Object.entries(categoryLabels).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
          <p className="text-sm text-gray-500 mt-1">
            Select the category that best describes your dispute.
          </p>
        </div>

        {/* Title */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Title *</label>
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="Brief summary of the dispute"
            className="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
            required
            maxLength={200}
          />
          <p className="text-sm text-gray-500 mt-1">
            A clear, concise title for your dispute (max 200 characters).
          </p>
        </div>

        {/* Location */}
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Building (optional)
            </label>
            <select
              value={buildingId}
              onChange={(e) => {
                setBuildingId(e.target.value);
                setUnitId('');
              }}
              className="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="">Select building...</option>
              {buildings?.map((b) => (
                <option key={b.id} value={b.id}>
                  {b.name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Unit (optional)</label>
            <select
              value={unitId}
              onChange={(e) => setUnitId(e.target.value)}
              className="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
              disabled={!buildingId}
            >
              <option value="">Select unit...</option>
              {filteredUnits?.map((u) => (
                <option key={u.id} value={u.id}>
                  {u.designation}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* Description */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Description of Issue *
          </label>
          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Describe the issue in detail. Include dates, times, and specific incidents."
            rows={6}
            className="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
            required
          />
          <p className="text-sm text-gray-500 mt-1">
            Provide as much detail as possible about the dispute.
          </p>
        </div>

        {/* Desired Resolution */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Desired Resolution (optional)
          </label>
          <textarea
            value={desiredResolution}
            onChange={(e) => setDesiredResolution(e.target.value)}
            placeholder="What outcome would you like to see?"
            rows={3}
            className="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <p className="text-sm text-gray-500 mt-1">
            Describe how you would like this dispute to be resolved.
          </p>
        </div>

        {/* Respondents */}
        {residents && residents.length > 0 && (
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Other Party/Parties Involved
            </label>
            <div className="border border-gray-300 rounded-md p-3 max-h-48 overflow-y-auto space-y-2">
              {filteredResidents?.map((resident) => (
                <label key={resident.id} className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={selectedRespondents.includes(resident.id)}
                    onChange={() => handleRespondentToggle(resident.id)}
                    className="rounded border-gray-300"
                  />
                  <span className="text-sm text-gray-700">{resident.name}</span>
                </label>
              ))}
            </div>
            <p className="text-sm text-gray-500 mt-1">
              Select the other party or parties involved in this dispute.
            </p>
          </div>
        )}

        {/* Info Box */}
        <div className="bg-blue-50 border border-blue-100 rounded-lg p-4">
          <h3 className="font-medium text-blue-900 mb-2">What happens next?</h3>
          <ol className="text-sm text-blue-800 space-y-1 list-decimal list-inside">
            <li>Your dispute will be assigned a reference number</li>
            <li>The other party will be notified and can respond</li>
            <li>A manager or mediator will review the case</li>
            <li>Mediation sessions may be scheduled if needed</li>
            <li>A resolution will be proposed and tracked</li>
          </ol>
        </div>

        {/* Evidence Notice */}
        <div className="bg-gray-50 border border-gray-200 rounded-lg p-4">
          <p className="text-sm text-gray-600">
            You can add supporting evidence (photos, documents) after filing the dispute.
          </p>
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-3 pt-4 border-t">
          <button
            type="button"
            onClick={onCancel}
            className="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
            disabled={isSubmitting}
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={!title.trim() || !description.trim() || isSubmitting}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
          >
            {isSubmitting ? 'Filing...' : 'File Dispute'}
          </button>
        </div>
      </form>
    </div>
  );
}
