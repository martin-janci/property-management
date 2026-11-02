/**
 * ItemForm Component
 *
 * Form for creating and editing marketplace items.
 * Part of Story 42.4: Item Marketplace.
 */

import type {
  CreateItemRequest,
  ItemCondition,
  ItemListingType,
  UpdateItemRequest,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface ItemFormProps {
  initialData?: Partial<CreateItemRequest>;
  isEditing?: boolean;
  isSubmitting?: boolean;
  onSubmit: (data: CreateItemRequest | UpdateItemRequest) => void;
  onCancel: () => void;
}

// `value` is the canonical string sent to the API; `key` resolves the label.
const categories: { value: string; key: string }[] = [
  { value: 'Electronics', key: 'electronics' },
  { value: 'Furniture', key: 'furniture' },
  { value: 'Clothing', key: 'clothing' },
  { value: 'Books', key: 'books' },
  { value: 'Sports', key: 'sports' },
  { value: 'Home & Garden', key: 'homeGarden' },
  { value: 'Kids & Baby', key: 'kidsBaby' },
  { value: 'Vehicles', key: 'vehicles' },
  { value: 'Other', key: 'other' },
];

const conditions: { value: ItemCondition; key: string }[] = [
  { value: 'new', key: 'new' },
  { value: 'like_new', key: 'likeNew' },
  { value: 'good', key: 'good' },
  { value: 'fair', key: 'fair' },
  { value: 'poor', key: 'poor' },
];

const listingTypes: { value: ItemListingType; key: string }[] = [
  { value: 'sale', key: 'sale' },
  { value: 'free', key: 'free' },
  { value: 'wanted', key: 'wanted' },
  { value: 'trade', key: 'trade' },
];

export function ItemForm({
  initialData,
  isEditing = false,
  isSubmitting = false,
  onSubmit,
  onCancel,
}: ItemFormProps) {
  const { t } = useTranslation();
  const [title, setTitle] = useState(initialData?.title || '');
  const [description, setDescription] = useState(initialData?.description || '');
  const [category, setCategory] = useState(initialData?.category || categories[0].value);
  const [condition, setCondition] = useState<ItemCondition>(initialData?.condition || 'good');
  const [listingType, setListingType] = useState<ItemListingType>(
    initialData?.listingType || 'sale'
  );
  const [price, setPrice] = useState(initialData?.price?.toString() || '');
  const [currency, setCurrency] = useState(initialData?.currency || 'EUR');
  const [imageUrls, setImageUrls] = useState<string[]>(initialData?.imageUrls || []);
  const [imageInput, setImageInput] = useState('');
  const [errors, setErrors] = useState<Record<string, string>>({});

  const showPrice = listingType === 'sale';

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!title.trim()) {
      newErrors.title = t('community.itemForm.errors.titleRequired');
    } else if (title.length < 3) {
      newErrors.title = t('community.itemForm.errors.titleMin');
    } else if (title.length > 100) {
      newErrors.title = t('community.itemForm.errors.titleMax');
    }

    if (!description.trim()) {
      newErrors.description = t('community.itemForm.errors.descriptionRequired');
    } else if (description.length < 10) {
      newErrors.description = t('community.itemForm.errors.descriptionMin');
    }

    if (showPrice && price) {
      const priceNum = Number(price);
      if (Number.isNaN(priceNum) || priceNum < 0) {
        newErrors.price = t('community.itemForm.errors.invalidPrice');
      }
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

  const addImageUrl = () => {
    if (imageInput.trim() && isValidUrl(imageInput) && imageUrls.length < 10) {
      setImageUrls([...imageUrls, imageInput.trim()]);
      setImageInput('');
    }
  };

  const removeImageUrl = (index: number) => {
    setImageUrls(imageUrls.filter((_, i) => i !== index));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (validate()) {
      const data: CreateItemRequest = {
        title: title.trim(),
        description: description.trim(),
        category,
        condition,
        listingType,
        price: showPrice && price ? Number(price) : undefined,
        currency: showPrice ? currency : undefined,
        imageUrls: imageUrls.length > 0 ? imageUrls : undefined,
      };
      onSubmit(data);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Image Upload */}
      <div>
        <span className="block text-sm font-medium text-gray-700 mb-2">
          {t('community.itemForm.photos')}
        </span>
        <div className="flex flex-wrap gap-3 mb-3">
          {imageUrls.map((url, index) => (
            <div key={url} className="relative w-24 h-24 rounded-lg overflow-hidden border">
              <img
                src={url}
                alt={t('community.itemForm.imageAlt', { index: index + 1 })}
                className="w-full h-full object-cover"
              />
              <button
                type="button"
                onClick={() => removeImageUrl(index)}
                className="absolute top-1 right-1 w-5 h-5 bg-red-500 text-white rounded-full text-xs flex items-center justify-center hover:bg-red-600"
              >
                ×
              </button>
            </div>
          ))}
          {imageUrls.length < 10 && (
            <div className="w-24 h-24 border-2 border-dashed border-gray-300 rounded-lg flex items-center justify-center text-gray-400">
              <svg
                className="w-8 h-8"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 4v16m8-8H4"
                />
              </svg>
            </div>
          )}
        </div>
        <div className="flex gap-2">
          <input
            type="url"
            value={imageInput}
            onChange={(e) => setImageInput(e.target.value)}
            placeholder={t('community.itemForm.imageUrlPlaceholder')}
            className="flex-1 px-3 py-2 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
          />
          <button
            type="button"
            onClick={addImageUrl}
            disabled={!imageInput.trim() || !isValidUrl(imageInput) || imageUrls.length >= 10}
            className="px-4 py-2 text-sm bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 disabled:opacity-50"
          >
            {t('community.common.add')}
          </button>
        </div>
      </div>

      {/* Title */}
      <div>
        <label htmlFor="title" className="block text-sm font-medium text-gray-700">
          {t('community.itemForm.titleLabel')} <span className="text-red-500">*</span>
        </label>
        <input
          type="text"
          id="title"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder={t('community.itemForm.titlePlaceholder')}
          className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 ${
            errors.title ? 'border-red-300' : 'border-gray-300'
          }`}
        />
        {errors.title && <p className="mt-1 text-sm text-red-600">{errors.title}</p>}
      </div>

      {/* Description */}
      <div>
        <label htmlFor="description" className="block text-sm font-medium text-gray-700">
          {t('community.itemForm.descriptionLabel')} <span className="text-red-500">*</span>
        </label>
        <textarea
          id="description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder={t('community.itemForm.descriptionPlaceholder')}
          rows={4}
          className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 ${
            errors.description ? 'border-red-300' : 'border-gray-300'
          }`}
        />
        {errors.description && <p className="mt-1 text-sm text-red-600">{errors.description}</p>}
      </div>

      {/* Listing Type */}
      <fieldset>
        <legend className="block text-sm font-medium text-gray-700 mb-2">
          {t('community.itemForm.listingTypeLegend')}
        </legend>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          {listingTypes.map((type) => (
            <label
              key={type.value}
              className={`flex flex-col items-center p-3 border rounded-lg cursor-pointer transition-colors ${
                listingType === type.value
                  ? 'border-blue-500 bg-blue-50'
                  : 'border-gray-200 hover:border-gray-300'
              }`}
            >
              <input
                type="radio"
                name="listingType"
                value={type.value}
                checked={listingType === type.value}
                onChange={(e) => setListingType(e.target.value as ItemListingType)}
                className="sr-only"
              />
              <span className="text-sm font-medium text-gray-900">
                {t(`community.itemForm.listingTypes.${type.key}.label`)}
              </span>
              <span className="text-xs text-gray-500 text-center mt-1">
                {t(`community.itemForm.listingTypes.${type.key}.description`)}
              </span>
            </label>
          ))}
        </div>
      </fieldset>

      {/* Price (only for sale) */}
      {showPrice && (
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label htmlFor="price" className="block text-sm font-medium text-gray-700">
              {t('community.itemForm.priceLabel')}
            </label>
            <input
              type="number"
              id="price"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
              placeholder={t('community.itemForm.pricePlaceholder')}
              min="0"
              step="0.01"
              className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 ${
                errors.price ? 'border-red-300' : 'border-gray-300'
              }`}
            />
            {errors.price && <p className="mt-1 text-sm text-red-600">{errors.price}</p>}
          </div>
          <div>
            <label htmlFor="currency" className="block text-sm font-medium text-gray-700">
              {t('community.itemForm.currencyLabel')}
            </label>
            <select
              id="currency"
              value={currency}
              onChange={(e) => setCurrency(e.target.value)}
              className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
            >
              <option value="EUR">EUR (€)</option>
              <option value="USD">USD ($)</option>
              <option value="GBP">GBP (£)</option>
              <option value="CZK">CZK (Kč)</option>
            </select>
          </div>
        </div>
      )}

      {/* Category */}
      <div>
        <label htmlFor="category" className="block text-sm font-medium text-gray-700">
          {t('community.itemForm.categoryLabel')}
        </label>
        <select
          id="category"
          value={category}
          onChange={(e) => setCategory(e.target.value)}
          className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
        >
          {categories.map((cat) => (
            <option key={cat.value} value={cat.value}>
              {t(`community.marketplace.categories.${cat.key}`)}
            </option>
          ))}
        </select>
      </div>

      {/* Condition */}
      <fieldset>
        <legend className="block text-sm font-medium text-gray-700 mb-2">
          {t('community.itemForm.conditionLegend')}
        </legend>
        <div className="space-y-2">
          {conditions.map((cond) => (
            <label
              key={cond.value}
              className={`flex items-center p-3 border rounded-lg cursor-pointer transition-colors ${
                condition === cond.value
                  ? 'border-blue-500 bg-blue-50'
                  : 'border-gray-200 hover:border-gray-300'
              }`}
            >
              <input
                type="radio"
                name="condition"
                value={cond.value}
                checked={condition === cond.value}
                onChange={(e) => setCondition(e.target.value as ItemCondition)}
                className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300"
              />
              <div className="ml-3">
                <span className="block text-sm font-medium text-gray-900">
                  {t(`community.itemForm.conditions.${cond.key}.label`)}
                </span>
                <span className="block text-xs text-gray-500">
                  {t(`community.itemForm.conditions.${cond.key}.description`)}
                </span>
              </div>
            </label>
          ))}
        </div>
      </fieldset>

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
              ? t('community.itemForm.updating')
              : t('community.itemForm.creating')
            : isEditing
              ? t('community.itemForm.update')
              : t('community.itemForm.create')}
        </button>
      </div>
    </form>
  );
}
