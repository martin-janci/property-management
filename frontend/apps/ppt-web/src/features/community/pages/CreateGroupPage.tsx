/**
 * CreateGroupPage
 *
 * Page for creating a new community group.
 * Part of Story 42.1: Community Groups.
 */

import type { CreateGroupRequest } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { GroupForm } from '../components/GroupForm';

interface CreateGroupPageProps {
  isSubmitting?: boolean;
  onSubmit: (data: CreateGroupRequest) => void;
  onCancel: () => void;
}

export function CreateGroupPage({ isSubmitting, onSubmit, onCancel }: CreateGroupPageProps) {
  const { t } = useTranslation();
  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      <div className="mb-6">
        <button
          type="button"
          onClick={onCancel}
          className="flex items-center text-gray-600 hover:text-gray-900"
        >
          <svg
            className="w-5 h-5 mr-1"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
          {t('community.common.backToGroups')}
        </button>
      </div>

      <div className="bg-white rounded-lg shadow p-6">
        <h1 className="text-2xl font-bold text-gray-900 mb-6">
          {t('community.createGroup.title')}
        </h1>
        <GroupForm
          isSubmitting={isSubmitting}
          onSubmit={(data) => onSubmit(data as CreateGroupRequest)}
          onCancel={onCancel}
        />
      </div>
    </div>
  );
}
