/**
 * Data layer for the payment-matching screen (#1521).
 *
 * All calls go to the manager-only api-server accounting endpoints. JSON calls go
 * through the shared `@ppt/api-client` client, so the `Authorization` /
 * `X-Tenant-ID` headers are injected by the centralized request interceptor
 * (#1616). The multipart statement upload uses a raw fetch (the generated client
 * serialises JSON bodies) but draws auth from the SAME providers — never the
 * bespoke `auth_token` / `current_tenant_id` localStorage keys the original
 * reality-web page invented (which sent `Bearer null`).
 */

import { client, getOrg, getToken } from '@ppt/api-client';

export interface BankStatement {
  id: string;
  source_filename: string;
  imported_at: string;
  account_iban: string;
}

export interface BankStatementLine {
  id: string;
  statement_id: string;
  booking_date: string;
  amount: string;
  currency: string;
  counterparty_iban?: string;
  variable_symbol?: string;
  raw_ref?: string;
  match_state: 'unmatched' | 'suggested' | 'matched';
}

export interface PaymentMatch {
  id: string;
  statement_line_id: string;
  invoice_id: string;
  /** Decimal arrives as a string. */
  confidence: string;
  state: 'suggested' | 'confirmed' | 'rejected';
}

export async function fetchStatements(): Promise<BankStatement[]> {
  const { data } = await client.get<BankStatement[], unknown, true>({
    url: '/api/v1/accounting/statements',
    throwOnError: true,
  });
  return data ?? [];
}

export async function fetchStatementLines(statementId: string): Promise<BankStatementLine[]> {
  const { data } = await client.get<BankStatementLine[], unknown, true>({
    url: `/api/v1/accounting/statements/${statementId}/lines`,
    throwOnError: true,
  });
  return data ?? [];
}

export async function fetchLineMatches(lineId: string): Promise<PaymentMatch[]> {
  const { data } = await client.get<PaymentMatch[], unknown, true>({
    url: `/api/v1/accounting/lines/${lineId}/matches`,
    throwOnError: true,
  });
  return data ?? [];
}

export async function confirmMatch(matchId: string): Promise<void> {
  await client.post<unknown, unknown, true>({
    url: `/api/v1/accounting/matches/${matchId}/confirm`,
    throwOnError: true,
  });
}

export async function rejectMatch(matchId: string): Promise<void> {
  await client.post<unknown, unknown, true>({
    url: `/api/v1/accounting/matches/${matchId}/reject`,
    throwOnError: true,
  });
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
