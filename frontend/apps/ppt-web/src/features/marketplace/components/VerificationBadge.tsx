/**
 * VerificationBadge component - displays provider verification badges.
 * Epic 68: Service Provider Marketplace (Story 68.4)
 */

import { useTranslation } from 'react-i18next';

export type VerificationType =
  | 'business_registration'
  | 'insurance'
  | 'certification'
  | 'license'
  | 'identity';

export type VerificationStatus = 'pending' | 'under_review' | 'verified' | 'rejected' | 'expired';

export type BadgeType =
  | 'verified_business'
  | 'insured'
  | 'certified'
  | 'top_rated'
  | 'fast_responder'
  | 'preferred';

export interface Verification {
  id: string;
  type: VerificationType;
  documentName: string;
  status: VerificationStatus;
  expiryDate?: string;
  verifiedAt?: string;
}

export interface Badge {
  type: BadgeType;
  awardedAt: string;
  expiresAt?: string;
}

interface VerificationBadgeProps {
  badge: Badge;
  size?: 'sm' | 'md' | 'lg';
  showTooltip?: boolean;
}

interface VerificationStatusBadgeProps {
  verification: Verification;
  size?: 'sm' | 'md' | 'lg';
}

/**
 * A trust signal is "expiring soon" only inside this window before its expiry
 * instant. Shared by {@link VerificationBadge} and {@link VerificationStatusBadge}
 * via {@link getExpiryState} so the threshold lives in exactly one place.
 */
const EXPIRING_SOON_WINDOW_MS = 30 * 24 * 60 * 60 * 1000;

export type ExpiryState = 'valid' | 'expiring' | 'expired';

/**
 * Pure classifier for a verification/badge expiry timestamp.
 *
 * - no `expiresAt`         → `'valid'` (nothing to expire)
 * - already past the instant → `'expired'` (must NOT read as "expiring soon")
 * - inside the 30-day window → `'expiring'`
 * - otherwise               → `'valid'`
 *
 * Extracted so both badge components share one source of truth for the
 * expired/expiring boundaries (issue #2824 — the logic was duplicated inline
 * and could drift).
 */
export function getExpiryState(expiresAt?: string, now: number = Date.now()): ExpiryState {
  if (!expiresAt) return 'valid';
  const msUntilExpiry = new Date(expiresAt).getTime() - now;
  if (msUntilExpiry <= 0) return 'expired';
  if (msUntilExpiry < EXPIRING_SOON_WINDOW_MS) return 'expiring';
  return 'valid';
}

const badgeConfig: Record<BadgeType, { icon: string; bgColor: string; textColor: string }> = {
  verified_business: {
    icon: 'M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z',
    bgColor: 'bg-green-100',
    textColor: 'text-green-800',
  },
  insured: {
    icon: 'M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z',
    bgColor: 'bg-blue-100',
    textColor: 'text-blue-800',
  },
  certified: {
    icon: 'M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z',
    bgColor: 'bg-purple-100',
    textColor: 'text-purple-800',
  },
  top_rated: {
    icon: 'M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z',
    bgColor: 'bg-yellow-100',
    textColor: 'text-yellow-800',
  },
  fast_responder: {
    icon: 'M13 10V3L4 14h7v7l9-11h-7z',
    bgColor: 'bg-cyan-100',
    textColor: 'text-cyan-800',
  },
  preferred: {
    icon: 'M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z',
    bgColor: 'bg-orange-100',
    textColor: 'text-orange-800',
  },
};

const verificationStatusColors: Record<VerificationStatus, { bgColor: string; textColor: string }> =
  {
    pending: { bgColor: 'bg-gray-100', textColor: 'text-gray-800' },
    under_review: { bgColor: 'bg-yellow-100', textColor: 'text-yellow-800' },
    verified: { bgColor: 'bg-green-100', textColor: 'text-green-800' },
    rejected: { bgColor: 'bg-red-100', textColor: 'text-red-800' },
    expired: { bgColor: 'bg-orange-100', textColor: 'text-orange-800' },
  };

const sizeClasses = {
  sm: { badge: 'px-2 py-0.5 text-xs', icon: 'w-3 h-3' },
  md: { badge: 'px-2.5 py-1 text-sm', icon: 'w-4 h-4' },
  lg: { badge: 'px-3 py-1.5 text-base', icon: 'w-5 h-5' },
};

export function VerificationBadge({
  badge,
  size = 'md',
  showTooltip = true,
}: VerificationBadgeProps) {
  const { t } = useTranslation();
  const config = badgeConfig[badge.type];
  const classes = sizeClasses[size];
  const label = t(`marketplace.verificationBadge.badge.${badge.type}`);

  const expiryState = getExpiryState(badge.expiresAt);
  const statusSuffix =
    expiryState === 'expired'
      ? ` (${t('marketplace.verificationBadge.expired')})`
      : expiryState === 'expiring'
        ? ` (${t('marketplace.verificationBadge.expiringSoon')})`
        : '';
  const ringClass =
    expiryState === 'expired'
      ? 'ring-2 ring-red-400'
      : expiryState === 'expiring'
        ? 'ring-2 ring-orange-300'
        : '';

  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full font-medium ${config.bgColor} ${config.textColor} ${classes.badge} ${ringClass}`}
      title={showTooltip ? `${label}${statusSuffix}` : undefined}
    >
      <svg
        className={classes.icon}
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        strokeWidth={2}
      >
        <title>{label}</title>
        <path strokeLinecap="round" strokeLinejoin="round" d={config.icon} />
      </svg>
      {label}
    </span>
  );
}

export function VerificationStatusBadge({
  verification,
  size = 'md',
}: VerificationStatusBadgeProps) {
  const { t } = useTranslation();
  const config = verificationStatusColors[verification.status];
  const classes = sizeClasses[size];
  const typeLabel = t(`marketplace.verificationBadge.type.${verification.type}`);
  const statusLabel = t(`marketplace.verificationBadge.status.${verification.status}`);

  // "Soon" only for a still-valid verification inside the 30-day window — an
  // already-expired date must not read as expiring soon (see getExpiryState).
  const isExpiring =
    verification.status === 'verified' && getExpiryState(verification.expiryDate) === 'expiring';

  return (
    <div className="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
      <div>
        <p className="font-medium text-gray-900">{typeLabel}</p>
        <p className="text-sm text-gray-500">{verification.documentName}</p>
        {verification.expiryDate && (
          <p className={`text-xs ${isExpiring ? 'text-orange-600' : 'text-gray-400'}`}>
            {t('marketplace.verificationBadge.expires')}:{' '}
            {new Date(verification.expiryDate).toLocaleDateString()}
            {isExpiring && ` (${t('marketplace.verificationBadge.soon')})`}
          </p>
        )}
      </div>
      <span
        className={`inline-flex items-center rounded-full font-medium ${config.bgColor} ${config.textColor} ${classes.badge}`}
      >
        {statusLabel}
      </span>
    </div>
  );
}

interface BadgeListProps {
  badges: Badge[];
  size?: 'sm' | 'md' | 'lg';
  maxDisplay?: number;
}

export function BadgeList({ badges, size = 'md', maxDisplay = 5 }: BadgeListProps) {
  const { t } = useTranslation();
  const displayedBadges = badges.slice(0, maxDisplay);
  const remainingCount = badges.length - maxDisplay;

  if (badges.length === 0) {
    return null;
  }

  return (
    <div className="flex flex-wrap gap-2">
      {displayedBadges.map((badge) => (
        <VerificationBadge key={badge.type} badge={badge} size={size} />
      ))}
      {remainingCount > 0 && (
        <span className="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-gray-100 text-gray-600">
          {t('marketplace.verificationBadge.moreCount', { count: remainingCount })}
        </span>
      )}
    </div>
  );
}

interface VerificationListProps {
  verifications: Verification[];
}

export function VerificationList({ verifications }: VerificationListProps) {
  const { t } = useTranslation();

  if (verifications.length === 0) {
    return (
      <div className="text-center py-8 bg-gray-50 rounded-lg">
        <svg
          className="mx-auto w-12 h-12 text-gray-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <title>{t('marketplace.verificationBadge.noVerifications')}</title>
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
          />
        </svg>
        <p className="mt-2 text-gray-500">{t('marketplace.verificationBadge.noVerifications')}</p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {verifications.map((verification) => (
        <VerificationStatusBadge key={verification.id} verification={verification} />
      ))}
    </div>
  );
}
