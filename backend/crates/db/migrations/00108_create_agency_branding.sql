-- Reality Portal: agency branding (UC-49).
--
-- Stores watermark and branding settings separate from the core agency record
-- to keep the agencies table lean and avoid touching migration 00032.

CREATE TABLE IF NOT EXISTS agency_branding (
    id                  UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    -- One branding record per agency.
    agency_id           UUID            NOT NULL UNIQUE REFERENCES reality_agencies(id) ON DELETE CASCADE,
    logo_url            TEXT,
    banner_url          TEXT,
    primary_color       TEXT,
    secondary_color     TEXT,
    -- Watermark style: logo | text | logo_and_text
    watermark_style     TEXT,
    -- Opacity 0.0–1.0 (NULL = use default 0.3).
    watermark_opacity   DOUBLE PRECISION CHECK (watermark_opacity IS NULL OR (watermark_opacity >= 0 AND watermark_opacity <= 1)),
    -- Position: top_left | top_right | bottom_left | bottom_right | center
    watermark_position  TEXT,
    -- URL of a custom watermark image (overrides logo_url for watermark).
    watermark_url       TEXT,
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_agency_branding_agency
    ON agency_branding(agency_id);

-- Auto-update updated_at.
CREATE OR REPLACE FUNCTION update_agency_branding_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_agency_branding_updated_at
    BEFORE UPDATE ON agency_branding
    FOR EACH ROW EXECUTE FUNCTION update_agency_branding_updated_at();
