/**
 * GroupForm Component
 *
 * Form for creating and editing community groups.
 * Part of Story 42.1: Community Groups.
 */

import type { CreateGroupRequest, GroupVisibility, UpdateGroupRequest } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface GroupFormProps {
  initialData?: Partial<CreateGroupRequest>;
  isEditing?: boolean;
  isSubmitting?: boolean;
  onSubmit: (data: CreateGroupRequest | UpdateGroupRequest) => void;
  onCancel: () => void;
}

// `value` is the canonical string sent to the API; `key` resolves the label.
const categories: { value: string; key: string }[] = [
  { value: 'Social', key: 'social' },
  { value: 'Sports', key: 'sports' },
  { value: 'Hobbies', key: 'hobbies' },
  { value: 'Parenting', key: 'parenting' },
  { value: 'Pets', key: 'pets' },
  { value: 'Business', key: 'business' },
  { value: 'Other', key: 'other' },
];

const visibilityOptions: { value: GroupVisibility; key: string }[] = [
  { value: 'public', key: 'public' },
  { value: 'private', key: 'private' },
  { value: 'hidden', key: 'hidden' },
];

export function GroupForm({
  initialData,
  isEditing = false,
  isSubmitting = false,
  onSubmit,
  onCancel,
}: GroupFormProps) {
  const { t } = useTranslation();
  const [name, setName] = useState(initialData?.name || '');
  const [description, setDescription] = useState(initialData?.description || '');
  const [category, setCategory] = useState(initialData?.category || categories[0].value);
  const [visibility, setVisibility] = useState<GroupVisibility>(
    initialData?.visibility || 'public'
  );
  const [coverImageUrl, setCoverImageUrl] = useState(initialData?.coverImageUrl || '');
  const [errors, setErrors] = useState<Record<string, string>>({});

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!name.trim()) {
      newErrors.name = t('community.groupForm.errors.nameRequired');
    } else if (name.length < 3) {
      newErrors.name = t('community.groupForm.errors.nameMin');
    } else if (name.length > 100) {
      newErrors.name = t('community.groupForm.errors.nameMax');
    }

    if (!description.trim()) {
      newErrors.description = t('community.groupForm.errors.descriptionRequired');
    } else if (description.length < 10) {
      newErrors.description = t('community.groupForm.errors.descriptionMin');
    } else if (description.length > 1000) {
      newErrors.description = t('community.groupForm.errors.descriptionMax');
    }

    if (!category) {
      newErrors.category = t('community.groupForm.errors.categoryRequired');
    }

    if (coverImageUrl && !isValidUrl(coverImageUrl)) {
      newErrors.coverImageUrl = t('community.groupForm.errors.invalidUrl');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const isValidUrl = (url: string): boolean => {
    try {
      new URL(url);
      return true;
    } catch {
      return false;
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (validate()) {
      onSubmit({
        name: name.trim(),
        description: description.trim(),
        category,
        visibility,
        coverImageUrl: coverImageUrl.trim() || undefined,
      });
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Cover Image Preview */}
      <div className="relative h-40 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg overflow-hidden">
        {coverImageUrl && (
          <img
            src={coverImageUrl}
            alt={t('community.groupForm.coverPreviewAlt')}
            className="w-full h-full object-cover"
            onError={(e) => {
              (e.target as HTMLImageElement).style.display = 'none';
            }}
          />
        )}
        <div className="absolute inset-0 flex items-center justify-center bg-black bg-opacity-30">
          <span className="text-white text-lg font-medium">
            {name || t('community.groupForm.namePreviewDefault')}
          </span>
        </div>
      </div>

      {/* Name */}
      <div>
        <label htmlFor="name" className="block text-sm font-medium text-gray-700">
          {t('community.groupForm.nameLabel')} <span className="text-red-500">*</span>
        </label>
        <input
          type="text"
          id="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t('community.groupForm.namePlaceholder')}
          className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 ${
            errors.name ? 'border-red-300' : 'border-gray-300'
          }`}
        />
        {errors.name && <p className="mt-1 text-sm text-red-600">{errors.name}</p>}
      </div>

      {/* Description */}
      <div>
        <label htmlFor="description" className="block text-sm font-medium text-gray-700">
          {t('community.groupForm.descriptionLabel')} <span className="text-red-500">*</span>
        </label>
        <textarea
          id="description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder={t('community.groupForm.descriptionPlaceholder')}
          rows={4}
          className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 ${
            errors.description ? 'border-red-300' : 'border-gray-300'
          }`}
        />
        <div className="mt-1 flex justify-between">
          {errors.description ? (
            <p className="text-sm text-red-600">{errors.description}</p>
          ) : (
            <span />
          )}
          <span className="text-xs text-gray-400">{description.length}/1000</span>
        </div>
      </div>

      {/* Category */}
      <div>
        <label htmlFor="category" className="block text-sm font-medium text-gray-700">
          {t('community.groupForm.categoryLabel')} <span className="text-red-500">*</span>
        </label>
        <select
          id="category"
          value={category}
          onChange={(e) => setCategory(e.target.value)}
          className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 ${
            errors.category ? 'border-red-300' : 'border-gray-300'
          }`}
        >
          {categories.map((cat) => (
            <option key={cat.value} value={cat.value}>
              {t(`community.groupList.categories.${cat.key}`)}
            </option>
          ))}
        </select>
        {errors.category && <p className="mt-1 text-sm text-red-600">{errors.category}</p>}
      </div>

      {/* Visibility */}
      <fieldset>
        <legend className="block text-sm font-medium text-gray-700 mb-2">
          {t('community.groupForm.visibilityLegend')}
        </legend>
        <div className="space-y-2">
          {visibilityOptions.map((option) => (
            <label
              key={option.value}
              className={`flex items-start p-3 border rounded-lg cursor-pointer transition-colors ${
                visibility === option.value
                  ? 'border-blue-500 bg-blue-50'
                  : 'border-gray-200 hover:border-gray-300'
              }`}
            >
              <input
                type="radio"
                name="visibility"
                value={option.value}
                checked={visibility === option.value}
                onChange={(e) => setVisibility(e.target.value as GroupVisibility)}
                className="mt-1 h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300"
              />
              <div className="ml-3">
                <span className="block text-sm font-medium text-gray-900">
                  {t(`community.groupForm.visibilityOptions.${option.key}.label`)}
                </span>
                <span className="block text-xs text-gray-500">
                  {t(`community.groupForm.visibilityOptions.${option.key}.description`)}
                </span>
              </div>
            </label>
          ))}
        </div>
      </fieldset>

      {/* Cover Image URL */}
      <div>
        <label htmlFor="coverImageUrl" className="block text-sm font-medium text-gray-700">
          {t('community.groupForm.coverUrlLabel')}
        </label>
        <input
          type="url"
          id="coverImageUrl"
          value={coverImageUrl}
          onChange={(e) => setCoverImageUrl(e.target.value)}
          placeholder={t('community.groupForm.coverUrlPlaceholder')}
          className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 ${
            errors.coverImageUrl ? 'border-red-300' : 'border-gray-300'
          }`}
        />
        {errors.coverImageUrl && (
          <p className="mt-1 text-sm text-red-600">{errors.coverImageUrl}</p>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center justify-end gap-3 pt-4 border-t">
        <button
          type="button"
          onClick={onCancel}
          disabled={isSubmitting}
          className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
        >
          {t('community.common.cancel')}
        </button>
        <button
          type="submit"
          disabled={isSubmitting}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {isSubmitting
            ? isEditing
              ? t('community.groupForm.updating')
              : t('community.groupForm.creating')
            : isEditing
              ? t('community.groupForm.update')
              : t('community.groupForm.create')}
        </button>
      </div>
    </form>
  );
}
