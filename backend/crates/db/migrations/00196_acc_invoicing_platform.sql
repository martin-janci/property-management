-- Migration: 00196_acc_invoicing_platform
-- ACC MVP (EPIC-ACC-05 invoicing extensions + EPIC-ACC-16 platform/security).
-- Extends the existing 00184 `invoice` table with doc-type / correction /
-- advance-settlement / FX columns, plus the cross-cutting platform tables
-- (document links, attachments, tags, share links, audit log, 2FA).
--
-- Invoice columns verified against 00184: id, tenant_id, contact_id, number,
-- issue_date, due_date, taxable_supply_date, currency, total_amount,
-- base_amount, vat_amount, variable_symbol, status, paid_amount, created_at,
-- updated_at. New columns use ALTER ... ADD COLUMN IF NOT EXISTS.

-- ============================================================================
-- invoice extensions (UC-ACC-05.4/.5/.6/.7/.15)
-- ============================================================================
-- doc_type: invoice | proforma | advance | credit_note (UC-ACC-05.4/.5/.7)
ALTER TABLE invoice ADD COLUMN IF NOT EXISTS doc_type TEXT NOT NULL DEFAULT 'invoice'
    CHECK (doc_type IN ('invoice', 'proforma', 'advance', 'credit_note'));
-- corrected_invoice_id: for credit notes, references the original (UC-ACC-05.7)
ALTER TABLE invoice ADD COLUMN IF NOT EXISTS corrected_invoice_id UUID REFERENCES invoice(id) ON DELETE SET NULL;
-- advance_settlement: net amount of a settled advance deducted from this final
-- invoice's amount due (UC-ACC-05.6).
ALTER TABLE invoice ADD COLUMN IF NOT EXISTS advance_settlement NUMERIC(18,2) NOT NULL DEFAULT 0;
-- settled_advance_id: the advance invoice this final invoice settles (UC-ACC-05.6)
ALTER TABLE invoice ADD COLUMN IF NOT EXISTS settled_advance_id UUID REFERENCES invoice(id) ON DELETE SET NULL;
-- exchange_rate + base_currency_amount: foreign-currency support (UC-ACC-05.4)
ALTER TABLE invoice ADD COLUMN IF NOT EXISTS exchange_rate NUMERIC(18,6);
ALTER TABLE invoice ADD COLUMN IF NOT EXISTS base_currency_amount NUMERIC(18,2);
-- numbering series this invoice's number was allocated from (EPIC-ACC-02.3)
ALTER TABLE invoice ADD COLUMN IF NOT EXISTS numbering_series_id UUID REFERENCES acc_numbering_series(id) ON DELETE SET NULL;
-- bank account shown on the document / drives QR (UC-ACC-02.10 / 05.8)
ALTER TABLE invoice ADD COLUMN IF NOT EXISTS bank_account_id UUID REFERENCES acc_bank_account(id) ON DELETE SET NULL;
-- 'sent' status is added to the lifecycle for EPIC-ACC-05.7/.10; the original
-- 00184 CHECK only allows draft/issued/paid/partially_paid/overdue. Relax it
-- so the full lifecycle (incl. 'sent' and 'cancelled') is storable.
ALTER TABLE invoice DROP CONSTRAINT IF EXISTS invoice_status_check;
ALTER TABLE invoice ADD CONSTRAINT invoice_status_check
    CHECK (status IN ('draft', 'issued', 'sent', 'paid', 'partially_paid', 'overdue', 'cancelled'));

CREATE INDEX IF NOT EXISTS idx_invoice_doc_type ON invoice(tenant_id, doc_type);
CREATE INDEX IF NOT EXISTS idx_invoice_corrected_id ON invoice(corrected_invoice_id);

-- ============================================================================
-- acc_document_link — typed relationship edges between documents (UC-ACC-05.15)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_document_link (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    from_invoice_id UUID NOT NULL REFERENCES invoice(id) ON DELETE CASCADE,
    to_invoice_id UUID NOT NULL REFERENCES invoice(id) ON DELETE CASCADE,
    -- 'settles' | 'corrects' | 'converts' | 'related'
    relation TEXT NOT NULL DEFAULT 'related',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, from_invoice_id, to_invoice_id, relation)
);

CREATE INDEX IF NOT EXISTS idx_acc_document_link_tenant_id ON acc_document_link(tenant_id);
CREATE INDEX IF NOT EXISTS idx_acc_document_link_from ON acc_document_link(from_invoice_id);
CREATE INDEX IF NOT EXISTS idx_acc_document_link_to ON acc_document_link(to_invoice_id);

ALTER TABLE acc_document_link ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_document_link FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_document_link_tenant_isolation ON acc_document_link
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_attachment — files attached to documents (UC-ACC-05.13)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_attachment (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    invoice_id UUID REFERENCES invoice(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL DEFAULT 0,
    storage_key TEXT NOT NULL,
    uploaded_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_acc_attachment_tenant_id ON acc_attachment(tenant_id);
CREATE INDEX IF NOT EXISTS idx_acc_attachment_invoice_id ON acc_attachment(invoice_id);

ALTER TABLE acc_attachment ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_attachment FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_attachment_tenant_isolation ON acc_attachment
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_tag — document tags (UC-ACC-05.14). Generic over an entity ref.
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_tag (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    -- 'invoice' for the MVP; future: 'expense', 'contact', ...
    entity_type TEXT NOT NULL DEFAULT 'invoice',
    entity_id UUID NOT NULL,
    tag TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, entity_type, entity_id, tag)
);

CREATE INDEX IF NOT EXISTS idx_acc_tag_tenant_id ON acc_tag(tenant_id);
CREATE INDEX IF NOT EXISTS idx_acc_tag_entity ON acc_tag(tenant_id, entity_type, entity_id);

ALTER TABLE acc_tag ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_tag FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_tag_tenant_isolation ON acc_tag
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_share_link — capability-token URLs for read-only invoice view (UC-ACC-05.11)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_share_link (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    invoice_id UUID NOT NULL REFERENCES invoice(id) ON DELETE CASCADE,
    -- hard-to-guess opaque token; hashed at rest is preferable, stored raw here
    -- for the MVP skeleton (coder may switch to a hash column via contract-note).
    token TEXT NOT NULL,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, token)
);

CREATE INDEX IF NOT EXISTS idx_acc_share_link_tenant_id ON acc_share_link(tenant_id);
CREATE INDEX IF NOT EXISTS idx_acc_share_link_invoice_id ON acc_share_link(invoice_id);
CREATE INDEX IF NOT EXISTS idx_acc_share_link_token ON acc_share_link(token);

ALTER TABLE acc_share_link ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_share_link FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_share_link_tenant_isolation ON acc_share_link
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_audit_log — append-only audit trail (UC-ACC-16.7 / 01.10)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    actor_id UUID REFERENCES users(id) ON DELETE SET NULL,
    -- 'create' | 'update' | 'delete' | 'issue' | 'send' | ...
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID,
    -- old -> new diff for the change (who/what/when/old->new)
    diff JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_acc_audit_log_tenant_id ON acc_audit_log(tenant_id);
CREATE INDEX IF NOT EXISTS idx_acc_audit_log_entity ON acc_audit_log(tenant_id, entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_acc_audit_log_created_at ON acc_audit_log(tenant_id, created_at DESC);

ALTER TABLE acc_audit_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_audit_log FORCE ROW LEVEL SECURITY;

-- Append-only: allow SELECT + INSERT within tenant scope; no UPDATE/DELETE
-- policy is created, so FORCE RLS blocks mutations (tamper-evidence, UC-ACC-16.7).
CREATE POLICY acc_audit_log_tenant_select ON acc_audit_log
    FOR SELECT USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

CREATE POLICY acc_audit_log_tenant_insert ON acc_audit_log
    FOR INSERT WITH CHECK (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_two_factor — per-user TOTP 2FA enrollment for ACC (UC-ACC-16.1 / 01.9)
-- Scoped per tenant+user (a user may operate multiple companies).
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_two_factor (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- TOTP secret (encrypted at rest preferred; coder may switch via contract-note)
    secret TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    confirmed_at TIMESTAMPTZ,
    -- one-time recovery codes (hashed), JSON array
    recovery_codes JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_acc_two_factor_tenant_id ON acc_two_factor(tenant_id);
CREATE INDEX IF NOT EXISTS idx_acc_two_factor_user_id ON acc_two_factor(user_id);

CREATE TRIGGER update_acc_two_factor_updated_at
    BEFORE UPDATE ON acc_two_factor
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE acc_two_factor ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_two_factor FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_two_factor_tenant_isolation ON acc_two_factor
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());
