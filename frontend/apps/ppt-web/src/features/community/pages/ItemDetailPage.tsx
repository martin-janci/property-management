/**
 * ItemDetailPage — `/community/marketplace/:itemId`.
 *
 * Shows a listing (`GET /community/marketplace/{id}`) and lets a neighbour
 * contact the seller (`POST /community/marketplace/{id}/inquiries`). The
 * seller sees their own listing read-only: the server has no endpoint to edit,
 * withdraw or mark a listing sold, nor to read inquiries.
 */

import { useCreateItemInquiry, useMarketplaceItem } from '@ppt/api-client';
import { type FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router-dom';
import { useToast } from '../../../components';
import { useAuth } from '../../../contexts';
import { getErrorMessage } from '../../../lib/api';
import { itemPriceLabel } from '../components/ItemCard';
import { LoadingState } from '../components/StateViews';
import { withBuilding } from '../hooks/useCommunityBuilding';
import { formatDate, formatMoney } from '../utils/format';

export function ItemDetailPage() {
  const { itemId } = useParams<{ itemId: string }>();
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();
  const itemQuery = useMarketplaceItem(itemId);
  const inquiry = useCreateItemInquiry();
  const [message, setMessage] = useState('');
  const [offer, setOffer] = useState('');
  const [sent, setSent] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const item = itemQuery.data;
  const back = () => navigate(withBuilding('/community/marketplace', item?.buildingId));

  if (itemQuery.isLoading) return <LoadingState />;
  if (itemQuery.error || !item || !itemId) {
    return (
      <div className="max-w-4xl mx-auto px-4 py-16 text-center">
        <p className="text-gray-500">{t('community.hub.itemNotFound')}</p>
        <button
          type="button"
          onClick={back}
          className="mt-4 text-sm text-blue-600 hover:text-blue-800"
        >
          {t('common.back')}
        </button>
      </div>
    );
  }

  const locale = i18n.language;
  const isSeller = !!user?.id && item.sellerId === user.id;
  const offerValue = offer.trim() ? Number(offer.replace(',', '.')) : undefined;

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!message.trim()) {
      setFormError(t('community.hub.messageRequired'));
      return;
    }
    if (offerValue !== undefined && (!Number.isFinite(offerValue) || offerValue < 0)) {
      setFormError(t('community.itemForm.errors.invalidPrice'));
      return;
    }
    setFormError(null);
    inquiry.mutate(
      {
        itemId,
        data: {
          message: message.trim(),
          offerPrice: offerValue !== undefined ? offerValue.toFixed(2) : undefined,
        },
      },
      {
        onSuccess: () => {
          setSent(true);
          setMessage('');
          setOffer('');
          showToast({
            type: 'success',
            title: t('common.success'),
            message: t('community.hub.inquirySent'),
          });
        },
        onError: (err) =>
          showToast({
            type: 'error',
            title: t('community.hub.inquiryFailed'),
            message: getErrorMessage(err),
          }),
      }
    );
  };

  const facts: { label: string; value: string }[] = [
    {
      label: t('community.hub.category'),
      value: t(`community.marketplaceCategories.${item.category}`),
    },
    { label: t('community.hub.condition'), value: t(`community.itemConditions.${item.condition}`) },
  ];
  if (item.location)
    facts.push({ label: t('community.eventForm.locationLabel'), value: item.location });
  if (item.pickupDetails)
    facts.push({ label: t('community.hub.pickupDetails'), value: item.pickupDetails });
  facts.push({ label: t('community.hub.listedOn'), value: formatDate(item.createdAt, locale) });

  return (
    <div className="max-w-5xl mx-auto px-4 py-8 space-y-6">
      <button type="button" onClick={back} className="text-sm text-gray-600 hover:text-gray-900">
        ← {t('community.hub.backToMarketplace')}
      </button>

      <div className="grid gap-6 lg:grid-cols-2">
        <div className="space-y-3">
          {item.photoUrls.length > 0 ? (
            item.photoUrls.map((url) => (
              <img
                key={url}
                src={url}
                alt={item.title}
                className="w-full rounded-lg object-cover"
              />
            ))
          ) : (
            <div className="h-64 rounded-lg bg-gray-100 flex items-center justify-center text-sm text-gray-400">
              {t('community.hub.noPhoto')}
            </div>
          )}
        </div>

        <div className="space-y-4">
          <div className="bg-white rounded-lg shadow p-6">
            <div className="flex items-start justify-between gap-2">
              <h1 className="text-2xl font-bold text-gray-900">{item.title}</h1>
              {item.status !== 'active' && (
                <span className="text-xs px-2 py-0.5 rounded-full bg-gray-200 text-gray-700">
                  {t(`community.itemStatus.${item.status}`, { defaultValue: item.status })}
                </span>
              )}
            </div>
            <p className="mt-2 text-2xl font-bold text-blue-700">
              {itemPriceLabel(item, locale, t)}
            </p>
            <div className="mt-2 flex flex-wrap gap-2 text-xs">
              {item.isNegotiable && (
                <span className="px-2 py-0.5 rounded-full bg-green-50 text-green-700">
                  {t('community.hub.negotiable')}
                </span>
              )}
              {item.isTradeAccepted && (
                <span className="px-2 py-0.5 rounded-full bg-purple-50 text-purple-700">
                  {t('community.hub.tradeAccepted')}
                </span>
              )}
            </div>
            {item.description && (
              <p className="mt-4 text-sm text-gray-700 whitespace-pre-wrap">{item.description}</p>
            )}
            <dl className="mt-4 grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
              {facts.map((f) => (
                <div key={f.label}>
                  <dt className="text-gray-500">{f.label}</dt>
                  <dd className="text-gray-900">{f.value}</dd>
                </div>
              ))}
            </dl>
            <p className="mt-4 text-xs text-gray-500">
              {t('community.itemCard.views', { count: item.viewCount })} ·{' '}
              {t('community.itemCard.messages', { count: item.inquiryCount })}
            </p>
          </div>

          {isSeller ? (
            <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 text-sm text-blue-800">
              {t('community.hub.ownListing')}
            </div>
          ) : item.status !== 'active' ? (
            <div className="bg-gray-50 border border-gray-200 rounded-lg p-4 text-sm text-gray-700">
              {t('community.hub.listingUnavailable')}
            </div>
          ) : (
            <form
              onSubmit={handleSubmit}
              noValidate
              className="bg-white rounded-lg shadow p-6 space-y-3"
            >
              <h2 className="text-lg font-semibold text-gray-900">
                {t('community.itemCard.contactSeller')}
              </h2>
              {sent && (
                <p role="status" className="text-sm text-green-700">
                  {t('community.hub.inquirySent')}
                </p>
              )}
              <label htmlFor="inquiry-message" className="block text-sm text-gray-700">
                {t('community.hub.message')} *
              </label>
              <textarea
                id="inquiry-message"
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                rows={4}
                placeholder={t('community.hub.messagePlaceholder')}
                className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md"
              />
              {!item.isFree && (
                <>
                  <label htmlFor="inquiry-offer" className="block text-sm text-gray-700">
                    {t('community.hub.offerPrice')}
                  </label>
                  <input
                    id="inquiry-offer"
                    inputMode="decimal"
                    value={offer}
                    onChange={(e) => setOffer(e.target.value)}
                    placeholder={formatMoney(item.price, item.currency, locale) ?? '0.00'}
                    className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md"
                  />
                </>
              )}
              {formError && <p className="text-sm text-red-600">{formError}</p>}
              <button
                type="submit"
                disabled={inquiry.isPending}
                className="w-full px-4 py-2 text-sm font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
              >
                {inquiry.isPending ? t('community.hub.sending') : t('community.hub.sendMessage')}
              </button>
            </form>
          )}
        </div>
      </div>
    </div>
  );
}
