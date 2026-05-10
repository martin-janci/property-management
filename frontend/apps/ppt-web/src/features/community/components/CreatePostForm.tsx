/**
 * CreatePostForm Component
 *
 * Form for creating new community posts.
 * Part of Story 42.2: Community Feed.
 */

import type { CreatePostRequest, PostType, PostVisibility } from '@ppt/api-client';
import type * as React from 'react';
import { useState } from 'react';

interface CreatePostFormProps {
  groupId?: string;
  userAvatar?: string;
  userName: string;
  isSubmitting?: boolean;
  onSubmit: (data: CreatePostRequest) => void;
  onCancel?: () => void;
}

const postTypes: { value: PostType; label: string; icon: React.JSX.Element }[] = [
  {
    value: 'text',
    label: 'Text',
    icon: (
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
          d="M4 6h16M4 12h16M4 18h7"
        />
      </svg>
    ),
  },
  {
    value: 'image',
    label: 'Photo',
    icon: (
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
          d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
        />
      </svg>
    ),
  },
  {
    value: 'poll',
    label: 'Poll',
    icon: (
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
          d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
        />
      </svg>
    ),
  },
];

const visibilityOptions: { value: PostVisibility; label: string }[] = [
  { value: 'group', label: 'Group Only' },
  { value: 'building', label: 'Building' },
  { value: 'public', label: 'Public' },
];

export function CreatePostForm({
  groupId,
  userAvatar,
  userName,
  isSubmitting,
  onSubmit,
  onCancel,
}: CreatePostFormProps) {
  const [content, setContent] = useState('');
  const [postType, setPostType] = useState<PostType>('text');
  const [visibility, setVisibility] = useState<PostVisibility>('group');
  const [mediaUrls, setMediaUrls] = useState<string[]>([]);
  const [mediaInput, setMediaInput] = useState('');
  const [isExpanded, setIsExpanded] = useState(false);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (content.trim() && groupId) {
      onSubmit({
        groupId,
        content: content.trim(),
        type: postType,
        visibility,
        mediaUrls: mediaUrls.length > 0 ? mediaUrls : undefined,
      });
      // Reset form
      setContent('');
      setMediaUrls([]);
      setMediaInput('');
      setIsExpanded(false);
    }
  };

  const addMediaUrl = () => {
    if (mediaInput.trim() && isValidUrl(mediaInput)) {
      setMediaUrls([...mediaUrls, mediaInput.trim()]);
      setMediaInput('');
    }
  };

  const removeMediaUrl = (index: number) => {
    setMediaUrls(mediaUrls.filter((_, i) => i !== index));
  };

  const isValidUrl = (url: string): boolean => {
    try {
      new URL(url);
      return true;
    } catch {
      return false;
    }
  };

  return (
    <div className="bg-white rounded-lg shadow">
      <form onSubmit={handleSubmit}>
        {/* Compact Header */}
        <div className="p-4 flex items-start gap-3">
          {userAvatar ? (
            <img src={userAvatar} alt={userName} className="w-10 h-10 rounded-full object-cover" />
          ) : (
            <div className="w-10 h-10 rounded-full bg-gray-300 flex items-center justify-center">
              <span className="text-gray-600 font-medium">{userName.charAt(0).toUpperCase()}</span>
            </div>
          )}
          <div className="flex-1">
            <textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              onFocus={() => setIsExpanded(true)}
              placeholder="What's on your mind?"
              rows={isExpanded ? 3 : 1}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-blue-500 focus:border-blue-500 resize-none"
            />
          </div>
        </div>

        {/* Expanded Options */}
        {isExpanded && (
          <>
            {/* Media URLs */}
            {postType === 'image' && (
              <div className="px-4 pb-4">
                <div className="flex gap-2 mb-2">
                  <input
                    type="url"
                    value={mediaInput}
                    onChange={(e) => setMediaInput(e.target.value)}
                    placeholder="Enter image URL"
                    className="flex-1 px-3 py-2 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
                  />
                  <button
                    type="button"
                    onClick={addMediaUrl}
                    disabled={!mediaInput.trim() || !isValidUrl(mediaInput)}
                    className="px-3 py-2 text-sm bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 disabled:opacity-50"
                  >
                    Add
                  </button>
                </div>
                {mediaUrls.length > 0 && (
                  <div className="flex flex-wrap gap-2">
                    {mediaUrls.map((url, index) => (
                      <div key={url} className="relative group">
                        <img
                          src={url}
                          alt={`Preview ${index + 1}`}
                          className="h-16 w-16 object-cover rounded"
                        />
                        <button
                          type="button"
                          onClick={() => removeMediaUrl(index)}
                          className="absolute -top-1 -right-1 w-5 h-5 bg-red-500 text-white rounded-full text-xs flex items-center justify-center opacity-0 group-hover:opacity-100"
                        >
                          ×
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Post Type Selector */}
            <div className="px-4 pb-3 flex items-center gap-2 border-t pt-3">
              {postTypes.map((type) => (
                <button
                  key={type.value}
                  type="button"
                  onClick={() => setPostType(type.value)}
                  className={`flex items-center gap-1 px-3 py-1 text-sm rounded-full transition-colors ${
                    postType === type.value
                      ? 'bg-blue-100 text-blue-700'
                      : 'text-gray-600 hover:bg-gray-100'
                  }`}
                >
                  {type.icon}
                  {type.label}
                </button>
              ))}
            </div>

            {/* Visibility & Actions */}
            <div className="px-4 pb-4 flex items-center justify-between border-t pt-3">
              <select
                value={visibility}
                onChange={(e) => setVisibility(e.target.value as PostVisibility)}
                className="px-3 py-1 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
              >
                {visibilityOptions.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>

              <div className="flex items-center gap-2">
                {onCancel && (
                  <button
                    type="button"
                    onClick={() => {
                      setIsExpanded(false);
                      onCancel();
                    }}
                    className="px-4 py-2 text-sm text-gray-600 hover:text-gray-800"
                  >
                    Cancel
                  </button>
                )}
                <button
                  type="submit"
                  disabled={!content.trim() || isSubmitting}
                  className="px-4 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {isSubmitting ? 'Posting...' : 'Post'}
                </button>
              </div>
            </div>
          </>
        )}
      </form>
    </div>
  );
}
