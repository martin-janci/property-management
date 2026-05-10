/**
 * GroupCard Component
 *
 * Displays a community group summary card with join/leave actions.
 * Part of Story 42.1: Community Groups.
 */

import type { CommunityGroupSummary, GroupVisibility } from '@ppt/api-client';

interface GroupCardProps {
  group: CommunityGroupSummary;
  onView?: (id: string) => void;
  onJoin?: (id: string) => void;
  onLeave?: (id: string) => void;
  isJoining?: boolean;
  isLeaving?: boolean;
}

const visibilityIcons: Record<GroupVisibility, React.JSX.Element> = {
  public: (
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
        d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
      />
    </svg>
  ),
  private: (
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
        d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
      />
    </svg>
  ),
  hidden: (
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
        d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"
      />
    </svg>
  ),
};

const visibilityLabels: Record<GroupVisibility, string> = {
  public: 'Public',
  private: 'Private',
  hidden: 'Hidden',
};

export function GroupCard({
  group,
  onView,
  onJoin,
  onLeave,
  isJoining,
  isLeaving,
}: GroupCardProps) {
  return (
    <div className="bg-white rounded-lg shadow overflow-hidden hover:shadow-md transition-shadow">
      {/* Cover Image */}
      <div className="h-32 bg-gradient-to-r from-blue-500 to-purple-600 relative">
        {group.coverImageUrl && (
          <img
            src={group.coverImageUrl}
            alt={`${group.name} cover`}
            className="w-full h-full object-cover"
          />
        )}
        {group.isOfficial && (
          <span className="absolute top-2 right-2 bg-blue-600 text-white text-xs px-2 py-1 rounded-full flex items-center gap-1">
            <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
              <path
                fillRule="evenodd"
                d="M6.267 3.455a3.066 3.066 0 001.745-.723 3.066 3.066 0 013.976 0 3.066 3.066 0 001.745.723 3.066 3.066 0 012.812 2.812c.051.643.304 1.254.723 1.745a3.066 3.066 0 010 3.976 3.066 3.066 0 00-.723 1.745 3.066 3.066 0 01-2.812 2.812 3.066 3.066 0 00-1.745.723 3.066 3.066 0 01-3.976 0 3.066 3.066 0 00-1.745-.723 3.066 3.066 0 01-2.812-2.812 3.066 3.066 0 00-.723-1.745 3.066 3.066 0 010-3.976 3.066 3.066 0 00.723-1.745 3.066 3.066 0 012.812-2.812zm7.44 5.252a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                clipRule="evenodd"
              />
            </svg>
            Official
          </span>
        )}
      </div>

      {/* Content */}
      <div className="p-4">
        <div className="flex items-start justify-between">
          <div className="flex-1 min-w-0">
            <h3 className="text-lg font-semibold text-gray-900 truncate">{group.name}</h3>
            <div className="flex items-center gap-2 mt-1 text-sm text-gray-500">
              <span className="flex items-center gap-1">
                {visibilityIcons[group.visibility]}
                {visibilityLabels[group.visibility]}
              </span>
              <span>•</span>
              <span>{group.category}</span>
            </div>
          </div>
        </div>

        <p className="mt-2 text-sm text-gray-600 line-clamp-2">{group.description}</p>

        <div className="mt-4 flex items-center justify-between">
          <div className="flex items-center text-sm text-gray-500">
            <svg
              className="w-4 h-4 mr-1"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z"
              />
            </svg>
            {group.memberCount} {group.memberCount === 1 ? 'member' : 'members'}
          </div>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => onView?.(group.id)}
              className="text-sm text-blue-600 hover:text-blue-800 font-medium"
            >
              View
            </button>
            {group.isMember ? (
              <button
                type="button"
                onClick={() => onLeave?.(group.id)}
                disabled={isLeaving}
                className="px-3 py-1 text-sm border border-red-300 text-red-600 rounded-md hover:bg-red-50 disabled:opacity-50"
              >
                {isLeaving ? 'Leaving...' : 'Leave'}
              </button>
            ) : (
              <button
                type="button"
                onClick={() => onJoin?.(group.id)}
                disabled={isJoining}
                className="px-3 py-1 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
              >
                {isJoining ? 'Joining...' : 'Join'}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
