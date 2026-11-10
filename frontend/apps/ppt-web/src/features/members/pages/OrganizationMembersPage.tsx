/**
 * OrganizationMembersPage — manage the people in the manager's organization.
 *
 * Lists members with their role, adds an existing account by e-mail (the
 * person registers first, then the manager adds them with a role), changes a
 * member's role and removes members. Backed by
 * `/api/v1/organizations/{id}/members` and `/roles`.
 *
 * The server enforces the escalation rules (no self role change, nobody above
 * the caller's level); its error codes are mapped to translated messages here.
 */

import {
  ApiError,
  type OrgMember,
  useAddOrgMember,
  useOrgMembers,
  useOrgRoles,
  useRemoveOrgMember,
  useUpdateOrgMember,
} from '@ppt/api-client';
import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ConfirmationDialog, useToast } from '../../../components';
import { useAuth } from '../../../contexts';

const inputClass =
  'w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500';

/** Map a members API failure to an i18n key under `members.errors`. */
export function memberErrorKey(error: unknown): string {
  if (error instanceof ApiError) {
    if (error.status === 404) return 'userNotFound';
    if (error.status === 409) return 'alreadyMember';
    if (error.status === 403) return 'forbidden';
    if (error.status === 400) return 'invalid';
  }
  return 'generic';
}

export function OrganizationMembersPage() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const { showToast } = useToast();
  const orgId = user?.organizationId ?? '';

  const { data: membersData, isLoading, error } = useOrgMembers(orgId || undefined);
  const { data: rolesData } = useOrgRoles(orgId || undefined);
  const addMember = useAddOrgMember(orgId);
  const updateMember = useUpdateOrgMember(orgId);
  const removeMember = useRemoveOrgMember(orgId);

  const roles = rolesData?.roles ?? [];
  const members = membersData?.members ?? [];

  const [email, setEmail] = useState('');
  const [roleId, setRoleId] = useState('');
  const [toRemove, setToRemove] = useState<OrgMember | null>(null);

  const fail = (err: unknown) =>
    showToast({
      type: 'error',
      title: t('common.error'),
      message: t(`members.errors.${memberErrorKey(err)}`),
    });

  const handleAdd = (e: React.FormEvent) => {
    e.preventDefault();
    const effectiveRole = roleId || roles[0]?.id;
    if (!email.trim() || !effectiveRole) return;
    addMember.mutate(
      { email: email.trim(), role_id: effectiveRole },
      {
        onSuccess: () => {
          setEmail('');
          showToast({ type: 'success', title: t('common.success'), message: t('members.added') });
        },
        onError: fail,
      }
    );
  };

  const handleRoleChange = (member: OrgMember, newRoleId: string) => {
    if (!newRoleId || newRoleId === member.role_id) return;
    updateMember.mutate(
      { userId: member.user_id, body: { role_id: newRoleId } },
      {
        onSuccess: () =>
          showToast({
            type: 'success',
            title: t('common.success'),
            message: t('members.roleChanged'),
          }),
        onError: fail,
      }
    );
  };

  const confirmRemove = () => {
    if (!toRemove) return;
    removeMember.mutate(toRemove.user_id, {
      onSuccess: () => {
        setToRemove(null);
        showToast({ type: 'success', title: t('common.success'), message: t('members.removed') });
      },
      onError: (err) => {
        setToRemove(null);
        fail(err);
      },
    });
  };

  return (
    <div className="max-w-5xl mx-auto px-4 py-8">
      <h1 className="text-2xl font-bold text-gray-900">{t('members.title')}</h1>
      <p className="mt-1 text-gray-600">{t('members.subtitle')}</p>

      <form
        onSubmit={handleAdd}
        className="mt-6 bg-white rounded-lg shadow p-4 grid grid-cols-1 sm:grid-cols-[1fr_auto_auto] gap-3 items-end"
        aria-label={t('members.addTitle')}
      >
        <div>
          <label htmlFor="member-email" className="block text-sm font-medium text-gray-700 mb-1">
            {t('members.email')}
          </label>
          <input
            id="member-email"
            type="email"
            autoComplete="off"
            className={inputClass}
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder={t('members.emailPlaceholder')}
          />
        </div>
        <div>
          <label htmlFor="member-role" className="block text-sm font-medium text-gray-700 mb-1">
            {t('members.role')}
          </label>
          <select
            id="member-role"
            className={inputClass}
            value={roleId || roles[0]?.id || ''}
            onChange={(e) => setRoleId(e.target.value)}
          >
            {roles.map((r) => (
              <option key={r.id} value={r.id}>
                {r.name}
              </option>
            ))}
          </select>
        </div>
        <button
          type="submit"
          disabled={addMember.isPending || !email.trim() || roles.length === 0}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {addMember.isPending ? t('members.adding') : t('members.add')}
        </button>
        <p className="sm:col-span-3 text-xs text-gray-500">{t('members.addHint')}</p>
      </form>

      <div className="mt-6 bg-white rounded-lg shadow overflow-x-auto">
        {isLoading ? (
          <div className="p-6 text-gray-500" role="status">
            {t('common.loading')}
          </div>
        ) : error ? (
          <div className="p-6 text-red-700" role="alert">
            {t('members.errors.loadFailed')}
          </div>
        ) : members.length === 0 ? (
          <div className="p-6 text-gray-500">{t('members.empty')}</div>
        ) : (
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th scope="col" className="px-4 py-3 text-left text-sm font-medium text-gray-600">
                  {t('members.name')}
                </th>
                <th scope="col" className="px-4 py-3 text-left text-sm font-medium text-gray-600">
                  {t('members.email')}
                </th>
                <th scope="col" className="px-4 py-3 text-left text-sm font-medium text-gray-600">
                  {t('members.role')}
                </th>
                <th scope="col" className="px-4 py-3">
                  <span className="sr-only">{t('members.actions')}</span>
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {members.map((m) => {
                const isSelf = m.user_id === user?.id;
                return (
                  <tr key={m.user_id}>
                    <td className="px-4 py-3 text-sm text-gray-900">{m.name}</td>
                    <td className="px-4 py-3 text-sm text-gray-600">{m.email}</td>
                    <td className="px-4 py-3 text-sm">
                      {isSelf || roles.length === 0 ? (
                        m.role_name
                      ) : (
                        <select
                          aria-label={t('members.changeRoleFor', { name: m.name })}
                          className="rounded-md border border-gray-300 px-2 py-1"
                          value={m.role_id ?? ''}
                          disabled={updateMember.isPending}
                          onChange={(e) => handleRoleChange(m, e.target.value)}
                        >
                          {!m.role_id && <option value="">{m.role_name}</option>}
                          {roles.map((r) => (
                            <option key={r.id} value={r.id}>
                              {r.name}
                            </option>
                          ))}
                        </select>
                      )}
                    </td>
                    <td className="px-4 py-3 text-right">
                      {!isSelf && (
                        <button
                          type="button"
                          onClick={() => setToRemove(m)}
                          className="text-sm text-red-600 hover:text-red-800"
                        >
                          {t('members.remove')}
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      <ConfirmationDialog
        isOpen={!!toRemove}
        title={t('members.removeTitle')}
        message={t('members.removeMessage', { name: toRemove?.name ?? '' })}
        confirmLabel={t('members.remove')}
        cancelLabel={t('common.cancel')}
        variant="danger"
        isLoading={removeMember.isPending}
        onConfirm={confirmRemove}
        onCancel={() => setToRemove(null)}
      />
    </div>
  );
}

OrganizationMembersPage.displayName = 'OrganizationMembersPage';
