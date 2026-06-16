/**
 * ListingCard — real-estate property card for Reality Portal.
 *
 * Renders a 4:3 photo with FEATURED/SALE/RENT badges, a heart favourite
 * button (top-right), price + per-m² callout, address, 2-line clamped title,
 * and a meta strip (rooms · m² · floor).
 *
 * Reference: reality-web/listings.html `.lcard`
 */

import type React from 'react';
import styles from './ListingCard.module.css';

export interface ListingCardListing {
  id: string;
  /** Absolute or relative URL for the main photo. If omitted a gradient placeholder shows. */
  photoUrl?: string;
  /** Alt text for the photo */
  photoAlt?: string;
  badges?: Array<'featured' | 'sale' | 'rent'>;
  isFavorite?: boolean;
  /** Formatted price string, e.g. "€380 000" */
  price: string;
  /** Formatted per-m² string, e.g. "€4 872/m²" */
  pricePerSqm?: string;
  /** Address / neighbourhood, e.g. "Ružinov, Bratislava" */
  address: string;
  /** Property title, displayed as a 2-line clamp */
  title: string;
  rooms?: string | number;
  area?: string;
  floor?: string | number;
}

export interface ListingCardProps {
  listing: ListingCardListing;
  onFavoriteToggle?: (id: string) => void;
  onClick?: (id: string) => void;
  className?: string;
}

const HeartIcon = ({ filled }: { filled: boolean }) => (
  <svg
    width="15"
    height="15"
    viewBox="0 0 24 24"
    fill={filled ? 'currentColor' : 'none'}
    stroke="currentColor"
    strokeWidth="2"
    aria-hidden="true"
  >
    <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z" />
  </svg>
);

const badgeClassMap: Record<'featured' | 'sale' | 'rent', string> = {
  featured: styles.badgeFeatured ?? '',
  sale: styles.badgeSale ?? '',
  rent: styles.badgeRent ?? '',
};

const badgeLabelMap: Record<'featured' | 'sale' | 'rent', string> = {
  featured: 'Featured',
  sale: 'Sale',
  rent: 'Rent',
};

/** ListingCard: real-estate property card. */
export const ListingCard: React.FC<ListingCardProps> = ({
  listing,
  onFavoriteToggle,
  onClick,
  className,
}) => {
  const {
    id,
    photoUrl,
    photoAlt,
    badges = [],
    isFavorite = false,
    price,
    pricePerSqm,
    address,
    title,
    rooms,
    area,
    floor,
  } = listing;

  const handleCardClick = (e: React.MouseEvent) => {
    // Don't bubble up when clicking the fav button
    const target = e.target as HTMLElement;
    if (target.closest('[data-fav]')) return;
    onClick?.(id);
  };

  const handleFavClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onFavoriteToggle?.(id);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onClick?.(id);
    }
  };

  return (
    <div
      className={[styles.card, className].filter(Boolean).join(' ')}
      onClick={handleCardClick}
      onKeyDown={handleKeyDown}
      role="button"
      tabIndex={0}
      aria-label={`${title}, ${price}`}
    >
      {/* Photo */}
      <div className={styles.photo}>
        {photoUrl && (
          <img className={styles.photoImg} src={photoUrl} alt={photoAlt ?? title} loading="lazy" />
        )}

        {/* Badges */}
        {badges.length > 0 && (
          <div className={styles.badges} aria-label="Property badges">
            {badges.map((b) => (
              <span key={b} className={[styles.badge, badgeClassMap[b]].join(' ')}>
                {badgeLabelMap[b]}
              </span>
            ))}
          </div>
        )}

        {/* Favourite */}
        <button
          className={[styles.fav, isFavorite ? styles.favActive : ''].filter(Boolean).join(' ')}
          onClick={handleFavClick}
          data-fav="true"
          aria-label={isFavorite ? 'Remove from favourites' : 'Add to favourites'}
          aria-pressed={isFavorite}
          type="button"
        >
          <HeartIcon filled={isFavorite} />
        </button>
      </div>

      {/* Card body */}
      <div className={styles.body}>
        <div className={styles.price}>
          {price}
          {pricePerSqm && <small className={styles.pricePerSqm}> · {pricePerSqm}</small>}
        </div>
        <div className={styles.address}>{address}</div>
        <div className={styles.title}>{title}</div>

        {(rooms != null || area != null || floor != null) && (
          <div className={styles.meta} aria-label="Property details">
            {rooms != null && <span className={styles.metaItem}>{rooms} rooms</span>}
            {area != null && <span className={styles.metaItem}>{area}</span>}
            {floor != null && <span className={styles.metaItem}>Floor {floor}</span>}
          </div>
        )}
      </div>
    </div>
  );
};

export default ListingCard;
