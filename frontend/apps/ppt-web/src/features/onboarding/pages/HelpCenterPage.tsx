/**
 * HelpCenterPage - main help hub page.
 * UC-42: Onboarding/Help Feature
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { HelpAnnouncement, HelpTopic } from '../types';

export interface HelpCenterPageProps {
  topics: HelpTopic[];
  announcements: HelpAnnouncement[];
  isLoading?: boolean;
  onSearch: (query: string) => void;
  onNavigateToFAQ: () => void;
  onNavigateToTutorials: () => void;
  onNavigateToSupport: () => void;
  onNavigateToFeedback: () => void;
  onNavigateToTopic: (topicId: string) => void;
}

export function HelpCenterPage({
  topics,
  announcements,
  isLoading,
  onSearch,
  onNavigateToFAQ,
  onNavigateToTutorials,
  onNavigateToSupport,
  onNavigateToFeedback,
  onNavigateToTopic,
}: HelpCenterPageProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');

  const popularTopics = topics.filter((topic) => topic.isPopular);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (searchQuery.trim()) {
      onSearch(searchQuery.trim());
    }
  };

  const handleSearchInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setSearchQuery(e.target.value);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="text-center mb-8">
        <h1 className="text-3xl font-bold text-gray-900">{t('help.title')}</h1>
        <p className="mt-2 text-gray-600">{t('help.subtitle')}</p>
      </div>

      {/* Search bar */}
      <form onSubmit={handleSearch} className="mb-8">
        <div className="relative max-w-2xl mx-auto">
          <input
            type="text"
            value={searchQuery}
            onChange={handleSearchInputChange}
            placeholder={t('help.searchPlaceholder')}
            className="w-full px-4 py-3 pl-12 text-gray-900 bg-white border border-gray-300 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
          <svg
            className="absolute left-4 top-1/2 transform -translate-y-1/2 w-5 h-5 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <title>{t('common.search')}</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <button
            type="submit"
            className="absolute right-2 top-1/2 transform -translate-y-1/2 px-4 py-1.5 bg-blue-600 text-white text-sm font-medium rounded-md hover:bg-blue-700"
          >
            {t('common.search')}
          </button>
        </div>
      </form>

      {/* Quick links */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
        <QuickLinkCard
          icon={
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <title>{t('help.faqLabel')}</title>
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
          }
          title={t('help.faqLabel')}
          description={t('help.faqDescription')}
          onClick={onNavigateToFAQ}
        />
        <QuickLinkCard
          icon={
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <title>{t('help.tutorialsLabel')}</title>
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
              />
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
          }
          title={t('help.tutorialsLabel')}
          description={t('help.tutorialsDescription')}
          onClick={onNavigateToTutorials}
        />
        <QuickLinkCard
          icon={
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <title>{t('help.supportLabel')}</title>
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M18.364 5.636l-3.536 3.536m0 5.656l3.536 3.536M9.172 9.172L5.636 5.636m3.536 9.192l-3.536 3.536M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-5 0a4 4 0 11-8 0 4 4 0 018 0z"
              />
            </svg>
          }
          title={t('help.supportLabel')}
          description={t('help.supportDescription')}
          onClick={onNavigateToSupport}
        />
        <QuickLinkCard
          icon={
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <title>{t('help.feedbackLabel')}</title>
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z"
              />
            </svg>
          }
          title={t('help.feedbackLabel')}
          description={t('help.feedbackDescription')}
          onClick={onNavigateToFeedback}
        />
      </div>

      {/* Popular help topics */}
      {popularTopics.length > 0 && (
        <div className="mb-8">
          <h2 className="text-xl font-semibold text-gray-900 mb-4">{t('help.popularTopics')}</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {popularTopics.map((topic) => (
              <button
                key={topic.id}
                type="button"
                onClick={() => onNavigateToTopic(topic.id)}
                className="p-4 bg-white rounded-lg border border-gray-200 shadow-sm hover:shadow-md hover:border-blue-300 transition-all text-left"
              >
                <h3 className="font-medium text-gray-900 mb-1">{topic.title}</h3>
                <p className="text-sm text-gray-500">{topic.description}</p>
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Recent announcements */}
      {announcements.length > 0 && (
        <div className="mb-8">
          <h2 className="text-xl font-semibold text-gray-900 mb-4">
            {t('help.recentAnnouncements')}
          </h2>
          <div className="space-y-3">
            {announcements.map((announcement) => (
              <AnnouncementCard key={announcement.id} announcement={announcement} />
            ))}
          </div>
        </div>
      )}

      {/* Contact section */}
      <div className="bg-gray-50 rounded-lg p-6 text-center">
        <h2 className="text-lg font-semibold text-gray-900 mb-2">{t('help.stillNeedHelp')}</h2>
        <p className="text-gray-600 mb-4">{t('help.stillNeedHelpDescription')}</p>
        <button
          type="button"
          onClick={onNavigateToSupport}
          className="px-6 py-2 bg-blue-600 text-white font-medium rounded-md hover:bg-blue-700"
        >
          {t('help.contactSupport')}
        </button>
      </div>
    </div>
  );
}

interface QuickLinkCardProps {
  icon: React.ReactNode;
  title: string;
  description: string;
  onClick: () => void;
}

function QuickLinkCard({ icon, title, description, onClick }: QuickLinkCardProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex flex-col items-center p-4 bg-white rounded-lg border border-gray-200 shadow-sm hover:shadow-md hover:border-blue-300 transition-all text-center"
    >
      <div className="p-3 bg-blue-100 text-blue-600 rounded-full mb-3">{icon}</div>
      <h3 className="font-medium text-gray-900 mb-1">{title}</h3>
      <p className="text-xs text-gray-500">{description}</p>
    </button>
  );
}

interface AnnouncementCardProps {
  announcement: HelpAnnouncement;
}

function AnnouncementCard({ announcement }: AnnouncementCardProps) {
  const typeStyles = {
    info: 'bg-blue-50 border-blue-200 text-blue-800',
    warning: 'bg-yellow-50 border-yellow-200 text-yellow-800',
    success: 'bg-green-50 border-green-200 text-green-800',
    update: 'bg-purple-50 border-purple-200 text-purple-800',
  };

  const typeIcons = {
    info: (
      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
        <title>Info</title>
        <path
          fillRule="evenodd"
          d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
          clipRule="evenodd"
        />
      </svg>
    ),
    warning: (
      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
        <title>Warning</title>
        <path
          fillRule="evenodd"
          d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
          clipRule="evenodd"
        />
      </svg>
    ),
    success: (
      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
        <title>Success</title>
        <path
          fillRule="evenodd"
          d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
          clipRule="evenodd"
        />
      </svg>
    ),
    update: (
      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
        <title>Update</title>
        <path
          fillRule="evenodd"
          d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z"
          clipRule="evenodd"
        />
      </svg>
    ),
  };

  return (
    <div className={`flex items-start p-4 rounded-lg border ${typeStyles[announcement.type]}`}>
      <div className="flex-shrink-0 mr-3">{typeIcons[announcement.type]}</div>
      <div className="flex-1">
        <h3 className="font-medium">{announcement.title}</h3>
        <p className="text-sm mt-1 opacity-80">{announcement.content}</p>
        <p className="text-xs mt-2 opacity-60">
          {new Date(announcement.publishedAt).toLocaleDateString()}
        </p>
      </div>
      {announcement.link && (
        <a
          href={announcement.link}
          className="text-sm font-medium hover:underline ml-4"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn more about this announcement
        </a>
      )}
    </div>
  );
}
