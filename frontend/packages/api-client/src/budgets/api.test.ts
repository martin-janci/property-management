/**
 * Contract tests for the budgets API client (Epic 24).
 *
 * Pin path, method, query string and JSON body of every call against the
 * api-server routes in `backend/servers/api-server/src/routes/budgets.rs`, and
 * the snake_case → camelCase / decimal-string → number mapping.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({ authenticatedFetch: vi.fn() }));

vi.mock('../lib/fetch', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/fetch')>()),
  authenticatedFetch: mocks.authenticatedFetch,
}));

import { ApiError } from '../lib/fetch';
import {
  activateBudget,
  addBudgetItem,
  approveBudget,
  closeBudget,
  createBudget,
  createBudgetCategory,
  deleteBudget,
  deleteBudgetItem,
  getBudget,
  getBudgetSummary,
  getBudgetVariance,
  listBudgetActuals,
  listBudgetCategories,
  listBudgetItems,
  listBudgets,
  mapBudget,
  mapBudgetItem,
  mapBudgetSummary,
  parseBudgetAmount,
  recordBudgetActual,
  submitBudget,
  updateBudget,
  updateBudgetItem,
} from './api';
import type { RawBudget, RawBudgetItem } from './types';

const ORG = 'org-1';

const RAW_BUDGET: RawBudget = {
  id: 'bud-1',
  organization_id: ORG,
  building_id: 'bld-1',
  fiscal_year: 2026,
  name: 'Operating 2026',
  status: 'draft',
  total_amount: '12500.50',
  notes: null,
  approved_by: null,
  approved_at: null,
  created_by: 'user-1',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-02T00:00:00Z',
};

const RAW_ITEM: RawBudgetItem = {
  id: 'item-1',
  budget_id: 'bud-1',
  category_id: 'cat-1',
  name: 'Cleaning',
  description: null,
  budgeted_amount: '1000.00',
  actual_amount: '1200.00',
  variance_amount: '200.00',
  variance_percent: '20.00',
  notes: null,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

function respond(body: unknown, status = 200) {
  mocks.authenticatedFetch.mockResolvedValueOnce({
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response);
}

function lastCall(): { path: string; method: string; body: unknown; contentType?: string } {
  const calls = mocks.authenticatedFetch.mock.calls;
  const [path, init] = calls[calls.length - 1] as [string, RequestInit | undefined];
  const headers = (init?.headers ?? {}) as Record<string, string>;
  return {
    path,
    method: (init?.method ?? 'GET').toUpperCase(),
    body: init?.body === undefined ? undefined : JSON.parse(String(init.body)),
    contentType: headers['Content-Type'],
  };
}

beforeEach(() => {
  mocks.authenticatedFetch.mockReset();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('budgets — CRUD wiring', () => {
  it('listBudgets: GET /budgets with organization_id and only the set filters', async () => {
    respond([RAW_BUDGET]);
    const result = await listBudgets({
      organizationId: ORG,
      fiscalYear: 2026,
      buildingId: 'bld-1',
    });
    const call = lastCall();
    expect(call.method).toBe('GET');
    const url = new URL(call.path, 'http://x');
    expect(url.pathname).toBe('/api/v1/budgets');
    expect(Object.fromEntries(url.searchParams)).toEqual({
      organization_id: ORG,
      building_id: 'bld-1',
      fiscal_year: '2026',
    });
    expect(result[0].totalAmount).toBe(12500.5);
  });

  it('getBudget: GET /budgets/{id}?organization_id', async () => {
    respond(RAW_BUDGET);
    await getBudget(ORG, 'bud-1');
    expect(lastCall()).toMatchObject({
      path: '/api/v1/budgets/bud-1?organization_id=org-1',
      method: 'GET',
      body: undefined,
    });
  });

  it('createBudget: POST /budgets with the flattened snake_case body', async () => {
    respond(RAW_BUDGET, 201);
    const created = await createBudget({
      organizationId: ORG,
      fiscalYear: 2026,
      name: 'Operating 2026',
      buildingId: 'bld-1',
      notes: 'n',
    });
    expect(lastCall()).toEqual({
      path: '/api/v1/budgets',
      method: 'POST',
      contentType: 'application/json',
      body: {
        organization_id: ORG,
        building_id: 'bld-1',
        fiscal_year: 2026,
        name: 'Operating 2026',
        notes: 'n',
      },
    });
    expect(created.id).toBe('bud-1');
  });

  it('createBudget: sends null building_id / notes when omitted', async () => {
    respond(RAW_BUDGET, 201);
    await createBudget({ organizationId: ORG, fiscalYear: 2027, name: 'Org-wide' });
    expect(lastCall().body).toEqual({
      organization_id: ORG,
      building_id: null,
      fiscal_year: 2027,
      name: 'Org-wide',
      notes: null,
    });
  });

  it('updateBudget: PUT /budgets/{id} with organization_id and only changed fields', async () => {
    respond(RAW_BUDGET);
    await updateBudget('bud-1', { organizationId: ORG, name: 'Renamed' });
    expect(lastCall()).toMatchObject({
      path: '/api/v1/budgets/bud-1',
      method: 'PUT',
      body: { organization_id: ORG, name: 'Renamed' },
    });
  });

  it('deleteBudget: DELETE /budgets/{id}?organization_id and tolerates 204', async () => {
    respond(undefined, 204);
    await expect(deleteBudget(ORG, 'bud-1')).resolves.toBeUndefined();
    expect(lastCall()).toMatchObject({
      path: '/api/v1/budgets/bud-1?organization_id=org-1',
      method: 'DELETE',
    });
  });

  it.each([
    ['submit', submitBudget],
    ['approve', approveBudget],
    ['activate', activateBudget],
    ['close', closeBudget],
  ] as const)('%sBudget: POST /budgets/{id}/%s?organization_id', async (action, fn) => {
    respond({ ...RAW_BUDGET, status: 'active' });
    const result = await fn(ORG, 'bud-1');
    expect(lastCall()).toMatchObject({
      path: `/api/v1/budgets/bud-1/${action}?organization_id=org-1`,
      method: 'POST',
      body: undefined,
    });
    expect(result.status).toBe('active');
  });

  it('throws ApiError carrying status, code and message on failure', async () => {
    respond({ code: 'INVALID_STATE', message: 'Only draft budgets can be deleted' }, 400);
    const error = await deleteBudget(ORG, 'bud-1').catch((e) => e);
    expect(error).toBeInstanceOf(ApiError);
    expect(error.status).toBe(400);
    expect(error.code).toBe('INVALID_STATE');
    expect(error.message).toBe('Only draft budgets can be deleted');
  });
});

describe('budgets — reporting', () => {
  it('getBudgetSummary: GET /budgets/{id}/summary, mapped to numbers', async () => {
    respond({
      total_budgeted: '1000.00',
      total_actual: '1200.00',
      total_variance: '200.00',
      variance_percent: '20.00',
      items_over_budget: 1,
      items_under_budget: 0,
    });
    const summary = await getBudgetSummary('bud-1');
    expect(lastCall()).toMatchObject({ path: '/api/v1/budgets/bud-1/summary', method: 'GET' });
    expect(summary).toEqual({
      totalBudgeted: 1000,
      totalActual: 1200,
      totalVariance: 200,
      variancePercent: 20,
      itemsOverBudget: 1,
      itemsUnderBudget: 0,
    });
  });

  it('getBudgetVariance: GET /budgets/{id}/variance', async () => {
    respond([
      {
        category_id: 'cat-1',
        category_name: 'Services',
        budgeted_amount: '10',
        actual_amount: '5',
        variance_amount: '-5',
        variance_percent: '-50',
      },
    ]);
    const rows = await getBudgetVariance('bud-1');
    expect(lastCall()).toMatchObject({ path: '/api/v1/budgets/bud-1/variance', method: 'GET' });
    expect(rows[0]).toMatchObject({ categoryName: 'Services', varianceAmount: -5 });
  });
});

describe('budgets — items, actuals, categories', () => {
  it('listBudgetItems: GET /budgets/{id}/items', async () => {
    respond([RAW_ITEM]);
    const items = await listBudgetItems('bud-1');
    expect(lastCall()).toMatchObject({ path: '/api/v1/budgets/bud-1/items', method: 'GET' });
    expect(items[0].budgetedAmount).toBe(1000);
  });

  it('addBudgetItem: POST /budgets/{id}/items with the amount as a decimal string', async () => {
    respond(RAW_ITEM, 201);
    await addBudgetItem('bud-1', { categoryId: 'cat-1', name: 'Cleaning', budgetedAmount: 1000 });
    expect(lastCall()).toMatchObject({
      path: '/api/v1/budgets/bud-1/items',
      method: 'POST',
      body: {
        category_id: 'cat-1',
        name: 'Cleaning',
        description: null,
        budgeted_amount: '1000.00',
        notes: null,
      },
    });
  });

  it('updateBudgetItem: PUT /budgets/items/{itemId} with only changed fields', async () => {
    respond(RAW_ITEM);
    await updateBudgetItem('item-1', { budgetedAmount: 99.5 });
    expect(lastCall()).toMatchObject({
      path: '/api/v1/budgets/items/item-1',
      method: 'PUT',
      body: { budgeted_amount: '99.50' },
    });
  });

  it('deleteBudgetItem: DELETE /budgets/items/{itemId}', async () => {
    respond(undefined, 204);
    await deleteBudgetItem('item-1');
    expect(lastCall()).toMatchObject({ path: '/api/v1/budgets/items/item-1', method: 'DELETE' });
  });

  it('listBudgetActuals / recordBudgetActual: /budgets/items/{itemId}/actuals', async () => {
    respond([]);
    await listBudgetActuals('item-1');
    expect(lastCall()).toMatchObject({
      path: '/api/v1/budgets/items/item-1/actuals',
      method: 'GET',
    });

    respond(
      {
        id: 'act-1',
        budget_item_id: 'item-1',
        transaction_id: null,
        amount: '50.00',
        description: null,
        transaction_date: '2026-03-01',
        recorded_by: 'user-1',
        created_at: '2026-03-01T00:00:00Z',
      },
      201
    );
    const actual = await recordBudgetActual('item-1', {
      amount: 50,
      transactionDate: '2026-03-01',
    });
    expect(lastCall()).toMatchObject({
      path: '/api/v1/budgets/items/item-1/actuals',
      method: 'POST',
      body: {
        amount: '50.00',
        transaction_date: '2026-03-01',
        description: null,
        transaction_id: null,
      },
    });
    expect(actual).toMatchObject({ amount: 50, budgetItemId: 'item-1' });
  });

  it('listBudgetCategories: GET /budgets/categories?organization_id', async () => {
    respond([
      {
        id: 'cat-1',
        organization_id: ORG,
        name: 'Services',
        description: null,
        parent_id: null,
        sort_order: 0,
        is_system: false,
        created_at: '2026-01-01T00:00:00Z',
      },
    ]);
    const cats = await listBudgetCategories(ORG);
    expect(lastCall()).toMatchObject({
      path: '/api/v1/budgets/categories?organization_id=org-1',
      method: 'GET',
    });
    expect(cats[0]).toMatchObject({ id: 'cat-1', isSystem: false, sortOrder: 0 });
  });

  it('createBudgetCategory: POST /budgets/categories with organization_id in the body', async () => {
    respond({ id: 'cat-2', organization_id: ORG, name: 'Energy', created_at: 'x' }, 201);
    await createBudgetCategory({ organizationId: ORG, name: 'Energy' });
    expect(lastCall()).toMatchObject({
      path: '/api/v1/budgets/categories',
      method: 'POST',
      body: { organization_id: ORG, name: 'Energy', description: null, parent_id: null },
    });
  });
});

describe('budgets — mappers', () => {
  it('mapBudget converts snake_case and decimal strings', () => {
    expect(mapBudget(RAW_BUDGET)).toEqual({
      id: 'bud-1',
      organizationId: ORG,
      buildingId: 'bld-1',
      fiscalYear: 2026,
      name: 'Operating 2026',
      status: 'draft',
      totalAmount: 12500.5,
      notes: null,
      approvedBy: null,
      approvedAt: null,
      createdBy: 'user-1',
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-02T00:00:00Z',
    });
  });

  it('mapBudgetItem keeps the backend variance sign (actual - budgeted)', () => {
    expect(mapBudgetItem(RAW_ITEM)).toMatchObject({
      budgetId: 'bud-1',
      categoryId: 'cat-1',
      budgetedAmount: 1000,
      actualAmount: 1200,
      varianceAmount: 200,
      variancePercent: 20,
    });
  });

  it('mapBudgetSummary defaults missing counts to 0', () => {
    expect(
      mapBudgetSummary({
        total_budgeted: '0',
        total_actual: '0',
        total_variance: '0',
        variance_percent: '0',
      } as never)
    ).toMatchObject({ itemsOverBudget: 0, itemsUnderBudget: 0 });
  });

  it('parseBudgetAmount handles strings, numbers, null and garbage', () => {
    expect(parseBudgetAmount('10.25')).toBe(10.25);
    expect(parseBudgetAmount(3)).toBe(3);
    expect(parseBudgetAmount(null)).toBe(0);
    expect(parseBudgetAmount('abc')).toBe(0);
  });
});
