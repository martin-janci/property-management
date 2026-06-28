-- Epic 128: OCR Meter Preview — Story 128.1: process_meter_reading + submit_correction persistence

CREATE TABLE IF NOT EXISTS ocr_meter_corrections (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    submitted_by    UUID        NOT NULL REFERENCES users(id),
    meter_reading_id UUID       REFERENCES meter_readings(id) ON DELETE SET NULL,
    original_value  DECIMAL(15, 3) NOT NULL,
    corrected_value DECIMAL(15, 3) NOT NULL,
    image_url       TEXT        NOT NULL,
    bounding_box    JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ocr_corrections_org ON ocr_meter_corrections(organization_id);
CREATE INDEX IF NOT EXISTS idx_ocr_corrections_reading ON ocr_meter_corrections(meter_reading_id);
CREATE INDEX IF NOT EXISTS idx_ocr_corrections_submitted_by ON ocr_meter_corrections(submitted_by);

ALTER TABLE ocr_meter_corrections ENABLE ROW LEVEL SECURITY;

CREATE POLICY ocr_corrections_select ON ocr_meter_corrections
    FOR SELECT
    USING (
        EXISTS (
            SELECT 1 FROM organization_members om
            WHERE om.organization_id = ocr_meter_corrections.organization_id
              AND om.user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        )
        OR is_super_admin()
    );

CREATE POLICY ocr_corrections_insert ON ocr_meter_corrections
    FOR INSERT
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM organization_members om
            WHERE om.organization_id = ocr_meter_corrections.organization_id
              AND om.user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        )
        OR is_super_admin()
    );
