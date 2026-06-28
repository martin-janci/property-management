-- Migration: 00201_acc_catalog_contacts_ext
-- ACC MVP (EPIC-ACC-04 Catalog + EPIC-ACC-03 Contacts/CRM extensions).
-- Catalog of goods/services with pricing/levels/categories/tags, plus the
-- per-contact CRM extensions (addresses, tags, per-contact defaults) layered
-- on top of the existing 00184 `contact` table.
--
-- All new tables follow the 00184 tenant + FORCE RLS pattern.

-- ============================================================================
-- acc_item_category — catalog categories (UC-ACC-04.4)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_item_category (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    parent_id UUID REFERENCES acc_item_category(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_acc_item_category_tenant_id ON acc_item_category(tenant_id);

CREATE TRIGGER update_acc_item_category_updated_at
    BEFORE UPDATE ON acc_item_category
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE acc_item_category ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_item_category FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_item_category_tenant_isolation ON acc_item_category
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_catalog_item — reusable catalog item (UC-ACC-04.1/.3/.7)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_catalog_item (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    code TEXT,
    name TEXT NOT NULL,
    description TEXT,
    -- 'goods' | 'service'
    kind TEXT NOT NULL DEFAULT 'service' CHECK (kind IN ('goods', 'service')),
    unit_id UUID REFERENCES acc_unit(id) ON DELETE SET NULL,
    vat_rate NUMERIC(5,2) NOT NULL DEFAULT 0,
    -- base list price (net); price levels live in acc_price_level
    unit_price NUMERIC(18,2) NOT NULL DEFAULT 0,
    -- whether unit_price is stored net (exclusive) or gross (inclusive)
    price_is_gross BOOLEAN NOT NULL DEFAULT FALSE,
    category_id UUID REFERENCES acc_item_category(id) ON DELETE SET NULL,
    stock_tracked BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, code)
);

CREATE INDEX IF NOT EXISTS idx_acc_catalog_item_tenant_id ON acc_catalog_item(tenant_id);
CREATE INDEX IF NOT EXISTS idx_acc_catalog_item_category_id ON acc_catalog_item(category_id);

CREATE TRIGGER update_acc_catalog_item_updated_at
    BEFORE UPDATE ON acc_catalog_item
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE acc_catalog_item ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_catalog_item FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_catalog_item_tenant_isolation ON acc_catalog_item
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_price_level — named price levels per catalog item (UC-ACC-04.2)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_price_level (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    item_id UUID NOT NULL REFERENCES acc_catalog_item(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    price NUMERIC(18,2) NOT NULL DEFAULT 0,
    price_is_gross BOOLEAN NOT NULL DEFAULT FALSE,
    currency TEXT NOT NULL DEFAULT 'EUR',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, item_id, name)
);

CREATE INDEX IF NOT EXISTS idx_acc_price_level_tenant_id ON acc_price_level(tenant_id);
CREATE INDEX IF NOT EXISTS idx_acc_price_level_item_id ON acc_price_level(item_id);

CREATE TRIGGER update_acc_price_level_updated_at
    BEFORE UPDATE ON acc_price_level
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE acc_price_level ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_price_level FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_price_level_tenant_isolation ON acc_price_level
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- Contact CRM extensions (EPIC-ACC-03). Layer onto existing 00184 `contact`.
-- Per-contact defaults columns (UC-ACC-03.9) added via ALTER ... IF NOT EXISTS.
-- ============================================================================
ALTER TABLE contact ADD COLUMN IF NOT EXISTS kind TEXT NOT NULL DEFAULT 'customer'
    CHECK (kind IN ('customer', 'supplier', 'both'));
ALTER TABLE contact ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE contact ADD COLUMN IF NOT EXISTS default_currency TEXT;
ALTER TABLE contact ADD COLUMN IF NOT EXISTS default_price_level TEXT;
ALTER TABLE contact ADD COLUMN IF NOT EXISTS default_payment_terms_days INT;
ALTER TABLE contact ADD COLUMN IF NOT EXISTS default_discount_pct NUMERIC(5,2);
ALTER TABLE contact ADD COLUMN IF NOT EXISTS default_language TEXT;
ALTER TABLE contact ADD COLUMN IF NOT EXISTS vat_verified_at TIMESTAMPTZ;
ALTER TABLE contact ADD COLUMN IF NOT EXISTS vat_verified_valid BOOLEAN;
ALTER TABLE contact ADD COLUMN IF NOT EXISTS credit_limit NUMERIC(18,2);
ALTER TABLE contact ADD COLUMN IF NOT EXISTS notes TEXT;
-- merge target: when a contact is merged into another, point here (UC-ACC-03.5)
ALTER TABLE contact ADD COLUMN IF NOT EXISTS merged_into_id UUID REFERENCES contact(id) ON DELETE SET NULL;

-- ============================================================================
-- acc_contact_address — multiple billing/delivery addresses (UC-ACC-03.4)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_contact_address (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id UUID NOT NULL REFERENCES contact(id) ON DELETE CASCADE,
    -- 'billing' | 'delivery'
    kind TEXT NOT NULL DEFAULT 'billing' CHECK (kind IN ('billing', 'delivery')),
    label TEXT,
    street TEXT,
    city TEXT,
    postal_code TEXT,
    country TEXT,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_acc_contact_address_tenant_id ON acc_contact_address(tenant_id);
CREATE INDEX IF NOT EXISTS idx_acc_contact_address_contact_id ON acc_contact_address(contact_id);

CREATE TRIGGER update_acc_contact_address_updated_at
    BEFORE UPDATE ON acc_contact_address
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE acc_contact_address ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_contact_address FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_contact_address_tenant_isolation ON acc_contact_address
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());

-- ============================================================================
-- acc_contact_tag — tags/segments on contacts (UC-ACC-03.6)
-- ============================================================================
CREATE TABLE IF NOT EXISTS acc_contact_tag (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id UUID NOT NULL REFERENCES contact(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, contact_id, tag)
);

CREATE INDEX IF NOT EXISTS idx_acc_contact_tag_tenant_id ON acc_contact_tag(tenant_id);
CREATE INDEX IF NOT EXISTS idx_acc_contact_tag_contact_id ON acc_contact_tag(contact_id);

ALTER TABLE acc_contact_tag ENABLE ROW LEVEL SECURITY;
ALTER TABLE acc_contact_tag FORCE ROW LEVEL SECURITY;

CREATE POLICY acc_contact_tag_tenant_isolation ON acc_contact_tag
    FOR ALL USING (tenant_id = get_current_org_id() AND get_current_org_not_deleted());
