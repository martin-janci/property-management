-- Migration: 00183_accounting_provider_snapshot_tables
-- PAP-191: External accounting provider Fáza 1.3 — Migrations: connection + snapshot/cursor tables (FORCE RLS)

-- ============================================================================
-- accounting_provider_connection
-- ============================================================================
CREATE TABLE IF NOT EXISTS accounting_provider_connection (
    tenant_id UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
    auth_flow TEXT NOT NULL CHECK (auth_flow IN ('ccf', 'acf')),
    provider_account_name TEXT NOT NULL,
    client_id TEXT NOT NULL,
    client_secret_enc TEXT, -- CCF, IntegrationCrypto AES-256-GCM
    refresh_token_enc TEXT, -- ACF only, IntegrationCrypto AES-256-GCM
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Trigger for updated_at
CREATE TRIGGER update_accounting_provider_connection_updated_at
    BEFORE UPDATE ON accounting_provider_connection
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- RLS
ALTER TABLE accounting_provider_connection ENABLE ROW LEVEL SECURITY;
ALTER TABLE accounting_provider_connection FORCE ROW LEVEL SECURITY;

CREATE POLICY accounting_provider_connection_tenant_isolation ON accounting_provider_connection
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- accounting_provider_contact
-- ============================================================================
CREATE TABLE IF NOT EXISTS accounting_provider_contact (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    external_id BIGINT NOT NULL,
    company_name TEXT,
    ico TEXT,
    dic TEXT,
    email TEXT,
    iban TEXT,
    date_last_change TIMESTAMPTZ NOT NULL,
    raw JSONB NOT NULL,
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, external_id)
);

CREATE INDEX IF NOT EXISTS idx_accounting_provider_contact_tenant_last_change ON accounting_provider_contact(tenant_id, date_last_change);

-- RLS
ALTER TABLE accounting_provider_contact ENABLE ROW LEVEL SECURITY;
ALTER TABLE accounting_provider_contact FORCE ROW LEVEL SECURITY;

CREATE POLICY accounting_provider_contact_tenant_isolation ON accounting_provider_contact
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- accounting_provider_issued_invoice
-- ============================================================================
CREATE TABLE IF NOT EXISTS accounting_provider_issued_invoice (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    external_id BIGINT NOT NULL,
    document_number TEXT NOT NULL,
    partner_external_id BIGINT NOT NULL,
    variable_symbol TEXT,
    iban TEXT,
    account_number TEXT,
    bank_code TEXT,
    currency TEXT,
    total_with_vat NUMERIC(18,2) NOT NULL,
    total_without_vat NUMERIC(18,2) NOT NULL,
    payment_status SMALLINT NOT NULL,
    date_of_issue DATE NOT NULL,
    date_of_maturity DATE NOT NULL,
    date_of_payment DATE,
    date_last_change TIMESTAMPTZ NOT NULL,
    raw JSONB NOT NULL,
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, external_id)
);

CREATE INDEX IF NOT EXISTS idx_accounting_provider_invoice_tenant_last_change ON accounting_provider_issued_invoice(tenant_id, date_last_change);
CREATE INDEX IF NOT EXISTS idx_accounting_provider_invoice_tenant_vs ON accounting_provider_issued_invoice(tenant_id, variable_symbol);
CREATE INDEX IF NOT EXISTS idx_accounting_provider_invoice_tenant_status ON accounting_provider_issued_invoice(tenant_id, payment_status);

-- RLS
ALTER TABLE accounting_provider_issued_invoice ENABLE ROW LEVEL SECURITY;
ALTER TABLE accounting_provider_issued_invoice FORCE ROW LEVEL SECURITY;

CREATE POLICY accounting_provider_invoice_tenant_isolation ON accounting_provider_issued_invoice
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- accounting_provider_sync_cursor
-- ============================================================================
CREATE TABLE IF NOT EXISTS accounting_provider_sync_cursor (
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    entity TEXT NOT NULL, -- 'contact' | 'issued_invoice'
    last_change_seen TIMESTAMPTZ NOT NULL,
    last_run_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_status TEXT,
    PRIMARY KEY (tenant_id, entity)
);

-- RLS
ALTER TABLE accounting_provider_sync_cursor ENABLE ROW LEVEL SECURITY;
ALTER TABLE accounting_provider_sync_cursor FORCE ROW LEVEL SECURITY;

CREATE POLICY accounting_provider_sync_cursor_tenant_isolation ON accounting_provider_sync_cursor
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- accounting_provider_payment_match_snapshot
-- ============================================================================
CREATE TABLE IF NOT EXISTS accounting_provider_payment_match_snapshot (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    invoice_id UUID NOT NULL REFERENCES accounting_provider_issued_invoice(id) ON DELETE CASCADE,
    bank_movement_ref TEXT NOT NULL,
    matched_by TEXT NOT NULL, -- 'variable_symbol' | 'iban+amount' | 'manual'
    confidence NUMERIC(4,3) NOT NULL,
    amount NUMERIC(18,2) NOT NULL,
    matched_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_accounting_provider_match_tenant_invoice ON accounting_provider_payment_match_snapshot(tenant_id, invoice_id);

-- RLS
ALTER TABLE accounting_provider_payment_match_snapshot ENABLE ROW LEVEL SECURITY;
ALTER TABLE accounting_provider_payment_match_snapshot FORCE ROW LEVEL SECURITY;

CREATE POLICY accounting_provider_match_tenant_isolation ON accounting_provider_payment_match_snapshot
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());
