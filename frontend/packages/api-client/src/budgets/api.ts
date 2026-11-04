/**
 * Budgets API client (Epic 24 — Budget & Financial Planning).
 *
 * Wraps the api-server `/api/v1/budgets` routes
 * (`backend/servers/api-server/src/routes/budgets.rs`):
 *
 *   GET    /budgets?organization_id&building_id&fiscal_year&status&limit&offset
 *   POST   /budgets                       { organization_id, building_id?, fiscal_year, name, notes? }
 *   GET    /budgets/{id}?organization_id
 *   PUT    /budgets/{id}                  { organization_id, name?, notes? }
 *   DELETE /budgets/{id}?organization_id  (draft only)
 *   POST   /budgets/{id}/{submit|approve|activate|close}?organization_id
 *   GET    /budgets/{id}/summary
 *   GET    /budgets/{id}/variance
 *   GET    /budgets/{id}/items            POST /budgets/{id}/items
 *   PUT    /budgets/items/{itemId}        DELETE /budgets/items/{itemId}
 *   GET    /budgets/items/{itemId}/actuals POST /budgets/items/{itemId}/actuals
 *   GET    /budgets/categories?organization_id  POST /budgets/categories
 *
 * Requests go through the shared `authenticatedFetch` primitive (Authorization +
 * X-Tenant-ID from the shared providers, configured base URL, MFA retry). Money
 * is sent and received as decimal strings (see `./types`).
 */

import { ApiError, authenticatedFetch } from '../lib/fetch';
import type {
  Budget,
  BudgetActual,
  BudgetCategory,
  BudgetCategoryVariance,
  BudgetItem,
  BudgetStatus,
  BudgetSummary,
  CreateBudgetCategoryInput,
  CreateBudgetInput,
  CreateBudgetItemInput,
  ListBudgetsParams,
  RawBudget,
  RawBudgetActual,
  RawBudgetCategory,
  RawBudgetCategoryVariance,
  RawBudgetDecimal,
  RawBudgetItem,
  RawBudgetSummary,
  RecordBudgetActualInput,
  UpdateBudgetInput,
  UpdateBudgetItemInput,
} from './types';

const BASE = '/api/v1/budgets';

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const headers: Record<string, string> = {};
  if (init?.body !== undefined) headers['Content-Type'] = 'application/json';
  const response = await authenticatedFetch(path, {
    ...init,
    headers: { ...headers, ...(init?.headers as Record<string, string> | undefined) },
  });
  if (!response.ok) {
    const err = (await response.json().catch(() => ({}))) as {
      code?: string;
      error?: string;
      message?: string;
    };
    throw new ApiError(
      response.status,
      err.message || err.code || err.error || `HTTP ${response.status}`,
      err.code ?? err.error
    );
  }
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

function query(params: Record<string, string | number | undefined | null>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') search.set(key, String(value));
  }
  const qs = search.toString();
  return qs ? `?${qs}` : '';
}

const orgQuery = (organizationId: string) => query({ organization_id: organizationId });

// ---------------------------------------------------------------------------
// Mappers (wire snake_case → camelCase, decimal strings → numbers)
// ---------------------------------------------------------------------------

/** Parse a wire decimal (string, number or null) to a number; NaN/null → 0. */
export function parseBudgetAmount(value: RawBudgetDecimal | null | undefined): number {
  if (value === null || value === undefined) return 0;
  const n = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(n) ? n : 0;
}

/** Serialise a money amount for the api-server (decimal string, 2 dp). */
export function toBudgetWireAmount(value: number): string {
  return value.toFixed(2);
}

export function mapBudget(raw: RawBudget): Budget {
  return {
    id: raw.id,
    organizationId: raw.organization_id,
    buildingId: raw.building_id ?? null,
    fiscalYear: raw.fiscal_year,
    name: raw.name,
    status: raw.status as BudgetStatus,
    totalAmount: parseBudgetAmount(raw.total_amount),
    notes: raw.notes ?? null,
    approvedBy: raw.approved_by ?? null,
    approvedAt: raw.approved_at ?? null,
    createdBy: raw.created_by,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

export function mapBudgetItem(raw: RawBudgetItem): BudgetItem {
  return {
    id: raw.id,
    budgetId: raw.budget_id,
    categoryId: raw.category_id,
    name: raw.name,
    description: raw.description ?? null,
    budgetedAmount: parseBudgetAmount(raw.budgeted_amount),
    actualAmount: parseBudgetAmount(raw.actual_amount),
    varianceAmount: parseBudgetAmount(raw.variance_amount),
    variancePercent: parseBudgetAmount(raw.variance_percent),
    notes: raw.notes ?? null,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

export function mapBudgetCategory(raw: RawBudgetCategory): BudgetCategory {
  return {
    id: raw.id,
    organizationId: raw.organization_id,
    name: raw.name,
    description: raw.description ?? null,
    parentId: raw.parent_id ?? null,
    sortOrder: raw.sort_order ?? 0,
    isSystem: raw.is_system ?? false,
    createdAt: raw.created_at,
  };
}

export function mapBudgetActual(raw: RawBudgetActual): BudgetActual {
  return {
    id: raw.id,
    budgetItemId: raw.budget_item_id,
    transactionId: raw.transaction_id ?? null,
    amount: parseBudgetAmount(raw.amount),
    description: raw.description ?? null,
    transactionDate: raw.transaction_date,
    recordedBy: raw.recorded_by,
    createdAt: raw.created_at,
  };
}

export function mapBudgetSummary(raw: RawBudgetSummary): BudgetSummary {
  return {
    totalBudgeted: parseBudgetAmount(raw.total_budgeted),
    totalActual: parseBudgetAmount(raw.total_actual),
    totalVariance: parseBudgetAmount(raw.total_variance),
    variancePercent: parseBudgetAmount(raw.variance_percent),
    itemsOverBudget: raw.items_over_budget ?? 0,
    itemsUnderBudget: raw.items_under_budget ?? 0,
  };
}

export function mapBudgetCategoryVariance(raw: RawBudgetCategoryVariance): BudgetCategoryVariance {
  return {
    categoryId: raw.category_id,
    categoryName: raw.category_name,
    budgetedAmount: parseBudgetAmount(raw.budgeted_amount),
    actualAmount: parseBudgetAmount(raw.actual_amount),
    varianceAmount: parseBudgetAmount(raw.variance_amount),
    variancePercent: parseBudgetAmount(raw.variance_percent),
  };
}

// ---------------------------------------------------------------------------
// Budgets
// ---------------------------------------------------------------------------

export async function listBudgets(params: ListBudgetsParams): Promise<Budget[]> {
  const raw = await request<RawBudget[]>(
    `${BASE}${query({
      organization_id: params.organizationId,
      building_id: params.buildingId,
      fiscal_year: params.fiscalYear,
      status: params.status,
      limit: params.limit,
      offset: params.offset,
    })}`
  );
  return (raw ?? []).map(mapBudget);
}

export async function getBudget(organizationId: string, id: string): Promise<Budget> {
  return mapBudget(await request<RawBudget>(`${BASE}/${id}${orgQuery(organizationId)}`));
}

export async function createBudget(input: CreateBudgetInput): Promise<Budget> {
  const raw = await request<RawBudget>(BASE, {
    method: 'POST',
    body: JSON.stringify({
      organization_id: input.organizationId,
      building_id: input.buildingId ?? null,
      fiscal_year: input.fiscalYear,
      name: input.name,
      notes: input.notes ?? null,
    }),
  });
  return mapBudget(raw);
}

export async function updateBudget(id: string, input: UpdateBudgetInput): Promise<Budget> {
  const body: Record<string, unknown> = { organization_id: input.organizationId };
  if (input.name !== undefined) body.name = input.name;
  if (input.notes !== undefined) body.notes = input.notes;
  return mapBudget(
    await request<RawBudget>(`${BASE}/${id}`, { method: 'PUT', body: JSON.stringify(body) })
  );
}

/** Delete a budget. The api-server only allows deleting `draft` budgets (400 otherwise). */
export async function deleteBudget(organizationId: string, id: string): Promise<void> {
  await request<void>(`${BASE}/${id}${orgQuery(organizationId)}`, { method: 'DELETE' });
}

/** Status transition actions exposed by the api-server. */
export type BudgetTransition = 'submit' | 'approve' | 'activate' | 'close';

async function transition(
  action: BudgetTransition,
  organizationId: string,
  id: string
): Promise<Budget> {
  return mapBudget(
    await request<RawBudget>(`${BASE}/${id}/${action}${orgQuery(organizationId)}`, {
      method: 'POST',
    })
  );
}

/** draft → pending_approval */
export const submitBudget = (organizationId: string, id: string) =>
  transition('submit', organizationId, id);
/** pending_approval → approved */
export const approveBudget = (organizationId: string, id: string) =>
  transition('approve', organizationId, id);
/** approved → active */
export const activateBudget = (organizationId: string, id: string) =>
  transition('activate', organizationId, id);
/** active → closed */
export const closeBudget = (organizationId: string, id: string) =>
  transition('close', organizationId, id);

export async function getBudgetSummary(id: string): Promise<BudgetSummary> {
  return mapBudgetSummary(await request<RawBudgetSummary>(`${BASE}/${id}/summary`));
}

export async function getBudgetVariance(id: string): Promise<BudgetCategoryVariance[]> {
  const raw = await request<RawBudgetCategoryVariance[]>(`${BASE}/${id}/variance`);
  return (raw ?? []).map(mapBudgetCategoryVariance);
}

// ---------------------------------------------------------------------------
// Budget items
// ---------------------------------------------------------------------------

export async function listBudgetItems(budgetId: string): Promise<BudgetItem[]> {
  const raw = await request<RawBudgetItem[]>(`${BASE}/${budgetId}/items`);
  return (raw ?? []).map(mapBudgetItem);
}

export async function addBudgetItem(
  budgetId: string,
  input: CreateBudgetItemInput
): Promise<BudgetItem> {
  const raw = await request<RawBudgetItem>(`${BASE}/${budgetId}/items`, {
    method: 'POST',
    body: JSON.stringify({
      category_id: input.categoryId,
      name: input.name,
      description: input.description ?? null,
      budgeted_amount: toBudgetWireAmount(input.budgetedAmount),
      notes: input.notes ?? null,
    }),
  });
  return mapBudgetItem(raw);
}

export async function updateBudgetItem(
  itemId: string,
  input: UpdateBudgetItemInput
): Promise<BudgetItem> {
  const body: Record<string, unknown> = {};
  if (input.name !== undefined) body.name = input.name;
  if (input.description !== undefined) body.description = input.description;
  if (input.budgetedAmount !== undefined)
    body.budgeted_amount = toBudgetWireAmount(input.budgetedAmount);
  if (input.notes !== undefined) body.notes = input.notes;
  return mapBudgetItem(
    await request<RawBudgetItem>(`${BASE}/items/${itemId}`, {
      method: 'PUT',
      body: JSON.stringify(body),
    })
  );
}

export async function deleteBudgetItem(itemId: string): Promise<void> {
  await request<void>(`${BASE}/items/${itemId}`, { method: 'DELETE' });
}

export async function listBudgetActuals(itemId: string): Promise<BudgetActual[]> {
  const raw = await request<RawBudgetActual[]>(`${BASE}/items/${itemId}/actuals`);
  return (raw ?? []).map(mapBudgetActual);
}

export async function recordBudgetActual(
  itemId: string,
  input: RecordBudgetActualInput
): Promise<BudgetActual> {
  const raw = await request<RawBudgetActual>(`${BASE}/items/${itemId}/actuals`, {
    method: 'POST',
    body: JSON.stringify({
      amount: toBudgetWireAmount(input.amount),
      transaction_date: input.transactionDate,
      description: input.description ?? null,
      transaction_id: input.transactionId ?? null,
    }),
  });
  return mapBudgetActual(raw);
}

// ---------------------------------------------------------------------------
// Categories
// ---------------------------------------------------------------------------

export async function listBudgetCategories(organizationId: string): Promise<BudgetCategory[]> {
  const raw = await request<RawBudgetCategory[]>(`${BASE}/categories${orgQuery(organizationId)}`);
  return (raw ?? []).map(mapBudgetCategory);
}

export async function createBudgetCategory(
  input: CreateBudgetCategoryInput
): Promise<BudgetCategory> {
  const body: Record<string, unknown> = {
    organization_id: input.organizationId,
    name: input.name,
    description: input.description ?? null,
    parent_id: input.parentId ?? null,
  };
  if (input.sortOrder !== undefined) body.sort_order = input.sortOrder;
  return mapBudgetCategory(
    await request<RawBudgetCategory>(`${BASE}/categories`, {
      method: 'POST',
      body: JSON.stringify(body),
    })
  );
}
