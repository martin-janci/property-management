/**
 * ItemCard Component
 *
 * Displays a marketplace item listing card.
 * Part of Story 42.4: Item Marketplace.
 */

import type { ItemCondition, ItemListingType, ItemStatus, MarketplaceItem } from '@ppt/api-client';
import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';

interface ItemCardProps {
  item: MarketplaceItem;
  onView?: (id: string) => void;
  onContact?: (id: string) => void;
  onEdit?: (id: string) => void;
  onDelete?: (id: string) => void;
  onMarkSold?: (id: string) => void;
  isCurrentUser?: boolean;
}

const conditionKeys: Record<ItemCondition, string> = {
  new: 'community.marketplace.condition.new',
  like_new: 'community.marketplace.condition.likeNew',
  good: 'community.marketplace.condition.good',
  fair: 'community.marketplace.condition.fair',
  poor: 'community.marketplace.condition.poor',
};

const conditionColors: Record<ItemCondition, string> = {
  new: 'bg-green-100 text-green-800',
  like_new: 'bg-blue-100 text-blue-800',
  good: 'bg-gray-100 text-gray-800',
  fair: 'bg-yellow-100 text-yellow-800',
  poor: 'bg-red-100 text-red-800',
};

const listingTypeKeys: Record<ItemListingType, string> = {
  sale: 'community.marketplace.listingType.sale',
  free: 'community.marketplace.listingType.free',
  wanted: 'community.marketplace.listingType.wanted',
  trade: 'community.marketplace.listingType.trade',
};

const listingTypeColors: Record<ItemListingType, string> = {
  sale: 'bg-green-600 text-white',
  free: 'bg-blue-600 text-white',
  wanted: 'bg-purple-600 text-white',
  trade: 'bg-orange-600 text-white',
};

const statusColors: Record<ItemStatus, string> = {
  active: '',
  reserved: 'bg-yellow-50 border-yellow-200',
  sold: 'bg-gray-50 border-gray-200 opacity-75',
  expired: 'bg-gray-50 border-gray-200 opacity-60',
  removed: 'bg-gray-50 border-gray-200 opacity-50',
};

function formatPrice(
  price: number | undefined,
  currency: string | undefined,
  listingType: ItemListingType,
  t: TFunction
): string {
  if (listingType === 'free') return t('community.itemCard.price.free');
  if (listingType === 'wanted') return t('community.itemCard.price.wanted');
  if (listingType === 'trade') return t('community.itemCard.price.trade');
  if (price === undefined) return t('community.itemCard.price.contact');
  return new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency: currency || 'EUR',
  }).format(price);
}

function formatTimeAgo(dateString: string, t: TFunction): string {
  const date = new Date(dateString);
  const now = new Date();
  const seconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (seconds < 3600)
    return t('community.itemCard.minutesAgo', { count: Math.floor(seconds / 60) });
  if (seconds < 86400)
    return t('community.itemCard.hoursAgo', { count: Math.floor(seconds / 3600) });
  if (seconds < 604800)
    return t('community.itemCard.daysAgo', { count: Math.floor(seconds / 86400) });
  return date.toLocaleDateString();
}

export function ItemCard({
  item,
  onView,
  onContact,
  onEdit,
  onDelete,
  onMarkSold,
  isCurrentUser,
}: ItemCardProps) {
  const { t } = useTranslation();
  const isAvailable = item.status === 'active';
  const isReserved = item.status === 'reserved';
  const isSold = item.status === 'sold';

  return (
    <div
      className={`bg-white rounded-lg shadow overflow-hidden hover:shadow-md transition-shadow border ${statusColors[item.status]}`}
    >
      {/* Image */}
      <div className="relative aspect-square bg-gray-100">
        {item.imageUrls.length > 0 ? (
          <img src={item.imageUrls[0]} alt={item.title} className="w-full h-full object-cover" />
        ) : (
          <div className="w-full h-full flex items-center justify-center">
            <svg
              className="w-16 h-16 text-gray-300"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
              />
            </svg>
          </div>
        )}
        {/* Listing Type Badge */}
        <span
          className={`absolute top-2 left-2 px-2 py-1 text-xs font-medium rounded ${listingTypeColors[item.listingType]}`}
        >
          {t(listingTypeKeys[item.listingType])}
        </span>
        {/* Image Count */}
        {item.imageUrls.length > 1 && (
          <span className="absolute bottom-2 right-2 bg-black bg-opacity-60 text-white text-xs px-2 py-1 rounded flex items-center gap-1">
            <svg
              className="w-3 h-3"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
              />
            </svg>
            {item.imageUrls.length}
          </span>
        )}
        {/* Status Overlay */}
        {!isAvailable && (
          <div className="absolute inset-0 bg-black bg-opacity-40 flex items-center justify-center">
            <span className="bg-white text-gray-800 px-3 py-1 rounded-full text-sm font-medium">
              {isReserved
                ? t('community.itemCard.reserved')
                : isSold
                  ? t('community.itemCard.sold')
                  : item.status}
            </span>
          </div>
        )}
      </div>

      {/* Content */}
      <div className="p-4">
        {/* Price */}
        <div className="text-lg font-bold text-gray-900">
          {formatPrice(item.price, item.currency, item.listingType, t)}
        </div>

        {/* Title */}
        <button
          type="button"
          className="mt-1 font-medium text-gray-900 truncate cursor-pointer hover:text-blue-600 text-left w-full"
          onClick={() => onView?.(item.id)}
        >
          {item.title}
        </button>

        {/* Category & Condition */}
        <div className="mt-2 flex items-center gap-2 text-xs">
          <span className="text-gray-500">{item.category}</span>
          <span className="text-gray-300">•</span>
          <span className={`px-1.5 py-0.5 rounded ${conditionColors[item.condition]}`}>
            {t(conditionKeys[item.condition])}
          </span>
        </div>

        {/* Seller Info */}
        <div className="mt-3 flex items-center gap-2">
          {item.sellerAvatar ? (
            <img
              src={item.sellerAvatar}
              alt={item.sellerName}
              className="w-6 h-6 rounded-full object-cover"
            />
          ) : (
            <div className="w-6 h-6 rounded-full bg-gray-300 flex items-center justify-center">
              <span className="text-gray-600 text-xs font-medium">
                {item.sellerName.charAt(0).toUpperCase()}
              </span>
            </div>
          )}
          <div className="flex-1 min-w-0">
            <span className="text-sm text-gray-600 truncate">{item.sellerName}</span>
            {item.sellerUnit && (
              <span className="text-xs text-gray-400 ml-1">• {item.sellerUnit}</span>
            )}
          </div>
        </div>

        {/* Stats */}
        <div className="mt-2 flex items-center gap-3 text-xs text-gray-400">
          <span>{formatTimeAgo(item.createdAt, t)}</span>
          <span>•</span>
          <span>{t('community.itemCard.views', { count: item.viewCount })}</span>
          {item.messageCount > 0 && (
            <>
              <span>•</span>
              <span>{t('community.itemCard.messages', { count: item.messageCount })}</span>
            </>
          )}
        </div>
      </div>

      {/* Actions */}
      <div className="px-4 py-3 bg-gray-50 border-t flex items-center justify-between">
        {isCurrentUser ? (
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => onEdit?.(item.id)}
              className="text-sm text-gray-600 hover:text-gray-800"
            >
              {t('community.common.edit')}
            </button>
            {isAvailable && (
              <button
                type="button"
                onClick={() => onMarkSold?.(item.id)}
                className="text-sm text-green-600 hover:text-green-800"
              >
                {t('community.itemCard.markSold')}
              </button>
            )}
            <button
              type="button"
              onClick={() => onDelete?.(item.id)}
              className="text-sm text-red-600 hover:text-red-800"
            >
              {t('community.common.delete')}
            </button>
          </div>
        ) : (
          <button
            type="button"
            onClick={() => onContact?.(item.id)}
            disabled={!isAvailable && !isReserved}
            className="flex-1 inline-flex items-center justify-center gap-2 px-4 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <svg
              className="w-4 h-4"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
              />
            </svg>
            {t('community.itemCard.contactSeller')}
          </button>
        )}
        <button
          type="button"
          onClick={() => onView?.(item.id)}
          className="p-2 text-gray-400 hover:text-gray-600 rounded"
          title={t('community.itemCard.viewDetails')}
        >
          <svg
            className="w-5 h-5"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
            />
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
            />
          </svg>
        </button>
      </div>
    </div>
  );
}
