/**
 * GroupMemberList Component
 *
 * Displays a list of group members with role badges.
 * Part of Story 42.1: Community Groups.
 */

import type { GroupMember, GroupMemberRole } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface GroupMemberListProps {
  members: GroupMember[];
  currentUserId?: string;
  canManageMembers?: boolean;
  onPromote?: (memberId: string, newRole: GroupMemberRole) => void;
  onRemove?: (memberId: string) => void;
  onBan?: (memberId: string) => void;
  isLoading?: boolean;
}

const roleColors: Record<GroupMemberRole, string> = {
  owner: 'bg-purple-100 text-purple-800',
  admin: 'bg-blue-100 text-blue-800',
  moderator: 'bg-green-100 text-green-800',
  member: 'bg-gray-100 text-gray-800',
};

const roleKeys: Record<GroupMemberRole, string> = {
  owner: 'community.groupMembers.roles.owner',
  admin: 'community.groupMembers.roles.admin',
  moderator: 'community.groupMembers.roles.moderator',
  member: 'community.groupMembers.roles.member',
};

export function GroupMemberList({
  members,
  currentUserId,
  canManageMembers,
  onPromote,
  onRemove,
  onBan,
  isLoading,
}: GroupMemberListProps) {
  const { t } = useTranslation();
  if (isLoading) {
    const skeletonKeys = ['skeleton-1', 'skeleton-2', 'skeleton-3', 'skeleton-4', 'skeleton-5'];
    return (
      <div className="space-y-3">
        {skeletonKeys.map((key) => (
          <div
            key={key}
            className="flex items-center gap-3 p-3 bg-white rounded-lg shadow animate-pulse"
          >
            <div className="w-10 h-10 rounded-full bg-gray-200" />
            <div className="flex-1 space-y-2">
              <div className="h-4 bg-gray-200 rounded w-1/3" />
              <div className="h-3 bg-gray-200 rounded w-1/4" />
            </div>
          </div>
        ))}
      </div>
    );
  }

  if (members.length === 0) {
    return (
      <div className="bg-white rounded-lg shadow p-6 text-center">
        <svg
          className="mx-auto h-12 w-12 text-gray-400"
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
        <p className="mt-4 text-sm text-gray-500">{t('community.groupMembers.empty')}</p>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {members.map((member) => {
        const isCurrentUser = member.userId === currentUserId;
        const canModifyMember = canManageMembers && !isCurrentUser && member.role !== 'owner';

        return (
          <div
            key={member.id}
            className="flex items-center gap-3 p-3 bg-white rounded-lg shadow hover:shadow-md transition-shadow"
          >
            {/* Avatar */}
            <div className="relative">
              {member.userAvatar ? (
                <img
                  src={member.userAvatar}
                  alt={member.userName}
                  className="w-10 h-10 rounded-full object-cover"
                />
              ) : (
                <div className="w-10 h-10 rounded-full bg-gray-300 flex items-center justify-center">
                  <span className="text-gray-600 font-medium text-sm">
                    {member.userName.charAt(0).toUpperCase()}
                  </span>
                </div>
              )}
              {member.status === 'pending' && (
                <span
                  className="absolute -top-1 -right-1 w-3 h-3 bg-yellow-400 rounded-full border-2 border-white"
                  title={t('community.groupMembers.pendingApproval')}
                />
              )}
            </div>

            {/* Info */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="font-medium text-gray-900 truncate">
                  {member.userName}
                  {isCurrentUser && (
                    <span className="text-gray-500 ml-1">{t('community.groupMembers.you')}</span>
                  )}
                </span>
                <span
                  className={`px-2 py-0.5 text-xs font-medium rounded ${roleColors[member.role]}`}
                >
                  {t(roleKeys[member.role])}
                </span>
              </div>
              <p className="text-xs text-gray-500">
                {t('community.groupMembers.joined', {
                  date: new Date(member.joinedAt).toLocaleDateString(),
                })}
              </p>
            </div>

            {/* Actions */}
            {canModifyMember && (
              <div className="flex items-center gap-1">
                {member.role === 'member' && (
                  <button
                    type="button"
                    onClick={() => onPromote?.(member.id, 'moderator')}
                    className="p-1 text-gray-400 hover:text-blue-600 rounded"
                    title={t('community.groupMembers.makeModerator')}
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
                        d="M5 10l7-7m0 0l7 7m-7-7v18"
                      />
                    </svg>
                  </button>
                )}
                {member.role === 'moderator' && (
                  <button
                    type="button"
                    onClick={() => onPromote?.(member.id, 'admin')}
                    className="p-1 text-gray-400 hover:text-blue-600 rounded"
                    title={t('community.groupMembers.makeAdmin')}
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
                        d="M5 10l7-7m0 0l7 7m-7-7v18"
                      />
                    </svg>
                  </button>
                )}
                <button
                  type="button"
                  onClick={() => onRemove?.(member.id)}
                  className="p-1 text-gray-400 hover:text-red-600 rounded"
                  title={t('community.groupMembers.removeFromGroup')}
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
                      d="M6 18L18 6M6 6l12 12"
                    />
                  </svg>
                </button>
                <button
                  type="button"
                  onClick={() => onBan?.(member.id)}
                  className="p-1 text-gray-400 hover:text-red-600 rounded"
                  title={t('community.groupMembers.banFromGroup')}
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
                      d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"
                    />
                  </svg>
                </button>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
