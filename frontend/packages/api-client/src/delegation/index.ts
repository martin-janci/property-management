/**
 * Delegation module (Epic 3, Story 3.4).
 *
 * Ownership delegation: a unit owner delegates voting / document / fault /
 * financial rights to another user. Wires to `/api/v1/delegations` on
 * api-server (`routes/delegations.rs`).
 */

export {
  acceptDelegation,
  checkDelegation,
  createDelegation,
  declineDelegation,
  getDelegation,
  listMyDelegations,
  listReceivedDelegations,
  revokeDelegation,
} from './api';
export {
  delegationKeys,
  useAcceptDelegation,
  useCreateDelegation,
  useDeclineDelegation,
  useDelegation,
  useMyDelegations,
  useReceivedDelegations,
  useRevokeDelegation,
} from './hooks';
export {
  type CheckDelegationResult,
  type CreateDelegationRequest,
  DELEGATION_SCOPES,
  type Delegation,
  type DelegationScope,
  type DelegationStatus,
  type DelegationSummary,
  type ListDelegationsQuery,
} from './types';
