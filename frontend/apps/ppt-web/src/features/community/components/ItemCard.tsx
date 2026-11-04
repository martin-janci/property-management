/**
 * ItemCard — a marketplace listing tile linking to its detail page.
 */

import type { MarketplaceItem } from '@ppt/api-client';
import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';
import { formatMoney } from '../utils/format';

export function itemPriceLabel(item: MarketplaceItem, locale: string, t: TFunction): string {
  if (item.isFree || item.category === 'free') return t('community.itemCard.price.free');
  if (item.category === 'wanted') return t('community.itemCard.price.wanted');
  return formatMoney(item.price, item.currency, locale) ?? t('community.itemCard.price.contact');
}

interface ItemCardProps {
  item: MarketplaceItem;
  onOpen: (id: string) => void;
}

export function ItemCard({ item, onOpen }: ItemCardProps) {
  const { t, i18n } = useTranslation();
  const photo = item.photoUrls[0];

  return (
    <article className="bg-white rounded-lg shadow overflow-hidden flex flex-col">
      <div className="h-40 bg-gray-100 flex items-center justify-center">
        {photo ? (
          <img src={photo} alt={item.title} className="w-full h-full object-cover" />
        ) : (
          <span className="text-sm text-gray-400">{t('community.hub.noPhoto')}</span>
        )}
      </div>
      <div className="p-4 flex flex-col flex-1">
        <div className="flex items-start justify-between gap-2">
          <h3 className="text-base font-semibold text-gray-900 line-clamp-2">{item.title}</h3>
          {item.status !== 'active' && (
            <span className="text-xs px-2 py-0.5 rounded-full bg-gray-200 text-gray-700 shrink-0">
              {t(`community.itemStatus.${item.status}`, { defaultValue: item.status })}
            </span>
          )}
        </div>
        <p className="mt-1 text-lg font-bold text-blue-700">
          {itemPriceLabel(item, i18n.language, t)}
        </p>
        <p className="mt-1 text-xs text-gray-500">
          {t(`community.marketplaceCategories.${item.category}`)} ·{' '}
          {t(`community.itemConditions.${item.condition}`)}
        </p>
        <div className="mt-auto pt-3">
          <button
            type="button"
            onClick={() => onOpen(item.id)}
            className="w-full px-3 py-2 text-sm font-medium text-blue-600 border border-blue-200 rounded-md hover:bg-blue-50"
          >
            {t('community.itemCard.viewDetails')}
          </button>
        </div>
      </div>
    </article>
  );
}
