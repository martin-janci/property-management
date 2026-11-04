/**
 * Budgets API types (Epic 24 — Budget & Financial Planning).
 *
 * Mirrors `backend/servers/api-server/src/routes/budgets.rs` and the models in
 * `backend/crates/db/src/models/budget.rs`.
 *
 * Wire shapes (`Raw*`) are snake_case and carry money as decimal STRINGS — the
 * api-server serialises `rust_decimal` with the `serde-str` feature, and with
 * that feature it also only DEserialises decimals from strings, so request
 * bodies must send amounts as strings too. The public shapes are camelCase with
 * money as `number`; the mappers in `./api` convert at the boundary.
 */

// ---------------------------------------------------------------------------
// Public (camelCase) types
// ---------------------------------------------------------------------------

/** Budget lifecycle: draft → pending_approval → approved → active → closed. */
export type BudgetStatus = 'draft' | 'pending_approval' | 'approved' | 'active' | 'closed';

export const BUDGET_STATUSES: readonly BudgetStatus[] = [
  'draft',
  'pending_approval',
  'approved',
  'active',
  'closed',
] as const;

export interface Budget {
  id: string;
  organizationId: string;
  buildingId: string | null;
  fiscalYear: number;
  name: string;
  status: BudgetStatus;
  /** Sum of item budgeted amounts (maintained by a DB trigger). */
  totalAmount: number;
  notes: string | null;
  approvedBy: string | null;
  approvedAt: string | null;
  createdBy: string;
  createdAt: string;
  updatedAt: string;
}

export interface BudgetItem {
  id: string;
  budgetId: string;
  categoryId: string;
  name: string;
  description: string | null;
  budgetedAmount: number;
  actualAmount: number;
  /** `actual - budgeted` (positive = over budget). */
  varianceAmount: number;
  variancePercent: number;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface BudgetCategory {
  id: string;
  organizationId: string;
  name: string;
  description: string | null;
  parentId: string | null;
  sortOrder: number;
  isSystem: boolean;
  createdAt: string;
}

export interface BudgetActual {
  id: string;
  budgetItemId: string;
  transactionId: string | null;
  amount: number;
  description: string | null;
  /** YYYY-MM-DD */
  transactionDate: string;
  recordedBy: string;
  createdAt: string;
}

export interface BudgetSummary {
  totalBudgeted: number;
  totalActual: number;
  /** `actual - budgeted` (positive = over budget). */
  totalVariance: number;
  variancePercent: number;
  itemsOverBudget: number;
  itemsUnderBudget: number;
}

export interface BudgetCategoryVariance {
  categoryId: string;
  categoryName: string;
  budgetedAmount: number;
  actualAmount: number;
  varianceAmount: number;
  variancePercent: number;
}

// ---------------------------------------------------------------------------
// Request inputs
// ---------------------------------------------------------------------------

export interface ListBudgetsParams {
  organizationId: string;
  buildingId?: string;
  fiscalYear?: number;
  status?: BudgetStatus;
  limit?: number;
  offset?: number;
}

export interface CreateBudgetInput {
  organizationId: string;
  buildingId?: string | null;
  fiscalYear: number;
  name: string;
  notes?: string | null;
}

export interface UpdateBudgetInput {
  organizationId: string;
  name?: string;
  notes?: string | null;
}

export interface CreateBudgetItemInput {
  categoryId: string;
  name: string;
  description?: string | null;
  budgetedAmount: number;
  notes?: string | null;
}

export interface UpdateBudgetItemInput {
  name?: string;
  description?: string | null;
  budgetedAmount?: number;
  notes?: string | null;
}

export interface RecordBudgetActualInput {
  amount: number;
  /** YYYY-MM-DD */
  transactionDate: string;
  description?: string | null;
  transactionId?: string | null;
}

export interface CreateBudgetCategoryInput {
  organizationId: string;
  name: string;
  description?: string | null;
  parentId?: string | null;
  sortOrder?: number;
}

// ---------------------------------------------------------------------------
// Wire (snake_case) types
// ---------------------------------------------------------------------------

/** Decimal as sent by the api-server (string), tolerating a number. */
export type RawBudgetDecimal = string | number;

export interface RawBudget {
  id: string;
  organization_id: string;
  building_id: string | null;
  fiscal_year: number;
  name: string;
  status: string;
  total_amount: RawBudgetDecimal;
  notes: string | null;
  approved_by: string | null;
  approved_at: string | null;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface RawBudgetItem {
  id: string;
  budget_id: string;
  category_id: string;
  name: string;
  description: string | null;
  budgeted_amount: RawBudgetDecimal;
  actual_amount: RawBudgetDecimal;
  variance_amount: RawBudgetDecimal;
  variance_percent: RawBudgetDecimal;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface RawBudgetCategory {
  id: string;
  organization_id: string;
  name: string;
  description: string | null;
  parent_id: string | null;
  sort_order: number;
  is_system: boolean;
  created_at: string;
}

export interface RawBudgetActual {
  id: string;
  budget_item_id: string;
  transaction_id: string | null;
  amount: RawBudgetDecimal;
  description: string | null;
  transaction_date: string;
  recorded_by: string;
  created_at: string;
}

export interface RawBudgetSummary {
  total_budgeted: RawBudgetDecimal;
  total_actual: RawBudgetDecimal;
  total_variance: RawBudgetDecimal;
  variance_percent: RawBudgetDecimal;
  items_over_budget: number;
  items_under_budget: number;
}

export interface RawBudgetCategoryVariance {
  category_id: string;
  category_name: string;
  budgeted_amount: RawBudgetDecimal;
  actual_amount: RawBudgetDecimal;
  variance_amount: RawBudgetDecimal;
  variance_percent: RawBudgetDecimal;
}
