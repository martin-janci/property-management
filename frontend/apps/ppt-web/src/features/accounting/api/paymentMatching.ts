/**
 * Data layer for the payment-matching screen (#1521).
 *
 * The JSON reads/decisions go through the generated `@ppt/api-client` functions
 * now that the accounting bank-statement / payment-matching endpoints are modelled
 * in TypeSpec (#1623) — so request/response types come from the contract and auth
 * headers are injected by the centralized request interceptor (#1616). Only the
 * multipart statement upload stays a raw fetch (the generated client serialises
 * JSON bodies, and the upload isn't modelled), but it draws auth from the SAME
 * token / active-org providers — never the bespoke localStorage keys the original
 * reality-web page invented (which sent `Bearer null`).
 */

import {
  type AccountingBankStatement,
  type AccountingBankStatementLine,
  type AccountingPaymentMatch,
  getOrg,
  getToken,
  paymentMatchesApiConfirm,
  paymentMatchesApiReject,
  statementLinesApiListMatches,
  statementsApiList,
  statementsApiListLines,
} from '@ppt/api-client';

export type { AccountingBankStatement, AccountingBankStatementLine, AccountingPaymentMatch };

export async function fetchStatements(): Promise<AccountingBankStatement[]> {
  const { data } = await statementsApiList({ throwOnError: true });
  return data ?? [];
}

export async function fetchStatementLines(
  statementId: string
): Promise<AccountingBankStatementLine[]> {
  const { data } = await statementsApiListLines({
    path: { id: statementId },
    throwOnError: true,
  });
  return data ?? [];
}

export async function fetchLineMatches(lineId: string): Promise<AccountingPaymentMatch[]> {
  const { data } = await statementLinesApiListMatches({
    path: { id: lineId },
    throwOnError: true,
  });
  return data ?? [];
}

export async function confirmMatch(matchId: string): Promise<void> {
  await paymentMatchesApiConfirm({ path: { id: matchId }, throwOnError: true });
}

export async function rejectMatch(matchId: string): Promise<void> {
  await paymentMatchesApiReject({ path: { id: matchId }, throwOnError: true });
}

export async function uploadStatement(file: File): Promise<void> {
  const formData = new FormData();
  formData.append('file', file);
  const token = getToken();
  const org = getOrg();
  const headers: Record<string, string> = {};
  if (token) headers.Authorization = `Bearer ${token}`;
  if (org) headers['X-Tenant-ID'] = org;

  const res = await fetch(`${import.meta.env.VITE_API_URL || ''}/api/v1/accounting/statements`, {
    method: 'POST',
    body: formData,
    headers,
  });
  if (!res.ok) throw new Error('Upload failed');
}
