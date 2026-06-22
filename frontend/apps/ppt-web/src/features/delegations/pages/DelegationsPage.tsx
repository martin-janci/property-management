/**
 * Delegations management page (Epic 3, Story 3.4).
 *
 * Lets a unit owner delegate voting / document / fault / financial rights to
 * another user, and lets a delegate accept or decline delegations they
 * received. Wires the full backend surface
 * (`backend/servers/api-server/src/routes/delegations.rs`) into ppt-web:
 *
 *   POST   /api/v1/delegations            — create        (useCreateDelegation)
 *   GET    /api/v1/delegations            — list mine      (useMyDelegations)
 *   GET    /api/v1/delegations/received   — list received  (useReceivedDelegations)
 *   POST   /api/v1/delegations/{id}/accept   — accept      (useAcceptDelegation)
 *   POST   /api/v1/delegations/{id}/decline  — decline     (useDeclineDelegation)
 *   DELETE /api/v1/delegations/{id}/revoke   — revoke      (useRevokeDelegation)
 */

import {
  DELEGATION_SCOPES,
  type DelegationScope,
  type DelegationSummary,
  useAcceptDelegation,
  useCreateDelegation,
  useDeclineDelegation,
  useMyDelegations,
  useReceivedDelegations,
  useRevokeDelegation,
} from '@ppt/api-client';
import type React from 'react';
import { useState } from 'react';
import { useToast } from '../../../components';

/** Map a delegation status to a small badge style. */
function statusBadgeClass(status: string): string {
  switch (status) {
    case 'active':
      return 'bg-green-100 text-green-800';
    case 'pending':
      return 'bg-yellow-100 text-yellow-800';
    case 'declined':
    case 'revoked':
    case 'expired':
      return 'bg-gray-100 text-gray-600';
    default:
      return 'bg-gray-100 text-gray-600';
  }
}

function ScopeBadges({ scopes }: { scopes: string[] }) {
  if (scopes.length === 0) return null;
  return (
    <span className="flex flex-wrap gap-1" aria-label={`Scopes: ${scopes.join(', ')}`}>
      {scopes.map((scope) => (
        <span
          key={scope}
          className="rounded bg-blue-50 px-2 py-0.5 text-xs font-medium text-blue-700"
        >
          {scope}
        </span>
      ))}
    </span>
  );
}

/** Shared visual shell for one delegation row. */
function DelegationRow({
  delegation,
  subjectLabel,
  subjectId,
  actions,
}: {
  delegation: DelegationSummary;
  subjectLabel: string;
  subjectId: string;
  actions?: React.ReactNode;
}) {
  return (
    <li className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-gray-200 p-4">
      <div className="min-w-0 space-y-1">
        <div className="flex items-center gap-2">
          <span
            className={`rounded-full px-2 py-0.5 text-xs font-medium ${statusBadgeClass(
              delegation.status
            )}`}
          >
            {delegation.status}
          </span>
          <span className="text-sm text-gray-700">
            {subjectLabel}: <span className="font-mono text-xs">{subjectId}</span>
          </span>
        </div>
        <div className="text-sm text-gray-600">
          Unit:{' '}
          {delegation.unit_id ? (
            <span className="font-mono text-xs">{delegation.unit_id}</span>
          ) : (
            <span className="italic">all units</span>
          )}
        </div>
        <ScopeBadges scopes={delegation.scopes} />
      </div>
      {actions && <div className="flex shrink-0 gap-2">{actions}</div>}
    </li>
  );
}

/** Create-delegation form. */
function CreateDelegationForm() {
  const { showToast } = useToast();
  const createDelegation = useCreateDelegation();
  const [delegateUserId, setDelegateUserId] = useState('');
  const [unitId, setUnitId] = useState('');
  const [scopes, setScopes] = useState<DelegationScope[]>(['all']);
  const [startDate, setStartDate] = useState('');
  const [endDate, setEndDate] = useState('');

  const toggleScope = (scope: DelegationScope) => {
    setScopes((prev) =>
      prev.includes(scope) ? prev.filter((s) => s !== scope) : [...prev, scope]
    );
  };

  const reset = () => {
    setDelegateUserId('');
    setUnitId('');
    setScopes(['all']);
    setStartDate('');
    setEndDate('');
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!delegateUserId.trim()) {
      showToast({
        type: 'error',
        title: 'Delegate required',
        message: 'Enter a delegate user ID.',
      });
      return;
    }
    if (scopes.length === 0) {
      showToast({ type: 'error', title: 'Scope required', message: 'Select at least one scope.' });
      return;
    }
    try {
      await createDelegation.mutateAsync({
        delegate_user_id: delegateUserId.trim(),
        unit_id: unitId.trim() || null,
        scopes,
        start_date: startDate || null,
        end_date: endDate || null,
      });
      showToast({
        type: 'success',
        title: 'Delegation created',
        message: 'Invitation sent to the delegate.',
      });
      reset();
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Failed to create delegation',
        message: err instanceof Error ? err.message : 'An unexpected error occurred.',
      });
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4 rounded-lg border border-gray-200 p-4">
      <h2 className="text-lg font-semibold">Delegate your rights</h2>
      <div className="grid gap-4 sm:grid-cols-2">
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium">Delegate user ID *</span>
          <input
            type="text"
            value={delegateUserId}
            onChange={(e) => setDelegateUserId(e.target.value)}
            required
            placeholder="user UUID"
            className="rounded-lg border border-gray-300 px-3 py-2"
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium">Unit ID (optional)</span>
          <input
            type="text"
            value={unitId}
            onChange={(e) => setUnitId(e.target.value)}
            placeholder="leave blank for all owned units"
            className="rounded-lg border border-gray-300 px-3 py-2"
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium">Start date (optional)</span>
          <input
            type="date"
            value={startDate}
            onChange={(e) => setStartDate(e.target.value)}
            className="rounded-lg border border-gray-300 px-3 py-2"
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium">End date (optional)</span>
          <input
            type="date"
            value={endDate}
            onChange={(e) => setEndDate(e.target.value)}
            className="rounded-lg border border-gray-300 px-3 py-2"
          />
        </label>
      </div>
      <fieldset>
        <legend className="text-sm font-medium">Scopes</legend>
        <div className="mt-2 flex flex-wrap gap-3">
          {DELEGATION_SCOPES.map((scope) => (
            <label key={scope} className="flex items-center gap-1.5 text-sm">
              <input
                type="checkbox"
                checked={scopes.includes(scope)}
                onChange={() => toggleScope(scope)}
              />
              {scope}
            </label>
          ))}
        </div>
      </fieldset>
      <button
        type="submit"
        disabled={createDelegation.isPending}
        className="btn-primary-token rounded-lg px-4 py-2 text-sm font-medium"
      >
        {createDelegation.isPending ? 'Creating…' : 'Create delegation'}
      </button>
    </form>
  );
}

/** Section that lists delegations the current user received (as delegate). */
function ReceivedDelegationsSection() {
  const { showToast } = useToast();
  const { data, isLoading, isError, error, refetch } = useReceivedDelegations();
  const accept = useAcceptDelegation();
  const decline = useDeclineDelegation();

  const run = async (mutate: (id: string) => Promise<unknown>, id: string, verb: string) => {
    try {
      await mutate(id);
      showToast({
        type: 'success',
        title: `Delegation ${verb}`,
        message: `You ${verb} the delegation.`,
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: `Failed to ${verb.replace(/ed$/, '')} delegation`,
        message: err instanceof Error ? err.message : 'An unexpected error occurred.',
      });
    }
  };

  if (isLoading) {
    return (
      <p role="status" aria-busy="true" className="text-sm text-gray-600">
        Loading received delegations…
      </p>
    );
  }
  if (isError) {
    return (
      <div role="alert" className="space-y-2 text-sm">
        <p className="text-red-700">
          {error instanceof Error ? error.message : 'Failed to load received delegations.'}
        </p>
        <button
          type="button"
          onClick={() => refetch()}
          className="btn-outline-token rounded-lg px-3 py-1.5"
        >
          Retry
        </button>
      </div>
    );
  }

  const list = data ?? [];
  if (list.length === 0) {
    return (
      <p role="status" className="text-sm text-gray-600">
        No one has delegated rights to you yet.
      </p>
    );
  }

  return (
    <ul className="space-y-3" aria-label="Delegations received">
      {list.map((d) => (
        <DelegationRow
          key={d.id}
          delegation={d}
          subjectLabel="From owner"
          subjectId={d.owner_user_id}
          actions={
            d.status === 'pending' ? (
              <>
                <button
                  type="button"
                  onClick={() => run(accept.mutateAsync, d.id, 'accepted')}
                  disabled={accept.isPending || decline.isPending}
                  className="btn-primary-token rounded-lg px-3 py-1.5 text-sm font-medium"
                >
                  Accept
                </button>
                <button
                  type="button"
                  onClick={() => run(decline.mutateAsync, d.id, 'declined')}
                  disabled={accept.isPending || decline.isPending}
                  className="btn-outline-token rounded-lg px-3 py-1.5 text-sm font-medium"
                >
                  Decline
                </button>
              </>
            ) : undefined
          }
        />
      ))}
    </ul>
  );
}

/** Section that lists delegations the current user created (as owner). */
function MyDelegationsSection() {
  const { showToast } = useToast();
  const { data, isLoading, isError, error, refetch } = useMyDelegations();
  const revoke = useRevokeDelegation();

  const handleRevoke = async (id: string) => {
    try {
      await revoke.mutateAsync(id);
      showToast({
        type: 'success',
        title: 'Delegation revoked',
        message: 'The delegate no longer has access.',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Failed to revoke delegation',
        message: err instanceof Error ? err.message : 'An unexpected error occurred.',
      });
    }
  };

  if (isLoading) {
    return (
      <p role="status" aria-busy="true" className="text-sm text-gray-600">
        Loading your delegations…
      </p>
    );
  }
  if (isError) {
    return (
      <div role="alert" className="space-y-2 text-sm">
        <p className="text-red-700">
          {error instanceof Error ? error.message : 'Failed to load your delegations.'}
        </p>
        <button
          type="button"
          onClick={() => refetch()}
          className="btn-outline-token rounded-lg px-3 py-1.5"
        >
          Retry
        </button>
      </div>
    );
  }

  const list = data ?? [];
  if (list.length === 0) {
    return (
      <p role="status" className="text-sm text-gray-600">
        You haven&apos;t delegated any rights yet.
      </p>
    );
  }

  const revocable = new Set(['pending', 'active']);

  return (
    <ul className="space-y-3" aria-label="Delegations created">
      {list.map((d) => (
        <DelegationRow
          key={d.id}
          delegation={d}
          subjectLabel="To delegate"
          subjectId={d.delegate_user_id}
          actions={
            revocable.has(d.status) ? (
              <button
                type="button"
                onClick={() => handleRevoke(d.id)}
                disabled={revoke.isPending}
                className="btn-outline-token rounded-lg px-3 py-1.5 text-sm font-medium"
                aria-label={`Revoke delegation to ${d.delegate_user_id}`}
              >
                Revoke
              </button>
            ) : undefined
          }
        />
      ))}
    </ul>
  );
}

export const DelegationsPage: React.FC = () => {
  return (
    <div className="mx-auto max-w-3xl space-y-8 p-4">
      <header className="space-y-1">
        <h1 className="text-2xl font-bold">Delegations</h1>
        <p className="text-sm text-gray-600">
          Delegate your ownership rights (voting, documents, faults, financial) to another user, or
          manage delegations others have shared with you.
        </p>
      </header>

      <CreateDelegationForm />

      <section className="space-y-3" aria-labelledby="received-heading">
        <h2 id="received-heading" className="text-lg font-semibold">
          Received delegations
        </h2>
        <ReceivedDelegationsSection />
      </section>

      <section className="space-y-3" aria-labelledby="mine-heading">
        <h2 id="mine-heading" className="text-lg font-semibold">
          Delegations you created
        </h2>
        <MyDelegationsSection />
      </section>
    </div>
  );
};
