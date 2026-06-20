/**
 * Tests for the payment-matching data layer (#1521 / #1623).
 *
 * Reads/decisions now delegate to the generated `@ppt/api-client` functions (the
 * accounting endpoints are modelled in TypeSpec), so those are mocked here. The
 * headline regression remains the upload: it must authenticate from the shared
 * token / active-org providers (real session) — not the never-written
 * `auth_token` / `current_tenant_id` localStorage keys the original reality-web
 * page used, which sent `Bearer null`.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  statementsApiList: vi.fn(),
  statementsApiListLines: vi.fn(),
  statementLinesApiListMatches: vi.fn(),
  paymentMatchesApiConfirm: vi.fn(),
  paymentMatchesApiReject: vi.fn(),
  getToken: vi.fn(),
  getOrg: vi.fn(),
}));

vi.mock('@ppt/api-client', () => ({
  statementsApiList: mocks.statementsApiList,
  statementsApiListLines: mocks.statementsApiListLines,
  statementLinesApiListMatches: mocks.statementLinesApiListMatches,
  paymentMatchesApiConfirm: mocks.paymentMatchesApiConfirm,
  paymentMatchesApiReject: mocks.paymentMatchesApiReject,
  getToken: mocks.getToken,
  getOrg: mocks.getOrg,
}));

import {
  confirmMatch,
  fetchLineMatches,
  fetchStatementLines,
  fetchStatements,
  rejectMatch,
  uploadStatement,
} from './paymentMatching';

beforeEach(() => {
  vi.clearAllMocks();
  mocks.statementsApiList.mockResolvedValue({ data: [] });
  mocks.statementsApiListLines.mockResolvedValue({ data: [] });
  mocks.statementLinesApiListMatches.mockResolvedValue({ data: [] });
  mocks.paymentMatchesApiConfirm.mockResolvedValue({ data: undefined });
  mocks.paymentMatchesApiReject.mockResolvedValue({ data: undefined });
});

describe('payment-matching reads', () => {
  it('fetchStatements calls statementsApiList (throwing) and unwraps data', async () => {
    mocks.statementsApiList.mockResolvedValueOnce({ data: [{ id: 's1' }] });
    const result = await fetchStatements();

    expect(mocks.statementsApiList).toHaveBeenCalledWith(
      expect.objectContaining({ throwOnError: true })
    );
    expect(result).toEqual([{ id: 's1' }]);
  });

  it('fetchStatements falls back to [] when the body is empty', async () => {
    mocks.statementsApiList.mockResolvedValueOnce({ data: undefined });
    expect(await fetchStatements()).toEqual([]);
  });

  it('fetchStatementLines scopes by statement id (path param)', async () => {
    await fetchStatementLines('stmt-9');
    expect(mocks.statementsApiListLines).toHaveBeenCalledWith(
      expect.objectContaining({ path: { id: 'stmt-9' }, throwOnError: true })
    );
  });

  it('fetchLineMatches scopes by line id (path param)', async () => {
    await fetchLineMatches('line-7');
    expect(mocks.statementLinesApiListMatches).toHaveBeenCalledWith(
      expect.objectContaining({ path: { id: 'line-7' }, throwOnError: true })
    );
  });
});

describe('payment-matching mutations', () => {
  it('confirmMatch confirms the given match id', async () => {
    await confirmMatch('m-1');
    expect(mocks.paymentMatchesApiConfirm).toHaveBeenCalledWith(
      expect.objectContaining({ path: { id: 'm-1' }, throwOnError: true })
    );
  });

  it('rejectMatch rejects the given match id', async () => {
    await rejectMatch('m-2');
    expect(mocks.paymentMatchesApiReject).toHaveBeenCalledWith(
      expect.objectContaining({ path: { id: 'm-2' }, throwOnError: true })
    );
  });
});

describe('uploadStatement auth wiring (the #1521 bug)', () => {
  it('uploads with Authorization + X-Tenant-ID from the real providers, as multipart', async () => {
    mocks.getToken.mockReturnValue('real-token');
    mocks.getOrg.mockReturnValue('org-123');
    const fetchMock = vi.fn().mockResolvedValue({ ok: true });
    vi.stubGlobal('fetch', fetchMock);

    const file = new File(['a,b,c'], 'statement.csv', { type: 'text/csv' });
    await uploadStatement(file);

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toContain('/api/v1/accounting/statements');
    expect(init.method).toBe('POST');
    expect(init.headers.Authorization).toBe('Bearer real-token');
    expect(init.headers['X-Tenant-ID']).toBe('org-123');
    expect(init.body).toBeInstanceOf(FormData);
    // The old localStorage keys must NOT be the source of truth.
    expect(init.headers.Authorization).not.toContain('null');

    vi.unstubAllGlobals();
  });

  it('omits auth headers when there is no session rather than sending Bearer null', async () => {
    mocks.getToken.mockReturnValue(null);
    mocks.getOrg.mockReturnValue(null);
    const fetchMock = vi.fn().mockResolvedValue({ ok: true });
    vi.stubGlobal('fetch', fetchMock);

    await uploadStatement(new File(['x'], 's.csv', { type: 'text/csv' }));

    const [, init] = fetchMock.mock.calls[0];
    expect(init.headers.Authorization).toBeUndefined();
    expect(init.headers['X-Tenant-ID']).toBeUndefined();

    vi.unstubAllGlobals();
  });

  it('throws when the upload response is not ok', async () => {
    mocks.getToken.mockReturnValue('t');
    mocks.getOrg.mockReturnValue('o');
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false }));

    await expect(uploadStatement(new File(['x'], 's.csv'))).rejects.toThrow('Upload failed');

    vi.unstubAllGlobals();
  });
});
