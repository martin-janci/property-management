/**
 * CreateGroupPage — `/community/groups/new`.
 *
 * Creates a group in the selected building
 * (`POST /community/buildings/{id}/groups`); the creator becomes its owner.
 * On success it opens the new group; on failure the form stays filled and the
 * server error is shown inline and as a toast.
 */

import {
  type CommunityGroupType,
  type CommunityGroupVisibility,
  useCreateGroup,
} from '@ppt/api-client';
import { type FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useToast } from '../../../components';
import { getErrorMessage } from '../../../lib/api';
import { CommunityLayout } from '../components/CommunityLayout';
import { useCommunityBuilding, withBuilding } from '../hooks/useCommunityBuilding';
import { GROUP_TYPES } from './GroupsPage';

const VISIBILITIES: CommunityGroupVisibility[] = ['public', 'private', 'secret'];

export function CreateGroupPage() {
  const { t } = useTranslation();
  const building = useCommunityBuilding();

  return (
    <CommunityLayout active="groups" title={t('community.createGroup.title')} building={building}>
      {(buildingId) => <CreateGroupForm buildingId={buildingId} />}
    </CommunityLayout>
  );
}

function CreateGroupForm({ buildingId }: { buildingId: string }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const createGroup = useCreateGroup();

  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [groupType, setGroupType] = useState<CommunityGroupType>('interest');
  const [visibility, setVisibility] = useState<CommunityGroupVisibility>('public');
  const [rules, setRules] = useState('');
  const [touched, setTouched] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  const trimmed = name.trim();
  const nameError = !trimmed
    ? t('community.groupForm.errors.nameRequired')
    : trimmed.length < 3
      ? t('community.groupForm.errors.nameMin')
      : trimmed.length > 100
        ? t('community.groupForm.errors.nameMax')
        : null;
  const descriptionError =
    description.trim().length > 1000 ? t('community.groupForm.errors.descriptionMax') : null;

  const backToGroups = () => navigate(withBuilding('/community/groups', buildingId));

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    setTouched(true);
    if (nameError || descriptionError) return;
    setSubmitError(null);
    createGroup.mutate(
      {
        buildingId,
        data: {
          name: trimmed,
          description: description.trim() || undefined,
          groupType,
          visibility,
          rules: rules.trim() || undefined,
        },
      },
      {
        onSuccess: (group) => {
          showToast({
            type: 'success',
            title: t('common.created', { defaultValue: 'Created' }),
            message: t('community.groupCreated'),
          });
          navigate(withBuilding(`/community/groups/${group.id}`, buildingId));
        },
        onError: (err) => {
          const message = getErrorMessage(err);
          setSubmitError(message);
          showToast({ type: 'error', title: t('community.hub.createGroupFailed'), message });
        },
      }
    );
  };

  const inputClass =
    'mt-1 w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500';

  return (
    <form
      onSubmit={handleSubmit}
      noValidate
      className="max-w-2xl bg-white rounded-lg shadow p-6 space-y-5"
    >
      {submitError && (
        <div
          role="alert"
          className="rounded-md bg-red-50 border border-red-200 p-3 text-sm text-red-700"
        >
          {t('community.hub.createGroupFailed')}: {submitError}
        </div>
      )}

      <div>
        <label htmlFor="group-name" className="block text-sm font-medium text-gray-700">
          {t('community.groupForm.nameLabel')} *
        </label>
        <input
          id="group-name"
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t('community.groupForm.namePlaceholder')}
          aria-invalid={touched && !!nameError}
          className={inputClass}
        />
        {touched && nameError && <p className="mt-1 text-sm text-red-600">{nameError}</p>}
      </div>

      <div>
        <label htmlFor="group-description" className="block text-sm font-medium text-gray-700">
          {t('community.groupForm.descriptionLabel')}
        </label>
        <textarea
          id="group-description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          rows={4}
          placeholder={t('community.groupForm.descriptionPlaceholder')}
          className={inputClass}
        />
        {touched && descriptionError && (
          <p className="mt-1 text-sm text-red-600">{descriptionError}</p>
        )}
      </div>

      <div>
        <label htmlFor="group-type" className="block text-sm font-medium text-gray-700">
          {t('community.hub.groupType')}
        </label>
        <select
          id="group-type"
          value={groupType}
          onChange={(e) => setGroupType(e.target.value as CommunityGroupType)}
          className={inputClass}
        >
          {GROUP_TYPES.map((gt) => (
            <option key={gt} value={gt}>
              {t(`community.groupTypes.${gt}`)}
            </option>
          ))}
        </select>
      </div>

      <fieldset>
        <legend className="text-sm font-medium text-gray-700">
          {t('community.groupForm.visibilityLegend')}
        </legend>
        <div className="mt-2 space-y-2">
          {VISIBILITIES.map((v) => (
            <label key={v} className="flex items-start gap-3 text-sm">
              <input
                type="radio"
                name="visibility"
                value={v}
                checked={visibility === v}
                onChange={() => setVisibility(v)}
                className="mt-1"
              />
              <span>
                <span className="font-medium text-gray-900">
                  {t(`community.groupForm.visibilityOptions.${v}.label`)}
                </span>
                <span className="block text-gray-500">
                  {t(`community.groupForm.visibilityOptions.${v}.description`)}
                </span>
              </span>
            </label>
          ))}
        </div>
      </fieldset>

      <div>
        <label htmlFor="group-rules" className="block text-sm font-medium text-gray-700">
          {t('community.hub.groupRules')}
        </label>
        <textarea
          id="group-rules"
          value={rules}
          onChange={(e) => setRules(e.target.value)}
          rows={3}
          placeholder={t('community.hub.groupRulesPlaceholder')}
          className={inputClass}
        />
      </div>

      <div className="flex justify-end gap-3 pt-2">
        <button
          type="button"
          onClick={backToGroups}
          className="px-4 py-2 text-sm font-medium text-gray-700 border border-gray-300 rounded-md hover:bg-gray-50"
        >
          {t('community.common.cancel')}
        </button>
        <button
          type="submit"
          disabled={createGroup.isPending}
          className="px-4 py-2 text-sm font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {createGroup.isPending
            ? t('community.groupForm.creating')
            : t('community.groupForm.create')}
        </button>
      </div>
    </form>
  );
}
