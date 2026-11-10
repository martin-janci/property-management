/**
 * Data layer for the payment-matching screen (#1521).
 *
 * The JSON reads/decisions go through the generated `@ppt/api-client` functions
 * now that the accounting bank-statement / payment-matching endpoints are modelled
 * in TypeSpec (#1623) — so request/response types come from the contract and auth
 * headers are injected by the centralized request interceptor (#1616). The
 * multipart statement upload stays a raw request (the generated client serialises
 * JSON bodies, and the upload isn't modelled), but it now goes through the shared
 * `authenticatedFetch` client (#3000) — so Authorization / X-Tenant-ID, base-URL
 * resolution and MFA retry are handled centrally, never hand-rolled here (the
 * original reality-web page invented localStorage keys that sent `Bearer null`).
 */

import {
  type AccountingBankStatement,
  type AccountingBankStatementLine,
  type AccountingPaymentMatch,
  authenticatedFetch,
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
  // Route the multipart upload through the shared authenticated client, exactly
  // like the generated SDK the JSON reads use: Authorization + X-Tenant-ID come
  // from the registered token/org providers and the base URL matches. No headers
  // are passed here so the browser keeps the multipart boundary, and the shared
  // client never forces a JSON `Content-Type`.
  const res = await authenticatedFetch('/api/v1/accounting/statements', {
    method: 'POST',
    body: formData,
  });
  if (!res.ok) throw new Error('Upload failed');
}
