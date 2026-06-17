-- Migration: 00184_accounting_mvp_native_tables
-- PAP-206: N1 — Accounting MVP: native data model + migrations + FORCE RLS (6 tables)
-- Native data model for the standalone CRM/economic MVP (independent of any external accounting provider).
-- Pattern: follow 00183 / force_*_rls pattern.

-- ============================================================================
-- contact
-- ============================================================================
CREATE TABLE IF NOT EXISTS contact (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    ico TEXT,
    dic TEXT,
    email TEXT,
    phone TEXT,
    address TEXT,
    iban TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_contact_tenant_id ON contact(tenant_id);

-- Trigger for updated_at
CREATE TRIGGER update_contact_updated_at
    BEFORE UPDATE ON contact
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- RLS
ALTER TABLE contact ENABLE ROW LEVEL SECURITY;
ALTER TABLE contact FORCE ROW LEVEL SECURITY;

CREATE POLICY contact_tenant_isolation ON contact
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- invoice
-- ============================================================================
CREATE TABLE IF NOT EXISTS invoice (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id UUID NOT NULL REFERENCES contact(id) ON DELETE RESTRICT,
    number TEXT NOT NULL,
    issue_date DATE NOT NULL,
    due_date DATE NOT NULL,
    taxable_supply_date DATE,
    currency TEXT NOT NULL DEFAULT 'CZK',
    total_amount NUMERIC(18,2) NOT NULL DEFAULT 0,
    base_amount NUMERIC(18,2) NOT NULL DEFAULT 0,
    vat_amount NUMERIC(18,2) NOT NULL DEFAULT 0,
    variable_symbol TEXT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'issued', 'paid', 'partially_paid', 'overdue')),
    paid_amount NUMERIC(18,2) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, number)
);

CREATE INDEX IF NOT EXISTS idx_invoice_tenant_id ON invoice(tenant_id);
CREATE INDEX IF NOT EXISTS idx_invoice_contact_id ON invoice(contact_id);
CREATE INDEX IF NOT EXISTS idx_invoice_tenant_vs ON invoice(tenant_id, variable_symbol);

-- Trigger for updated_at
CREATE TRIGGER update_invoice_updated_at
    BEFORE UPDATE ON invoice
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- RLS
ALTER TABLE invoice ENABLE ROW LEVEL SECURITY;
ALTER TABLE invoice FORCE ROW LEVEL SECURITY;

CREATE POLICY invoice_tenant_isolation ON invoice
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- invoice_item
-- ============================================================================
CREATE TABLE IF NOT EXISTS invoice_item (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_id UUID NOT NULL REFERENCES invoice(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    qty NUMERIC(18,4) NOT NULL DEFAULT 1,
    unit_price NUMERIC(18,2) NOT NULL DEFAULT 0,
    vat_rate NUMERIC(5,2) NOT NULL DEFAULT 0,
    total_amount NUMERIC(18,2) NOT NULL DEFAULT 0,
    base_amount NUMERIC(18,2) NOT NULL DEFAULT 0,
    vat_amount NUMERIC(18,2) NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_invoice_item_invoice_id ON invoice_item(invoice_id);
CREATE INDEX IF NOT EXISTS idx_invoice_item_tenant_id ON invoice_item(tenant_id);

-- RLS
ALTER TABLE invoice_item ENABLE ROW LEVEL SECURITY;
ALTER TABLE invoice_item FORCE ROW LEVEL SECURITY;

CREATE POLICY invoice_item_tenant_isolation ON invoice_item
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- bank_statement
-- ============================================================================
CREATE TABLE IF NOT EXISTS bank_statement (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    source_filename TEXT NOT NULL,
    imported_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    period TEXT,
    account_iban TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bank_statement_tenant_id ON bank_statement(tenant_id);

-- RLS
ALTER TABLE bank_statement ENABLE ROW LEVEL SECURITY;
ALTER TABLE bank_statement FORCE ROW LEVEL SECURITY;

CREATE POLICY bank_statement_tenant_isolation ON bank_statement
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- bank_statement_line
-- ============================================================================
CREATE TABLE IF NOT EXISTS bank_statement_line (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    statement_id UUID NOT NULL REFERENCES bank_statement(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    booking_date DATE NOT NULL,
    amount NUMERIC(18,2) NOT NULL,
    currency TEXT NOT NULL,
    counterparty_iban TEXT,
    variable_symbol TEXT,
    raw_ref TEXT,
    match_state TEXT NOT NULL DEFAULT 'unmatched'
);

CREATE INDEX IF NOT EXISTS idx_bank_statement_line_statement_id ON bank_statement_line(statement_id);
CREATE INDEX IF NOT EXISTS idx_bank_statement_line_tenant_id ON bank_statement_line(tenant_id);
CREATE INDEX IF NOT EXISTS idx_bank_statement_line_tenant_vs ON bank_statement_line(tenant_id, variable_symbol);

-- RLS
ALTER TABLE bank_statement_line ENABLE ROW LEVEL SECURITY;
ALTER TABLE bank_statement_line FORCE ROW LEVEL SECURITY;

CREATE POLICY bank_statement_line_tenant_isolation ON bank_statement_line
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- payment_match
-- ============================================================================
CREATE TABLE IF NOT EXISTS payment_match (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    statement_line_id UUID NOT NULL REFERENCES bank_statement_line(id) ON DELETE CASCADE,
    invoice_id UUID NOT NULL REFERENCES invoice(id) ON DELETE CASCADE,
    confidence NUMERIC(4,3) NOT NULL,
    decided_by UUID REFERENCES users(id) ON DELETE SET NULL,
    decided_at TIMESTAMPTZ,
    state TEXT NOT NULL CHECK (state IN ('suggested', 'confirmed', 'rejected')),
    UNIQUE (statement_line_id, invoice_id)
);

CREATE INDEX IF NOT EXISTS idx_payment_match_tenant_id ON payment_match(tenant_id);
CREATE INDEX IF NOT EXISTS idx_payment_match_line_id ON payment_match(statement_line_id);
CREATE INDEX IF NOT EXISTS idx_payment_match_invoice_id ON payment_match(invoice_id);

-- RLS
ALTER TABLE payment_match ENABLE ROW LEVEL SECURITY;
ALTER TABLE payment_match FORCE ROW LEVEL SECURITY;

CREATE POLICY payment_match_tenant_isolation ON payment_match
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());
