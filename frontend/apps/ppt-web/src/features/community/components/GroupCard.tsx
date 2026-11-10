/**
 * GroupCard — one community group in the groups grid, with join / leave.
 */

import type { CommunityGroupWithMembership } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface GroupCardProps {
  group: CommunityGroupWithMembership;
  onOpen: (id: string) => void;
  onJoin: (id: string) => void;
  onLeave: (id: string) => void;
  isJoining?: boolean;
  isLeaving?: boolean;
}

export function GroupCard({
  group,
  onOpen,
  onJoin,
  onLeave,
  isJoining,
  isLeaving,
}: GroupCardProps) {
  const { t } = useTranslation();
  const isOwner = group.memberRole === 'owner';

  return (
    <article className="bg-white rounded-lg shadow overflow-hidden flex flex-col">
      <div
        className="h-20 bg-gradient-to-r from-blue-500 to-purple-600 relative"
        style={group.color ? { background: group.color } : undefined}
      >
        {group.coverImageUrl && (
          <img
            src={group.coverImageUrl}
            alt={t('community.groupCard.coverAlt', { name: group.name })}
            className="w-full h-full object-cover"
          />
        )}
        {group.isOfficial && (
          <span className="absolute top-2 right-2 bg-white/90 text-blue-700 text-xs font-medium px-2 py-0.5 rounded-full">
            {t('community.common.official')}
          </span>
        )}
      </div>
      <div className="p-4 flex flex-col flex-1">
        <h3 className="text-lg font-semibold text-gray-900 truncate">
          <button
            type="button"
            onClick={() => onOpen(group.id)}
            className="hover:text-blue-700 text-left"
          >
            {group.name}
          </button>
        </h3>
        <p className="mt-1 text-xs text-gray-500">
          {t(`community.groupTypes.${group.groupType}`)} ·{' '}
          {t(`community.groupCard.visibility.${group.visibility}`)}
        </p>
        {group.description && (
          <p className="mt-2 text-sm text-gray-600 line-clamp-2">{group.description}</p>
        )}
        <div className="mt-auto pt-4 flex items-center justify-between gap-2">
          <span className="text-sm text-gray-500">
            {t('community.groupDetail.members', { count: group.memberCount })}
          </span>
          {group.isMember ? (
            isOwner ? (
              <span className="text-xs text-gray-500">
                {t('community.groupMembers.roles.owner')}
              </span>
            ) : (
              <button
                type="button"
                onClick={() => onLeave(group.id)}
                disabled={isLeaving}
                className="px-3 py-1 text-sm border border-red-300 text-red-600 rounded-md hover:bg-red-50 disabled:opacity-50"
              >
                {isLeaving ? t('community.groupCard.leaving') : t('community.groupCard.leave')}
              </button>
            )
          ) : (
            <button
              type="button"
              onClick={() => onJoin(group.id)}
              disabled={isJoining}
              className="px-3 py-1 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
            >
              {isJoining ? t('community.groupCard.joining') : t('community.groupCard.join')}
            </button>
          )}
        </div>
      </div>
    </article>
  );
}
