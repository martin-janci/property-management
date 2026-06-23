-- Epic 18 / Story 18.2 (#1687): Guest ID-document upload + OCR extract seam.
--
-- Stores the uploaded ID-document blobs (by S3 key) per guest, plus the
-- result of a (Stage B) OCR extraction. The OCR extraction is best-effort and
-- never auto-applied to the guest record — the manager confirms via the
-- existing PUT /api/v1/rentals/guests/{id} flow.
--
-- Own-org table: carries its own `organization_id`. RLS org-isolation policy is
-- copied verbatim from migration 00112 (rental_* tables).

CREATE TABLE rental_guest_id_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guest_id UUID NOT NULL REFERENCES rental_guests(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    -- S3-compatible storage key for the uploaded document bytes.
    file_key VARCHAR(1000) NOT NULL,
    -- MIME type captured at upload time (png/jpeg/pdf allowlist enforced in app).
    mime VARCHAR(255) NOT NULL,
    -- User who uploaded the document.
    uploaded_by UUID REFERENCES users(id) ON DELETE SET NULL,

    -- Best-effort OCR extraction result (GuestIdExtraction JSON). NULL until
    -- the extract endpoint runs against a configured provider (Stage B).
    extraction_json JSONB,
    -- When the extraction was produced. NULL until extracted.
    extracted_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rental_guest_id_documents_guest_id
    ON rental_guest_id_documents (guest_id);
CREATE INDEX idx_rental_guest_id_documents_organization_id
    ON rental_guest_id_documents (organization_id);

-- ============================================
-- RLS: own-org isolation (verbatim from 00112)
-- ============================================
ALTER TABLE rental_guest_id_documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE rental_guest_id_documents FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rental_guest_id_documents_tenant_isolation ON rental_guest_id_documents;
CREATE POLICY rental_guest_id_documents_tenant_isolation ON rental_guest_id_documents
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());
