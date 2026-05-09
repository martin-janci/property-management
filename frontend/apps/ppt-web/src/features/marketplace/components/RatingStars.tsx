/**
 * RatingStars component - displays and inputs star ratings.
 * Epic 68: Service Provider Marketplace (Story 68.5)
 */

interface RatingStarsProps {
  rating: number;
  size?: 'sm' | 'md' | 'lg';
  showValue?: boolean;
}

const sizeClasses = {
  sm: 'w-4 h-4',
  md: 'w-5 h-5',
  lg: 'w-6 h-6',
};

const textSizeClasses = {
  sm: 'text-sm',
  md: 'text-base',
  lg: 'text-lg',
};

export function RatingStars({ rating, size = 'md', showValue = false }: RatingStarsProps) {
  const fullStars = Math.floor(rating);
  const hasHalfStar = rating % 1 >= 0.5;
  const starSize = sizeClasses[size];
  const textSize = textSizeClasses[size];

  return (
    <div className="flex items-center gap-1">
      <div className="flex" aria-label={`${rating} out of 5 stars`}>
        {[...Array(5)].map((_, i) => {
          if (i < fullStars) {
            // Full star
            return (
              <svg
                key={`star-${i}`}
                className={`${starSize} text-yellow-400`}
                fill="currentColor"
                viewBox="0 0 20 20"
              >
                <title>Filled star</title>
                <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
              </svg>
            );
          }
          if (i === fullStars && hasHalfStar) {
            // Half star
            return (
              <svg key={`star-${i}`} className={`${starSize} text-yellow-400`} viewBox="0 0 20 20">
                <title>Half-filled star</title>
                <defs>
                  <linearGradient id={`half-gradient-${i}`}>
                    <stop offset="50%" stopColor="currentColor" />
                    <stop offset="50%" stopColor="var(--ppt-border-strong)" />
                  </linearGradient>
                </defs>
                <path
                  fill={`url(#half-gradient-${i})`}
                  d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z"
                />
              </svg>
            );
          }
          // Empty star
          return (
            <svg
              key={`star-${i}`}
              className={`${starSize} text-gray-300`}
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <title>Empty star</title>
              <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
            </svg>
          );
        })}
      </div>
      {showValue && (
        <span className={`${textSize} font-medium text-gray-700 ml-1`}>{rating.toFixed(1)}</span>
      )}
    </div>
  );
}

interface RatingInputProps {
  value: number;
  onChange: (value: number) => void;
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
}

export function RatingInput({ value, onChange, size = 'md', disabled = false }: RatingInputProps) {
  const starSize = sizeClasses[size];

  const ratingLabels = ['Poor', 'Fair', 'Good', 'Very Good', 'Excellent'];

  return (
    <div className="flex items-center gap-2">
      <div className="flex gap-1">
        {[1, 2, 3, 4, 5].map((star) => (
          <button
            key={star}
            type="button"
            onClick={() => !disabled && onChange(star)}
            disabled={disabled}
            className={`${disabled ? 'cursor-not-allowed' : 'cursor-pointer'} focus:outline-none focus:ring-2 focus:ring-yellow-400 focus:ring-offset-2 rounded-sm`}
            aria-label={`Rate ${star} star${star !== 1 ? 's' : ''}`}
          >
            <svg
              className={`${starSize} ${
                star <= value ? 'text-yellow-400' : 'text-gray-300'
              } ${!disabled && 'hover:text-yellow-300'} transition-colors`}
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <title>{star <= value ? 'Selected' : 'Not selected'}</title>
              <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
            </svg>
          </button>
        ))}
      </div>
      {value > 0 && <span className="text-sm text-gray-600">{ratingLabels[value - 1]}</span>}
    </div>
  );
}
