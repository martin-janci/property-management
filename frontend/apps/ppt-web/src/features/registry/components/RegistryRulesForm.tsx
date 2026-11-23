/**
 * RegistryRulesForm Component
 *
 * Form for configuring building registry rules (Epic 57, Story 57.7).
 */

import type { BuildingRegistryRules, PetType, UpdateRegistryRulesRequest } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface RegistryRulesFormProps {
  initialData?: BuildingRegistryRules;
  onSubmit: (data: UpdateRegistryRulesRequest) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

const petTypes: PetType[] = ['dog', 'cat', 'bird', 'fish', 'reptile', 'rodent', 'other'];

export function RegistryRulesForm({
  initialData,
  onSubmit,
  onCancel,
  isSubmitting,
}: RegistryRulesFormProps) {
  const { t } = useTranslation();
  const [formData, setFormData] = useState<UpdateRegistryRulesRequest>({
    petsAllowed: initialData?.petsAllowed ?? true,
    petsRequireApproval: initialData?.petsRequireApproval ?? true,
    maxPetsPerUnit: initialData?.maxPetsPerUnit,
    allowedPetTypes: initialData?.allowedPetTypes || [],
    bannedPetBreeds: initialData?.bannedPetBreeds || [],
    maxPetWeight: initialData?.maxPetWeight,
    vehiclesRequireApproval: initialData?.vehiclesRequireApproval ?? false,
    maxVehiclesPerUnit: initialData?.maxVehiclesPerUnit,
    notes: initialData?.notes || '',
  });

  const [bannedBreedInput, setBannedBreedInput] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(formData);
  };

  const handlePetTypeToggle = (petType: PetType) => {
    const current = formData.allowedPetTypes || [];
    const updated = current.includes(petType)
      ? current.filter((t) => t !== petType)
      : [...current, petType];
    setFormData({ ...formData, allowedPetTypes: updated });
  };

  const addBannedBreed = () => {
    if (!bannedBreedInput.trim()) return;
    const current = formData.bannedPetBreeds || [];
    if (!current.includes(bannedBreedInput.trim())) {
      setFormData({ ...formData, bannedPetBreeds: [...current, bannedBreedInput.trim()] });
    }
    setBannedBreedInput('');
  };

  const removeBannedBreed = (breed: string) => {
    const current = formData.bannedPetBreeds || [];
    setFormData({ ...formData, bannedPetBreeds: current.filter((b) => b !== breed) });
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Pet rules section */}
      <div>
        <h3 className="text-lg font-medium text-gray-900 mb-4">
          {t('registry.rules.form.petRulesHeading')}
        </h3>

        <div className="space-y-4">
          {/* Pets Allowed */}
          <div className="flex items-center">
            <input
              id="petsAllowed"
              type="checkbox"
              checked={formData.petsAllowed}
              onChange={(e) => setFormData({ ...formData, petsAllowed: e.target.checked })}
              className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
            />
            <label htmlFor="petsAllowed" className="ml-2 block text-sm text-gray-900">
              {t('registry.rules.form.petsAllowed')}
            </label>
          </div>

          {formData.petsAllowed && (
            <>
              {/* Require Approval */}
              <div className="flex items-center ml-6">
                <input
                  id="petsRequireApproval"
                  type="checkbox"
                  checked={formData.petsRequireApproval}
                  onChange={(e) =>
                    setFormData({ ...formData, petsRequireApproval: e.target.checked })
                  }
                  className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label htmlFor="petsRequireApproval" className="ml-2 block text-sm text-gray-900">
                  {t('registry.rules.form.petsRequireApproval')}
                </label>
              </div>

              {/* Max Pets Per Unit */}
              <div className="ml-6">
                <label
                  htmlFor="maxPetsPerUnit"
                  className="block text-sm font-medium text-gray-700 mb-1"
                >
                  {t('registry.rules.form.maxPetsPerUnit')}
                </label>
                <input
                  id="maxPetsPerUnit"
                  type="number"
                  min="1"
                  max="10"
                  value={formData.maxPetsPerUnit || ''}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      maxPetsPerUnit: e.target.value ? Number(e.target.value) : undefined,
                    })
                  }
                  className="w-32 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder={t('registry.rules.form.noLimit')}
                />
              </div>

              {/* Allowed Pet Types */}
              <div className="ml-6">
                <span className="block text-sm font-medium text-gray-700 mb-2">
                  {t('registry.rules.form.allowedPetTypes')}
                </span>
                <div className="flex flex-wrap gap-2">
                  {petTypes.map((type) => (
                    <label
                      key={type}
                      className="inline-flex items-center px-3 py-2 border border-gray-300 rounded-md cursor-pointer hover:bg-gray-50"
                    >
                      <input
                        type="checkbox"
                        checked={(formData.allowedPetTypes || []).includes(type)}
                        onChange={() => handlePetTypeToggle(type)}
                        className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                      />
                      <span className="ml-2 text-sm text-gray-900">
                        {t(`registry.pet.types.${type}`)}
                      </span>
                    </label>
                  ))}
                </div>
              </div>

              {/* Banned Breeds */}
              <div className="ml-6">
                <span className="block text-sm font-medium text-gray-700 mb-2">
                  {t('registry.rules.form.bannedBreeds')}
                </span>
                <div className="flex gap-2 mb-2">
                  <input
                    type="text"
                    value={bannedBreedInput}
                    onChange={(e) => setBannedBreedInput(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        e.preventDefault();
                        addBannedBreed();
                      }
                    }}
                    className="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    placeholder={t('registry.rules.form.bannedBreedPlaceholder')}
                  />
                  <button
                    type="button"
                    onClick={addBannedBreed}
                    className="px-4 py-2 bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200"
                  >
                    {t('registry.rules.form.add')}
                  </button>
                </div>
                {formData.bannedPetBreeds && formData.bannedPetBreeds.length > 0 && (
                  <div className="flex flex-wrap gap-2">
                    {formData.bannedPetBreeds.map((breed) => (
                      <span
                        key={breed}
                        className="inline-flex items-center px-3 py-1 bg-red-100 text-red-800 rounded-full text-sm"
                      >
                        {breed}
                        <button
                          type="button"
                          onClick={() => removeBannedBreed(breed)}
                          className="ml-2 hover:text-red-900"
                        >
                          ×
                        </button>
                      </span>
                    ))}
                  </div>
                )}
              </div>

              {/* Max Pet Weight */}
              <div className="ml-6">
                <label
                  htmlFor="maxPetWeight"
                  className="block text-sm font-medium text-gray-700 mb-1"
                >
                  {t('registry.rules.form.maxPetWeight')}
                </label>
                <input
                  id="maxPetWeight"
                  type="number"
                  min="1"
                  max="200"
                  value={formData.maxPetWeight || ''}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      maxPetWeight: e.target.value ? Number(e.target.value) : undefined,
                    })
                  }
                  className="w-32 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder={t('registry.rules.form.noLimit')}
                />
              </div>
            </>
          )}
        </div>
      </div>

      {/* Vehicle rules section */}
      <div className="border-t pt-6">
        <h3 className="text-lg font-medium text-gray-900 mb-4">
          {t('registry.rules.form.vehicleRulesHeading')}
        </h3>

        <div className="space-y-4">
          {/* Require Approval */}
          <div className="flex items-center">
            <input
              id="vehiclesRequireApproval"
              type="checkbox"
              checked={formData.vehiclesRequireApproval}
              onChange={(e) =>
                setFormData({ ...formData, vehiclesRequireApproval: e.target.checked })
              }
              className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
            />
            <label htmlFor="vehiclesRequireApproval" className="ml-2 block text-sm text-gray-900">
              {t('registry.rules.form.vehiclesRequireApproval')}
            </label>
          </div>

          {/* Max Vehicles Per Unit */}
          <div>
            <label
              htmlFor="maxVehiclesPerUnit"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('registry.rules.form.maxVehiclesPerUnit')}
            </label>
            <input
              id="maxVehiclesPerUnit"
              type="number"
              min="1"
              max="10"
              value={formData.maxVehiclesPerUnit || ''}
              onChange={(e) =>
                setFormData({
                  ...formData,
                  maxVehiclesPerUnit: e.target.value ? Number(e.target.value) : undefined,
                })
              }
              className="w-32 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder={t('registry.rules.form.noLimit')}
            />
          </div>
        </div>
      </div>

      {/* Notes */}
      <div className="border-t pt-6">
        <label htmlFor="notes" className="block text-sm font-medium text-gray-700 mb-1">
          {t('registry.rules.form.additionalNotes')}
        </label>
        <textarea
          id="notes"
          rows={4}
          value={formData.notes || ''}
          onChange={(e) => setFormData({ ...formData, notes: e.target.value })}
          className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          placeholder={t('registry.rules.form.notesPlaceholder')}
        />
      </div>

      {/* Actions */}
      <div className="flex justify-end gap-3 pt-6 border-t">
        <button
          type="button"
          onClick={onCancel}
          disabled={isSubmitting}
          className="px-4 py-2 border border-gray-300 rounded-md text-gray-700 hover:bg-gray-50 disabled:opacity-50"
        >
          {t('registry.rules.form.cancel')}
        </button>
        <button
          type="submit"
          disabled={isSubmitting}
          className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 flex items-center gap-2"
        >
          {isSubmitting && (
            <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
          )}
          {isSubmitting ? t('registry.rules.form.saving') : t('registry.rules.form.save')}
        </button>
      </div>
    </form>
  );
}
