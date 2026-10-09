/**
 * Tests for the payment-matching data layer (#1521 / #1623).
 *
 * Reads/decisions now delegate to the generated `@ppt/api-client` functions (the
 * accounting endpoints are modelled in TypeSpec), so those are mocked here. The
 * headline regression remains the upload: it must go through the shared
 * `authenticatedFetch` client (#3000) — so Authorization / X-Tenant-ID come from
 * the registered token/org providers and are never hand-rolled here (the original
 * reality-web page invented localStorage keys that sent `Bearer null`).
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  statementsApiList: vi.fn(),
  statementsApiListLines: vi.fn(),
  statementLinesApiListMatches: vi.fn(),
  paymentMatchesApiConfirm: vi.fn(),
  paymentMatchesApiReject: vi.fn(),
  authenticatedFetch: vi.fn(),
}));

vi.mock('@ppt/api-client', () => ({
  statementsApiList: mocks.statementsApiList,
  statementsApiListLines: mocks.statementsApiListLines,
  statementLinesApiListMatches: mocks.statementLinesApiListMatches,
  paymentMatchesApiConfirm: mocks.paymentMatchesApiConfirm,
  paymentMatchesApiReject: mocks.paymentMatchesApiReject,
  authenticatedFetch: mocks.authenticatedFetch,
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
  mocks.authenticatedFetch.mockResolvedValue({ ok: true } as Response);
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

describe('uploadStatement routes through the shared authenticated client (#3000)', () => {
  it('POSTs multipart FormData via authenticatedFetch — not a hand-rolled fetch', async () => {
    // If uploadStatement fell back to a bare `fetch`, this stub would trip and
    // the delegation assertions below would fail.
    const bareFetch = vi.fn();
    vi.stubGlobal('fetch', bareFetch);

    const file = new File(['a,b,c'], 'statement.csv', { type: 'text/csv' });
    await uploadStatement(file);

    expect(bareFetch).not.toHaveBeenCalled();
    expect(mocks.authenticatedFetch).toHaveBeenCalledTimes(1);
    const [url, init] = mocks.authenticatedFetch.mock.calls[0];
    expect(String(url)).toContain('/api/v1/accounting/statements');
    expect(init.method).toBe('POST');
    expect(init.body).toBeInstanceOf(FormData);
    // Auth headers are the shared client's job now — uploadStatement must NOT
    // hand-roll Authorization / X-Tenant-ID (that path sent `Bearer null`).
    expect(init.headers).toBeUndefined();

    vi.unstubAllGlobals();
  });

  it('throws when the shared client reports a non-ok response', async () => {
    mocks.authenticatedFetch.mockResolvedValueOnce({ ok: false } as Response);

    await expect(uploadStatement(new File(['x'], 's.csv'))).rejects.toThrow('Upload failed');
  });
});
