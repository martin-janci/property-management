/**
 * GroupDetailPage
 *
 * Page for viewing a community group's details, members, and posts.
 * Part of Story 42.1: Community Groups.
 */

import type { CommunityGroup, GroupMember, GroupMemberRole } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { GroupMemberList } from '../components/GroupMemberList';

interface GroupDetailPageProps {
  group: CommunityGroup;
  members: GroupMember[];
  currentUserId?: string;
  isMember?: boolean;
  isAdmin?: boolean;
  isOwner?: boolean;
  isJoining?: boolean;
  isLeaving?: boolean;
  onJoin: () => void;
  onLeave: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onNavigateToSettings: () => void;
  onNavigateBack: () => void;
  onPromoteMember: (memberId: string, newRole: GroupMemberRole) => void;
  onRemoveMember: (memberId: string) => void;
  onBanMember: (memberId: string) => void;
}

type TabType = 'about' | 'members' | 'posts';

export function GroupDetailPage({
  group,
  members,
  currentUserId,
  isMember,
  isAdmin,
  isOwner,
  isJoining,
  isLeaving,
  onJoin,
  onLeave,
  onEdit,
  onDelete,
  onNavigateToSettings,
  onNavigateBack,
  onPromoteMember,
  onRemoveMember,
  onBanMember,
}: GroupDetailPageProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabType>('about');

  const canManageMembers = isAdmin || isOwner;

  const tabs: { id: TabType; label: string }[] = [
    { id: 'about', label: t('community.groupDetail.tabAbout') },
    { id: 'members', label: t('community.groupDetail.tabMembers', { count: group.memberCount }) },
    { id: 'posts', label: t('community.groupDetail.tabPosts', { count: group.postCount }) },
  ];

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Back Button */}
      <button
        type="button"
        onClick={onNavigateBack}
        className="flex items-center text-gray-600 hover:text-gray-900 mb-6"
      >
        <svg
          className="w-5 h-5 mr-1"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          aria-hidden="true"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        {t('community.common.backToGroups')}
      </button>

      {/* Header */}
      <div className="bg-white rounded-lg shadow overflow-hidden">
        {/* Cover Image */}
        <div className="h-48 bg-gradient-to-r from-blue-500 to-purple-600 relative">
          {group.coverImageUrl && (
            <img
              src={group.coverImageUrl}
              alt={t('community.groupCard.coverAlt', { name: group.name })}
              className="w-full h-full object-cover"
            />
          )}
          {group.isOfficial && (
            <span className="absolute top-4 right-4 bg-blue-600 text-white text-sm px-3 py-1 rounded-full flex items-center gap-1">
              <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
                <path
                  fillRule="evenodd"
                  d="M6.267 3.455a3.066 3.066 0 001.745-.723 3.066 3.066 0 013.976 0 3.066 3.066 0 001.745.723 3.066 3.066 0 012.812 2.812c.051.643.304 1.254.723 1.745a3.066 3.066 0 010 3.976 3.066 3.066 0 00-.723 1.745 3.066 3.066 0 01-2.812 2.812 3.066 3.066 0 00-1.745.723 3.066 3.066 0 01-3.976 0 3.066 3.066 0 00-1.745-.723 3.066 3.066 0 01-2.812-2.812 3.066 3.066 0 00-.723-1.745 3.066 3.066 0 010-3.976 3.066 3.066 0 00.723-1.745 3.066 3.066 0 012.812-2.812zm7.44 5.252a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                  clipRule="evenodd"
                />
              </svg>
              {t('community.common.official')}
            </span>
          )}
        </div>

        {/* Group Info */}
        <div className="p-6">
          <div className="flex items-start justify-between">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">{group.name}</h1>
              <div className="flex items-center gap-3 mt-2 text-sm text-gray-500">
                <span className="flex items-center gap-1">
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
                      d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"
                    />
                  </svg>
                  {group.category}
                </span>
                <span>•</span>
                <span className="flex items-center gap-1">
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
                      d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z"
                    />
                  </svg>
                  {t('community.groupDetail.members', { count: group.memberCount })}
                </span>
                <span>•</span>
                <span>{t(`community.groupCard.visibility.${group.visibility}`)}</span>
              </div>
            </div>

            <div className="flex items-center gap-2">
              {isMember ? (
                <>
                  {(isAdmin || isOwner) && (
                    <button
                      type="button"
                      onClick={onNavigateToSettings}
                      className="p-2 text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded-md"
                      title={t('community.groupDetail.groupSettings')}
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
                          d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                        />
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                        />
                      </svg>
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={onLeave}
                    disabled={isLeaving || isOwner}
                    className="px-4 py-2 text-sm border border-red-300 text-red-600 rounded-md hover:bg-red-50 disabled:opacity-50"
                    title={
                      isOwner
                        ? t('community.groupDetail.ownersCannotLeave')
                        : t('community.groupDetail.leaveGroupTitle')
                    }
                  >
                    {isLeaving
                      ? t('community.groupDetail.leaving')
                      : t('community.groupDetail.leaveGroup')}
                  </button>
                </>
              ) : (
                <button
                  type="button"
                  onClick={onJoin}
                  disabled={isJoining}
                  className="px-4 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
                >
                  {isJoining
                    ? t('community.groupDetail.joining')
                    : t('community.groupDetail.joinGroup')}
                </button>
              )}
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="border-t border-gray-200">
          <nav className="flex">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                type="button"
                onClick={() => setActiveTab(tab.id)}
                className={`flex-1 py-3 text-sm font-medium text-center border-b-2 transition-colors ${
                  activeTab === tab.id
                    ? 'border-blue-500 text-blue-600'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }`}
              >
                {tab.label}
              </button>
            ))}
          </nav>
        </div>
      </div>

      {/* Tab Content */}
      <div className="mt-6">
        {activeTab === 'about' && (
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              {t('community.groupDetail.aboutTitle')}
            </h2>
            <p className="text-gray-600 whitespace-pre-wrap">{group.description}</p>
            <div className="mt-6 pt-4 border-t text-sm text-gray-500">
              <p>
                {t('community.groupDetail.createdOn', {
                  date: new Date(group.createdAt).toLocaleDateString(),
                })}
              </p>
            </div>
          </div>
        )}

        {activeTab === 'members' && (
          <div>
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">
                {t('community.groupDetail.membersTitle')}
              </h2>
              {canManageMembers && (
                <button type="button" className="text-sm text-blue-600 hover:text-blue-800">
                  {t('community.groupDetail.inviteMembers')}
                </button>
              )}
            </div>
            <GroupMemberList
              members={members}
              currentUserId={currentUserId}
              canManageMembers={canManageMembers}
              onPromote={onPromoteMember}
              onRemove={onRemoveMember}
              onBan={onBanMember}
            />
          </div>
        )}

        {activeTab === 'posts' && (
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
                d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z"
              />
            </svg>
            <p className="mt-4 text-gray-500">{t('community.groupDetail.postsPlaceholder')}</p>
          </div>
        )}
      </div>

      {/* Admin Actions */}
      {(isAdmin || isOwner) && (
        <div className="mt-8 bg-white rounded-lg shadow p-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            {t('community.groupDetail.adminActions')}
          </h2>
          <div className="flex items-center gap-4">
            <button
              type="button"
              onClick={onEdit}
              className="px-4 py-2 text-sm bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200"
            >
              {t('community.groupDetail.editGroup')}
            </button>
            {isOwner && (
              <button
                type="button"
                onClick={onDelete}
                className="px-4 py-2 text-sm border border-red-300 text-red-600 rounded-md hover:bg-red-50"
              >
                {t('community.groupDetail.deleteGroup')}
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
