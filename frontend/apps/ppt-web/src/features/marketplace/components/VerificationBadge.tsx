/**
 * VerificationBadge component - displays provider verification badges.
 * Epic 68: Service Provider Marketplace (Story 68.4)
 */

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

const badgeConfig: Record<
  BadgeType,
  { label: string; icon: string; bgColor: string; textColor: string }
> = {
  verified_business: {
    label: 'Verified Business',
    icon: 'M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z',
    bgColor: 'bg-green-100',
    textColor: 'text-green-800',
  },
  insured: {
    label: 'Insured',
    icon: 'M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z',
    bgColor: 'bg-blue-100',
    textColor: 'text-blue-800',
  },
  certified: {
    label: 'Certified',
    icon: 'M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z',
    bgColor: 'bg-purple-100',
    textColor: 'text-purple-800',
  },
  top_rated: {
    label: 'Top Rated',
    icon: 'M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z',
    bgColor: 'bg-yellow-100',
    textColor: 'text-yellow-800',
  },
  fast_responder: {
    label: 'Fast Responder',
    icon: 'M13 10V3L4 14h7v7l9-11h-7z',
    bgColor: 'bg-cyan-100',
    textColor: 'text-cyan-800',
  },
  preferred: {
    label: 'Preferred',
    icon: 'M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z',
    bgColor: 'bg-orange-100',
    textColor: 'text-orange-800',
  },
};

const verificationStatusConfig: Record<
  VerificationStatus,
  { label: string; bgColor: string; textColor: string }
> = {
  pending: { label: 'Pending', bgColor: 'bg-gray-100', textColor: 'text-gray-800' },
  under_review: { label: 'Under Review', bgColor: 'bg-yellow-100', textColor: 'text-yellow-800' },
  verified: { label: 'Verified', bgColor: 'bg-green-100', textColor: 'text-green-800' },
  rejected: { label: 'Rejected', bgColor: 'bg-red-100', textColor: 'text-red-800' },
  expired: { label: 'Expired', bgColor: 'bg-orange-100', textColor: 'text-orange-800' },
};

const verificationTypeLabels: Record<VerificationType, string> = {
  business_registration: 'Business Registration',
  insurance: 'Insurance',
  certification: 'Certification',
  license: 'License',
  identity: 'Identity',
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
  const config = badgeConfig[badge.type];
  const classes = sizeClasses[size];

  const msUntilExpiry = badge.expiresAt
    ? new Date(badge.expiresAt).getTime() - new Date().getTime()
    : undefined;
  // Already past the expiry instant → expired, not "expiring soon".
  const isExpired = msUntilExpiry !== undefined && msUntilExpiry <= 0;
  // Only a not-yet-expired badge inside the 30-day window is "expiring soon".
  const isExpiring =
    msUntilExpiry !== undefined && msUntilExpiry > 0 && msUntilExpiry < 30 * 24 * 60 * 60 * 1000;

  const statusSuffix = isExpired ? ' (Expired)' : isExpiring ? ' (Expiring soon)' : '';
  const ringClass = isExpired ? 'ring-2 ring-red-400' : isExpiring ? 'ring-2 ring-orange-300' : '';

  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full font-medium ${config.bgColor} ${config.textColor} ${classes.badge} ${ringClass}`}
      title={showTooltip ? `${config.label}${statusSuffix}` : undefined}
    >
      <svg
        className={classes.icon}
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        strokeWidth={2}
      >
        <title>{config.label}</title>
        <path strokeLinecap="round" strokeLinejoin="round" d={config.icon} />
      </svg>
      {config.label}
    </span>
  );
}

export function VerificationStatusBadge({
  verification,
  size = 'md',
}: VerificationStatusBadgeProps) {
  const config = verificationStatusConfig[verification.status];
  const classes = sizeClasses[size];
  const typeLabel = verificationTypeLabels[verification.type];

  const msUntilExpiry = verification.expiryDate
    ? new Date(verification.expiryDate).getTime() - new Date().getTime()
    : undefined;
  // "Soon" only for a still-valid verification inside the 30-day window —
  // an already-expired date (msUntilExpiry <= 0) must not read as expiring soon.
  const isExpiring =
    verification.status === 'verified' &&
    msUntilExpiry !== undefined &&
    msUntilExpiry > 0 &&
    msUntilExpiry < 30 * 24 * 60 * 60 * 1000;

  return (
    <div className="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
      <div>
        <p className="font-medium text-gray-900">{typeLabel}</p>
        <p className="text-sm text-gray-500">{verification.documentName}</p>
        {verification.expiryDate && (
          <p className={`text-xs ${isExpiring ? 'text-orange-600' : 'text-gray-400'}`}>
            Expires: {new Date(verification.expiryDate).toLocaleDateString()}
            {isExpiring && ' (Soon)'}
          </p>
        )}
      </div>
      <span
        className={`inline-flex items-center rounded-full font-medium ${config.bgColor} ${config.textColor} ${classes.badge}`}
      >
        {config.label}
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
          +{remainingCount} more
        </span>
      )}
    </div>
  );
}

interface VerificationListProps {
  verifications: Verification[];
}

export function VerificationList({ verifications }: VerificationListProps) {
  if (verifications.length === 0) {
    return (
      <div className="text-center py-8 bg-gray-50 rounded-lg">
        <svg
          className="mx-auto w-12 h-12 text-gray-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <title>No verifications</title>
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
          />
        </svg>
        <p className="mt-2 text-gray-500">No verifications submitted yet</p>
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
