/**
 * FileDisputeDialog - dialog for filing a new dispute.
 * Epic 77: Dispute Resolution (Story 77.1)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { categoryLabels, type DisputeCategory } from './DisputeCard';

export interface FileDisputeData {
  category: DisputeCategory;
  title: string;
  description: string;
  desiredResolution?: string;
  respondentIds: string[];
  buildingId?: string;
  unitId?: string;
}

interface FileDisputeDialogProps {
  isOpen: boolean;
  buildings?: Array<{ id: string; name: string }>;
  units?: Array<{ id: string; designation: string; buildingId: string }>;
  residents?: Array<{ id: string; name: string; unitId?: string }>;
  onSubmit: (data: FileDisputeData) => void;
  onClose: () => void;
}

export function FileDisputeDialog({
  isOpen,
  buildings,
  units,
  residents,
  onSubmit,
  onClose,
}: FileDisputeDialogProps) {
  const { t } = useTranslation();
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

  const handleClose = () => {
    setCategory('other');
    setTitle('');
    setDescription('');
    setDesiredResolution('');
    setSelectedRespondents([]);
    setBuildingId('');
    setUnitId('');
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <button
        type="button"
        className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
        onClick={handleClose}
        onKeyDown={(e) => e.key === 'Escape' && handleClose()}
        aria-label={t('aria.closeDialog')}
      />
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="relative w-full max-w-2xl bg-white rounded-lg shadow-xl">
          <form onSubmit={handleSubmit}>
            {/* Header */}
            <div className="flex items-center justify-between p-6 border-b">
              <h2 className="text-xl font-semibold text-gray-900">File a Dispute</h2>
              <button
                type="button"
                onClick={handleClose}
                className="text-gray-400 hover:text-gray-600"
              >
                X
              </button>
            </div>

            {/* Content */}
            <div className="p-6 space-y-4 max-h-[60vh] overflow-y-auto">
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
              </div>

              {/* Location (optional) */}
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
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Unit (optional)
                  </label>
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
                  rows={4}
                  className="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  required
                />
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
                  rows={2}
                  className="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              {/* Respondents */}
              {residents && residents.length > 0 && (
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Other Party/Parties Involved
                  </label>
                  <div className="border border-gray-300 rounded-md p-3 max-h-40 overflow-y-auto space-y-2">
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
                </div>
              )}

              {/* Evidence Notice */}
              <div className="bg-blue-50 border border-blue-100 rounded-lg p-4">
                <p className="text-sm text-blue-800">
                  You can add evidence (photos, documents) after filing the dispute.
                </p>
              </div>
            </div>

            {/* Footer */}
            <div className="flex justify-end gap-3 p-6 border-t">
              <button
                type="button"
                onClick={handleClose}
                className="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={!title.trim() || !description.trim()}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
              >
                File Dispute
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
