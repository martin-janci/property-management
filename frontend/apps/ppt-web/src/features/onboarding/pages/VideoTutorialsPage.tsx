/**
 * VideoTutorialsPage - video tutorials page.
 * UC-42: Onboarding/Help Feature
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { VideoTutorial, VideoTutorialCategory } from '../types';
import { formatVideoDuration } from '../types';

export interface VideoTutorialsPageProps {
  videos: VideoTutorial[];
  isLoading?: boolean;
  onNavigateBack: () => void;
  onPlayVideo: (videoId: string) => void;
}

export function VideoTutorialsPage({
  videos,
  isLoading,
  onNavigateBack,
  onPlayVideo,
}: VideoTutorialsPageProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<VideoTutorialCategory>('all');
  const [selectedVideo, setSelectedVideo] = useState<VideoTutorial | null>(null);

  const categories: { value: VideoTutorialCategory; label: string }[] = [
    { value: 'all', label: t('help.tutorials.categoryAll') },
    { value: 'getting_started', label: t('help.tutorials.categoryGettingStarted') },
    { value: 'features', label: t('help.tutorials.categoryFeatures') },
    { value: 'tips', label: t('help.tutorials.categoryTips') },
    { value: 'advanced', label: t('help.tutorials.categoryAdvanced') },
  ];

  const filteredVideos = videos.filter((video) => {
    const matchesCategory = selectedCategory === 'all' || video.category === selectedCategory;
    const matchesSearch =
      !searchQuery ||
      video.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      video.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      video.tags?.some((tag) => tag.toLowerCase().includes(searchQuery.toLowerCase()));
    return matchesCategory && matchesSearch;
  });

  const featuredVideos = videos.filter((video) => video.isFeatured);

  const handleVideoClick = (video: VideoTutorial) => {
    setSelectedVideo(video);
    onPlayVideo(video.id);
  };

  const closeVideoModal = () => {
    setSelectedVideo(null);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onNavigateBack}
          className="flex items-center text-sm text-gray-600 hover:text-gray-800 mb-4"
        >
          <svg className="w-4 h-4 mr-1" fill="currentColor" viewBox="0 0 20 20">
            <title>{t('common.back')}</title>
            <path
              fillRule="evenodd"
              d="M9.707 16.707a1 1 0 01-1.414 0l-6-6a1 1 0 010-1.414l6-6a1 1 0 011.414 1.414L5.414 9H17a1 1 0 110 2H5.414l4.293 4.293a1 1 0 010 1.414z"
              clipRule="evenodd"
            />
          </svg>
          {t('help.backToHelpCenter')}
        </button>

        <h1 className="text-2xl font-bold text-gray-900">{t('help.tutorials.title')}</h1>
        <p className="mt-1 text-sm text-gray-500">{t('help.tutorials.subtitle')}</p>
      </div>

      {/* Search and filter */}
      <div className="mb-6 space-y-4">
        <div className="relative max-w-md">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t('help.tutorials.searchPlaceholder')}
            className="w-full px-4 py-2 pl-10 text-gray-900 bg-white border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
          <svg
            className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-gray-400"
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
        </div>

        <div className="flex flex-wrap gap-2">
          {categories.map((category) => (
            <button
              key={category.value}
              type="button"
              onClick={() => setSelectedCategory(category.value)}
              className={`px-4 py-2 text-sm font-medium rounded-lg transition-colors ${
                selectedCategory === category.value
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              {category.label}
            </button>
          ))}
        </div>
      </div>

      {/* Featured videos */}
      {selectedCategory === 'all' && featuredVideos.length > 0 && (
        <div className="mb-8">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            {t('help.tutorials.featured')}
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {featuredVideos.slice(0, 2).map((video) => (
              <FeaturedVideoCard key={video.id} video={video} onClick={handleVideoClick} />
            ))}
          </div>
        </div>
      )}

      {/* Video grid */}
      {filteredVideos.length === 0 ? (
        <div className="text-center py-12 bg-white rounded-lg shadow">
          <svg
            className="mx-auto h-12 w-12 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <title>{t('help.tutorials.noResults')}</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"
            />
          </svg>
          <p className="mt-4 text-gray-500">{t('help.tutorials.noResults')}</p>
          <p className="mt-2 text-sm text-gray-400">{t('help.tutorials.noResultsDescription')}</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
          {filteredVideos.map((video) => (
            <VideoCard key={video.id} video={video} onClick={handleVideoClick} />
          ))}
        </div>
      )}

      {/* Video modal */}
      {selectedVideo && <VideoModal video={selectedVideo} onClose={closeVideoModal} />}
    </div>
  );
}

interface VideoCardProps {
  video: VideoTutorial;
  onClick: (video: VideoTutorial) => void;
}

function VideoCard({ video, onClick }: VideoCardProps) {
  const { t } = useTranslation();

  const categoryLabels: Record<VideoTutorialCategory, string> = {
    all: t('help.tutorials.categoryAll'),
    getting_started: t('help.tutorials.categoryGettingStarted'),
    features: t('help.tutorials.categoryFeatures'),
    tips: t('help.tutorials.categoryTips'),
    advanced: t('help.tutorials.categoryAdvanced'),
  };

  return (
    <button
      type="button"
      onClick={() => onClick(video)}
      className="text-left bg-white rounded-lg border border-gray-200 overflow-hidden shadow-sm hover:shadow-md transition-shadow"
    >
      {/* Thumbnail */}
      <div className="relative aspect-video bg-gray-100">
        {video.thumbnailUrl ? (
          <img src={video.thumbnailUrl} alt={video.title} className="w-full h-full object-cover" />
        ) : (
          <div className="w-full h-full flex items-center justify-center bg-gray-200">
            <svg className="w-12 h-12 text-gray-400" fill="currentColor" viewBox="0 0 20 20">
              <title>{t('help.tutorials.videoPlaceholder')}</title>
              <path
                fillRule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z"
                clipRule="evenodd"
              />
            </svg>
          </div>
        )}
        {/* Play button overlay */}
        <div className="absolute inset-0 flex items-center justify-center bg-black bg-opacity-30 opacity-0 hover:opacity-100 transition-opacity">
          <div className="w-14 h-14 bg-white bg-opacity-90 rounded-full flex items-center justify-center">
            <svg className="w-8 h-8 text-blue-600 ml-1" fill="currentColor" viewBox="0 0 20 20">
              <title>{t('help.tutorials.play')}</title>
              <path
                fillRule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z"
                clipRule="evenodd"
              />
            </svg>
          </div>
        </div>
        {/* Duration badge */}
        <div className="absolute bottom-2 right-2 px-2 py-1 bg-black bg-opacity-75 text-white text-xs rounded">
          {formatVideoDuration(video.duration)}
        </div>
      </div>

      {/* Content */}
      <div className="p-4">
        <div className="flex items-center gap-2 mb-2">
          <span className="text-xs px-2 py-0.5 bg-gray-100 text-gray-600 rounded">
            {categoryLabels[video.category]}
          </span>
          {video.isFeatured && (
            <span className="text-xs px-2 py-0.5 bg-blue-100 text-blue-600 rounded">
              {t('help.tutorials.featured')}
            </span>
          )}
        </div>
        <h3 className="font-medium text-gray-900 line-clamp-2">{video.title}</h3>
        <p className="text-sm text-gray-500 mt-1 line-clamp-2">{video.description}</p>
        {video.viewCount !== undefined && (
          <p className="text-xs text-gray-400 mt-2">
            {video.viewCount.toLocaleString(getActiveLocale())} {t('help.tutorials.views')}
          </p>
        )}
      </div>
    </button>
  );
}

interface FeaturedVideoCardProps {
  video: VideoTutorial;
  onClick: (video: VideoTutorial) => void;
}

function FeaturedVideoCard({ video, onClick }: FeaturedVideoCardProps) {
  const { t } = useTranslation();

  return (
    <button
      type="button"
      onClick={() => onClick(video)}
      className="text-left bg-gradient-to-r from-blue-600 to-blue-700 rounded-lg overflow-hidden shadow-lg hover:shadow-xl transition-shadow"
    >
      <div className="flex">
        {/* Thumbnail */}
        <div className="relative w-2/5 aspect-video bg-blue-800">
          {video.thumbnailUrl ? (
            <img
              src={video.thumbnailUrl}
              alt={video.title}
              className="w-full h-full object-cover"
            />
          ) : (
            <div className="w-full h-full flex items-center justify-center">
              <svg className="w-16 h-16 text-blue-400" fill="currentColor" viewBox="0 0 20 20">
                <title>{t('help.tutorials.videoPlaceholder')}</title>
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z"
                  clipRule="evenodd"
                />
              </svg>
            </div>
          )}
          {/* Play button overlay */}
          <div className="absolute inset-0 flex items-center justify-center">
            <div className="w-12 h-12 bg-white bg-opacity-90 rounded-full flex items-center justify-center shadow-lg">
              <svg className="w-6 h-6 text-blue-600 ml-1" fill="currentColor" viewBox="0 0 20 20">
                <title>{t('help.tutorials.play')}</title>
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z"
                  clipRule="evenodd"
                />
              </svg>
            </div>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 p-5 text-white">
          <span className="text-xs px-2 py-0.5 bg-white bg-opacity-20 rounded">
            {t('help.tutorials.featured')}
          </span>
          <h3 className="font-semibold text-lg mt-2 line-clamp-2">{video.title}</h3>
          <p className="text-sm text-blue-100 mt-1 line-clamp-2">{video.description}</p>
          <div className="flex items-center gap-3 mt-3 text-xs text-blue-200">
            <span>{formatVideoDuration(video.duration)}</span>
            {video.viewCount !== undefined && (
              <span>
                {video.viewCount.toLocaleString(getActiveLocale())} {t('help.tutorials.views')}
              </span>
            )}
          </div>
        </div>
      </div>
    </button>
  );
}

interface VideoModalProps {
  video: VideoTutorial;
  onClose: () => void;
}

function VideoModal({ video, onClose }: VideoModalProps) {
  const { t } = useTranslation();

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black bg-opacity-75">
      <div className="bg-white rounded-lg max-w-4xl w-full max-h-[90vh] overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b">
          <h3 className="font-semibold text-gray-900">{video.title}</h3>
          <button
            type="button"
            onClick={onClose}
            className="p-1 text-gray-400 hover:text-gray-600 rounded"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <title>{t('common.close')}</title>
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        {/* Video player */}
        <div className="aspect-video bg-black">
          <iframe
            src={video.videoUrl}
            title={video.title}
            className="w-full h-full"
            allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
            allowFullScreen
          />
        </div>

        {/* Description */}
        <div className="p-4">
          <p className="text-gray-600">{video.description}</p>
          {video.tags && video.tags.length > 0 && (
            <div className="flex flex-wrap gap-2 mt-3">
              {video.tags.map((tag) => (
                <span key={tag} className="px-2 py-1 text-xs bg-gray-100 text-gray-600 rounded">
                  {tag}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
