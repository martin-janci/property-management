/**
 * CreatePostForm — composer for a new post in one of the caller's groups.
 *
 * Posts always belong to a group (`POST /groups/{group_id}/posts`). On the
 * feed the author picks one of the groups they are a member of; on a group's
 * detail page the group is fixed.
 */

import type { CommunityPostType, CreateCommunityPostRequest } from '@ppt/api-client';
import { type FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';

const POST_TYPES: CommunityPostType[] = ['text', 'question', 'announcement'];

interface CreatePostFormProps {
  /** Groups the author may post to. With exactly one, no picker is shown. */
  groups: { id: string; name: string }[];
  isSubmitting?: boolean;
  /** Resolves on success (the form then clears); rejects to keep the draft. */
  onSubmit: (groupId: string, data: CreateCommunityPostRequest) => Promise<unknown>;
}

export function CreatePostForm({ groups, isSubmitting, onSubmit }: CreatePostFormProps) {
  const { t } = useTranslation();
  const [groupId, setGroupId] = useState(groups[0]?.id ?? '');
  const [postType, setPostType] = useState<CommunityPostType>('text');
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [isAnonymous, setIsAnonymous] = useState(false);

  const effectiveGroupId = groups.some((g) => g.id === groupId) ? groupId : (groups[0]?.id ?? '');
  const canSubmit = !!effectiveGroupId && content.trim().length > 0 && !isSubmitting;

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;
    try {
      await onSubmit(effectiveGroupId, {
        content: content.trim(),
        postType,
        title: title.trim() || undefined,
        isAnonymous: isAnonymous || undefined,
      });
      setTitle('');
      setContent('');
      setIsAnonymous(false);
      setPostType('text');
    } catch {
      // The caller surfaces the error; keep the draft so nothing is lost.
    }
  };

  return (
    <form onSubmit={handleSubmit} className="bg-white rounded-lg shadow p-4 space-y-3">
      <div className="flex flex-wrap gap-3">
        {groups.length > 1 && (
          <label className="flex items-center gap-2 text-sm text-gray-600">
            <span>{t('community.hub.postTo')}</span>
            <select
              value={effectiveGroupId}
              onChange={(e) => setGroupId(e.target.value)}
              className="px-2 py-1 text-sm border border-gray-300 rounded-md"
            >
              {groups.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name}
                </option>
              ))}
            </select>
          </label>
        )}
        <label className="flex items-center gap-2 text-sm text-gray-600">
          <span>{t('community.hub.postType')}</span>
          <select
            value={postType}
            onChange={(e) => setPostType(e.target.value as CommunityPostType)}
            className="px-2 py-1 text-sm border border-gray-300 rounded-md"
          >
            {POST_TYPES.map((type) => (
              <option key={type} value={type}>
                {t(`community.postTypes.${type}`)}
              </option>
            ))}
          </select>
        </label>
      </div>
      <input
        type="text"
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        maxLength={200}
        placeholder={t('community.hub.postTitlePlaceholder')}
        aria-label={t('community.hub.postTitlePlaceholder')}
        className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
      />
      <textarea
        value={content}
        onChange={(e) => setContent(e.target.value)}
        rows={3}
        maxLength={5000}
        placeholder={t('community.createPost.contentPlaceholder')}
        aria-label={t('community.createPost.contentPlaceholder')}
        className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
      />
      <div className="flex items-center justify-between gap-3">
        <label className="flex items-center gap-2 text-sm text-gray-600">
          <input
            type="checkbox"
            checked={isAnonymous}
            onChange={(e) => setIsAnonymous(e.target.checked)}
            className="rounded border-gray-300"
          />
          {t('community.hub.postAnonymously')}
        </label>
        <button
          type="submit"
          disabled={!canSubmit}
          className="px-4 py-2 text-sm font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {isSubmitting ? t('community.createPost.posting') : t('community.createPost.submit')}
        </button>
      </div>
    </form>
  );
}
