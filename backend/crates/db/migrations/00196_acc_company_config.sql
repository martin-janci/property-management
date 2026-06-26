-- Migration: 00196_acc_company_config
-- ACC MVP (EPIC-ACC-02 Company & Document Configuration): per-company settings
-- that drive every document — profile/branding, tax mode, numbering series,
-- units, VAT rates, bank accounts, email/document templates.
--
-- Tenant model: ACC "company/agenda" == existing organizations tenant. Every
-- table follows the 00184 pattern EXACTLY:
--   tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE
--   ENABLE + FORCE ROW LEVEL SECURITY
--   policy FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted())

-- ============================================================================
-- acc_company_settings — per-org profile/branding/tax-mode (UC-ACC-02.1/.2/.11)
-- One row per tenant.
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_company_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    legal_name TEXT NOT NULL DEFAULT '',
    ico TEXT,
    dic TEXT,
    vat_id TEXT,
    address TEXT,
    email TEXT,
    phone TEXT,
    web TEXT,
    logo_url TEXT,
    brand_color TEXT,
    -- Tax mode: 'tax_records' (daňová evidencia) | 'accounting' (podvojné účtovníctvo)
    tax_mode TEXT NOT NULL DEFAULT 'tax_records' CHECK (tax_mode IN ('tax_records', 'accounting')),
    vat_payer BOOLEAN NOT NULL DEFAULT FALSE,
    base_currency TEXT NOT NULL DEFAULT 'EUR',
    -- Rounding mode: 'arithmetic' | 'bankers' | 'none'
    rounding_mode TEXT NOT NULL DEFAULT 'arithmetic' CHECK (rounding_mode IN ('arithmetic', 'bankers', 'none')),
    default_payment_terms_days INT NOT NULL DEFAULT 14,
    default_invoice_note TEXT,
    default_footer TEXT,
    default_language TEXT NOT NULL DEFAULT 'sk',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id)
);

CREATE INDEX IF NOT EXISTS idx_acc_company_settings_tenant_id ON acc_company_settings(tenant_id);

CREATE TRIGGER update_acc_company_settings_updated_at
    BEFORE UPDATE ON acc_company_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE acc_company_settings ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_company_settings FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_company_settings_tenant_isolation ON acc_company_settings
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_numbering_series — gapless numbering per doc type/year (UC-ACC-02.3)
-- Atomic allocation: bump next_value in a single UPDATE ... RETURNING under RLS.
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_numbering_series (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    -- 'invoice' | 'proforma' | 'advance' | 'credit_note'
    doc_type TEXT NOT NULL CHECK (doc_type IN ('invoice', 'proforma', 'advance', 'credit_note')),
    year INT NOT NULL,
    prefix TEXT NOT NULL DEFAULT '',
    -- format template e.g. '{prefix}{year}{seq:04}' resolved by accounting-core::numbering
    format TEXT NOT NULL DEFAULT '{prefix}{year}{seq:04}',
    next_value BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, doc_type, year)
);

CREATE INDEX IF NOT EXISTS idx_acc_numbering_series_tenant_id ON acc_numbering_series(tenant_id);

CREATE TRIGGER update_acc_numbering_series_updated_at
    BEFORE UPDATE ON acc_numbering_series
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE acc_numbering_series ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_numbering_series FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_numbering_series_tenant_isolation ON acc_numbering_series
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_unit — units of measure (UC-ACC-02.8)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_unit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, code)
);

CREATE INDEX IF NOT EXISTS idx_acc_unit_tenant_id ON acc_unit(tenant_id);

CREATE TRIGGER update_acc_unit_updated_at
    BEFORE UPDATE ON acc_unit
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE acc_unit ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_unit FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_unit_tenant_isolation ON acc_unit
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_vat_rate — configurable VAT rates (UC-ACC-02.9)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_vat_rate (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    rate NUMERIC(5,2) NOT NULL DEFAULT 0,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    -- date-effective for legislative changes (EPIC-ACC-16.8)
    effective_from DATE,
    effective_to DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_acc_vat_rate_tenant_id ON acc_vat_rate(tenant_id);

CREATE TRIGGER update_acc_vat_rate_updated_at
    BEFORE UPDATE ON acc_vat_rate
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE acc_vat_rate ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_vat_rate FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_vat_rate_tenant_isolation ON acc_vat_rate
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_bank_account — bank accounts shown on documents (UC-ACC-02.10)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_bank_account (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    iban TEXT NOT NULL,
    swift TEXT,
    bank_name TEXT,
    currency TEXT NOT NULL DEFAULT 'EUR',
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_acc_bank_account_tenant_id ON acc_bank_account(tenant_id);

CREATE TRIGGER update_acc_bank_account_updated_at
    BEFORE UPDATE ON acc_bank_account
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE acc_bank_account ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_bank_account FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_bank_account_tenant_isolation ON acc_bank_account
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_email_template — templated emails for sending docs (UC-ACC-02.6)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_email_template (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    -- 'invoice' | 'proforma' | 'reminder' | 'thank_you'
    kind TEXT NOT NULL DEFAULT 'invoice',
    language TEXT NOT NULL DEFAULT 'sk',
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_acc_email_template_tenant_id ON acc_email_template(tenant_id);

CREATE TRIGGER update_acc_email_template_updated_at
    BEFORE UPDATE ON acc_email_template
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE acc_email_template ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_email_template FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_email_template_tenant_isolation ON acc_email_template
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_document_template — document design/branding template (UC-ACC-02.4)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_document_template (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    -- 'classic' | 'modern' | 'minimal' theme key resolved by the renderer
    theme TEXT NOT NULL DEFAULT 'classic',
    accent_color TEXT,
    show_logo BOOLEAN NOT NULL DEFAULT TRUE,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_acc_document_template_tenant_id ON acc_document_template(tenant_id);

CREATE TRIGGER update_acc_document_template_updated_at
    BEFORE UPDATE ON acc_document_template
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE acc_document_template ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_document_template FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_document_template_tenant_isolation ON acc_document_template
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());
