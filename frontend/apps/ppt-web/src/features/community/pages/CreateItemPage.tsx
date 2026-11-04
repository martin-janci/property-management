/**
 * CreateItemPage — `/community/marketplace/new`.
 *
 * Creates a listing in the selected building
 * (`POST /community/buildings/{id}/marketplace`) and opens it on success.
 */

import {
  type MarketplaceCategory,
  type MarketplaceItemCondition,
  useCreateItem,
} from '@ppt/api-client';
import { type FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useToast } from '../../../components';
import { getErrorMessage } from '../../../lib/api';
import { CommunityLayout } from '../components/CommunityLayout';
import { useCommunityBuilding, withBuilding } from '../hooks/useCommunityBuilding';
import { ITEM_CONDITIONS, MARKETPLACE_CATEGORIES } from './MarketplacePage';

export function CreateItemPage() {
  const { t } = useTranslation();
  const building = useCommunityBuilding();

  return (
    <CommunityLayout
      active="marketplace"
      title={t('community.itemForm.create')}
      building={building}
    >
      {(buildingId) => <CreateItemForm buildingId={buildingId} />}
    </CommunityLayout>
  );
}

function CreateItemForm({ buildingId }: { buildingId: string }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const createItem = useCreateItem();

  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [category, setCategory] = useState<MarketplaceCategory>('furniture');
  const [condition, setCondition] = useState<MarketplaceItemCondition>('good');
  const [isFree, setIsFree] = useState(false);
  const [price, setPrice] = useState('');
  const [isNegotiable, setIsNegotiable] = useState(false);
  const [isTradeAccepted, setIsTradeAccepted] = useState(false);
  const [photoUrl, setPhotoUrl] = useState('');
  const [location, setLocation] = useState('');
  const [pickupDetails, setPickupDetails] = useState('');
  const [touched, setTouched] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  const priceValue = price.trim() ? Number(price.replace(',', '.')) : undefined;
  const errors: Record<string, string | null> = {
    title:
      title.trim().length === 0
        ? t('community.itemForm.errors.titleRequired')
        : title.trim().length < 3
          ? t('community.itemForm.errors.titleMin')
          : null,
    price:
      !isFree && priceValue !== undefined && (!Number.isFinite(priceValue) || priceValue < 0)
        ? t('community.itemForm.errors.invalidPrice')
        : null,
    photoUrl:
      photoUrl.trim() && !/^https?:\/\/\S+$/i.test(photoUrl.trim())
        ? t('community.eventForm.errors.invalidUrl')
        : null,
  };
  const hasErrors = Object.values(errors).some(Boolean);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    setTouched(true);
    if (hasErrors) return;
    setSubmitError(null);
    createItem.mutate(
      {
        buildingId,
        data: {
          title: title.trim(),
          description: description.trim() || undefined,
          category,
          condition,
          isFree: isFree || undefined,
          price: !isFree && priceValue !== undefined ? priceValue.toFixed(2) : undefined,
          isNegotiable: (!isFree && isNegotiable) || undefined,
          isTradeAccepted: isTradeAccepted || undefined,
          photoUrls: photoUrl.trim() ? [photoUrl.trim()] : undefined,
          location: location.trim() || undefined,
          pickupDetails: pickupDetails.trim() || undefined,
        },
      },
      {
        onSuccess: (item) => {
          showToast({
            type: 'success',
            title: t('common.success'),
            message: t('community.hub.itemCreated'),
          });
          navigate(withBuilding(`/community/marketplace/${item.id}`, buildingId));
        },
        onError: (err) => {
          const message = getErrorMessage(err);
          setSubmitError(message);
          showToast({ type: 'error', title: t('community.hub.createItemFailed'), message });
        },
      }
    );
  };

  const inputClass =
    'mt-1 w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500';
  const err = (key: string) =>
    touched && errors[key] ? <p className="mt-1 text-sm text-red-600">{errors[key]}</p> : null;

  return (
    <form
      onSubmit={handleSubmit}
      noValidate
      className="max-w-2xl bg-white rounded-lg shadow p-6 space-y-5"
    >
      {submitError && (
        <div
          role="alert"
          className="rounded-md bg-red-50 border border-red-200 p-3 text-sm text-red-700"
        >
          {t('community.hub.createItemFailed')}: {submitError}
        </div>
      )}
      <div>
        <label htmlFor="item-title" className="block text-sm font-medium text-gray-700">
          {t('community.itemForm.titleLabel')} *
        </label>
        <input
          id="item-title"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder={t('community.itemForm.titlePlaceholder')}
          className={inputClass}
        />
        {err('title')}
      </div>
      <div>
        <label htmlFor="item-description" className="block text-sm font-medium text-gray-700">
          {t('community.itemForm.descriptionLabel')}
        </label>
        <textarea
          id="item-description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          rows={4}
          placeholder={t('community.itemForm.descriptionPlaceholder')}
          className={inputClass}
        />
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <div>
          <label htmlFor="item-category" className="block text-sm font-medium text-gray-700">
            {t('community.hub.category')}
          </label>
          <select
            id="item-category"
            value={category}
            onChange={(e) => setCategory(e.target.value as MarketplaceCategory)}
            className={inputClass}
          >
            {MARKETPLACE_CATEGORIES.map((c) => (
              <option key={c} value={c}>
                {t(`community.marketplaceCategories.${c}`)}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label htmlFor="item-condition" className="block text-sm font-medium text-gray-700">
            {t('community.hub.condition')}
          </label>
          <select
            id="item-condition"
            value={condition}
            onChange={(e) => setCondition(e.target.value as MarketplaceItemCondition)}
            className={inputClass}
          >
            {ITEM_CONDITIONS.map((c) => (
              <option key={c} value={c}>
                {t(`community.itemConditions.${c}`)}
              </option>
            ))}
          </select>
        </div>
      </div>

      <fieldset className="space-y-3">
        <legend className="text-sm font-medium text-gray-700">{t('community.hub.pricing')}</legend>
        <label className="flex items-center gap-2 text-sm text-gray-600">
          <input
            type="checkbox"
            checked={isFree}
            onChange={(e) => setIsFree(e.target.checked)}
            className="rounded border-gray-300"
          />
          {t('community.hub.giveAwayFree')}
        </label>
        {!isFree && (
          <div>
            <label htmlFor="item-price" className="block text-sm text-gray-700">
              {t('community.itemForm.priceLabel')}
            </label>
            <input
              id="item-price"
              inputMode="decimal"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
              placeholder="0.00"
              className={inputClass}
            />
            {err('price')}
            <label className="mt-2 flex items-center gap-2 text-sm text-gray-600">
              <input
                type="checkbox"
                checked={isNegotiable}
                onChange={(e) => setIsNegotiable(e.target.checked)}
                className="rounded border-gray-300"
              />
              {t('community.hub.negotiable')}
            </label>
          </div>
        )}
        <label className="flex items-center gap-2 text-sm text-gray-600">
          <input
            type="checkbox"
            checked={isTradeAccepted}
            onChange={(e) => setIsTradeAccepted(e.target.checked)}
            className="rounded border-gray-300"
          />
          {t('community.hub.tradeAccepted')}
        </label>
      </fieldset>

      <div>
        <label htmlFor="item-photo" className="block text-sm font-medium text-gray-700">
          {t('community.hub.photoUrl')}
        </label>
        <input
          id="item-photo"
          type="url"
          value={photoUrl}
          onChange={(e) => setPhotoUrl(e.target.value)}
          placeholder="https://"
          className={inputClass}
        />
        {err('photoUrl')}
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <div>
          <label htmlFor="item-location" className="block text-sm font-medium text-gray-700">
            {t('community.eventForm.locationLabel')}
          </label>
          <input
            id="item-location"
            value={location}
            onChange={(e) => setLocation(e.target.value)}
            className={inputClass}
          />
        </div>
        <div>
          <label htmlFor="item-pickup" className="block text-sm font-medium text-gray-700">
            {t('community.hub.pickupDetails')}
          </label>
          <input
            id="item-pickup"
            value={pickupDetails}
            onChange={(e) => setPickupDetails(e.target.value)}
            className={inputClass}
          />
        </div>
      </div>

      <div className="flex justify-end gap-3 pt-2">
        <button
          type="button"
          onClick={() => navigate(withBuilding('/community/marketplace', buildingId))}
          className="px-4 py-2 text-sm font-medium text-gray-700 border border-gray-300 rounded-md hover:bg-gray-50"
        >
          {t('community.common.cancel')}
        </button>
        <button
          type="submit"
          disabled={createItem.isPending}
          className="px-4 py-2 text-sm font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {createItem.isPending ? t('community.itemForm.creating') : t('community.itemForm.create')}
        </button>
      </div>
    </form>
  );
}
