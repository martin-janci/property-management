-- gap-80-3: Add mediation_notes, resolution_notes, and resolved_at columns to disputes table.
-- These fields support the PATCH /disputes/{id}/resolve and PATCH /disputes/{id}/mediation-notes endpoints.

ALTER TABLE disputes
    ADD COLUMN IF NOT EXISTS resolved_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS resolution_notes TEXT,
    ADD COLUMN IF NOT EXISTS mediation_notes TEXT;

COMMENT ON COLUMN disputes.resolved_at IS 'Timestamp when the dispute was marked resolved';
COMMENT ON COLUMN disputes.resolution_notes IS 'Notes recorded by the manager when resolving the dispute';
COMMENT ON COLUMN disputes.mediation_notes IS 'Ongoing notes maintained by the assigned mediator or manager';
