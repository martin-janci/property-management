-- Issue #673 follow-up to PR #542:
-- Persist e-signature nonces server-side so the future /sign consumer
-- can enforce one-shot use of a signed envelope. Without this table,
-- an attacker who intercepts a signed signing URL can replay it for
-- the entire 7-day TTL (the nonce is bound into the HMAC, but never
-- compared against any stored value).
--
-- Schema rationale:
-- * `envelope_id` = `signature_requests.id` in this codebase. The
--   column name keeps the door open for an external-provider envelope
--   later without an ALTER.
-- * `nonce` is the UUID returned by `LightweightProvider::build_signing_url`
--   (issued fresh per invitation/reminder).
-- * `(envelope_id, nonce)` is UNIQUE — INSERT is the consume operation.
--   A second INSERT with the same pair raises 23505, which the API layer
--   translates into 409 CONFLICT.
-- * `used_at` is when the nonce was first recorded as consumed
--   (currently: at link-issue time; once /sign lands, the consumer will
--   be the writer and `issued_at` semantics can be split if needed).
CREATE TABLE IF NOT EXISTS e_signature_nonces (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    envelope_id UUID NOT NULL REFERENCES signature_requests(id) ON DELETE CASCADE,
    nonce       UUID NOT NULL,
    used_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT e_signature_nonces_envelope_nonce_unique UNIQUE (envelope_id, nonce)
);

-- Lookup index for the replay check: given an envelope + nonce, does a row exist?
-- The unique constraint above already creates a btree on (envelope_id, nonce),
-- so no extra index is needed for the hot read path.

-- Per-envelope cleanup / audit query: list all nonces issued for an envelope.
CREATE INDEX IF NOT EXISTS idx_e_signature_nonces_envelope_id
    ON e_signature_nonces (envelope_id);

COMMENT ON TABLE e_signature_nonces IS
    'One row per issued e-signature link nonce. Unique (envelope_id, nonce) enforces one-shot use; replays raise 23505 / 409.';

-- -----------------------------------------------------------------------------
-- Row-Level Security
-- -----------------------------------------------------------------------------
-- This table is tenant-scoped transitively: every row FKs to a
-- `signature_requests` row, which is itself org-isolated. The RLS
-- policy-coverage gate (scripts/check-rls-coverage.sh) therefore requires at
-- least one policy here. Two policies, modelled on `device_push_tokens`
-- (00157): a service policy for the system writer (nonce issuance runs with no
-- user context), and a tenant policy that derives the org from the parent
-- signature request so org members can read/audit their own envelopes' nonces.
--
-- NB: ENABLE (not FORCE) — this matches the sibling envelope tables
-- `signature_requests` (00037) and `documents` (00020), so the table owner
-- used by `#[sqlx::test]` and trusted service paths is not itself constrained,
-- exactly as those siblings behave.
ALTER TABLE e_signature_nonces ENABLE ROW LEVEL SECURITY;

-- Service writer: nonce issuance / consume runs on the bare pool with no
-- `app.current_user_id` set. Mirrors device_push_tokens_service_policy.
CREATE POLICY e_signature_nonces_service_policy
    ON e_signature_nonces
    FOR ALL
    USING (NULLIF(current_setting('app.current_user_id', TRUE), '') IS NULL)
    WITH CHECK (NULLIF(current_setting('app.current_user_id', TRUE), '') IS NULL);

-- Tenant read/audit: org admins/managers (or super admins) may see the nonces
-- of envelopes belonging to their organization. Org is derived through the
-- parent signature_requests row.
CREATE POLICY e_signature_nonces_tenant_policy
    ON e_signature_nonces
    FOR ALL
    USING (
        EXISTS (
            SELECT 1
            FROM signature_requests sr
            JOIN organization_members om
              ON om.organization_id = sr.organization_id
            WHERE sr.id = e_signature_nonces.envelope_id
              AND om.user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
              AND om.role_type IN ('org_admin', 'manager')
        )
        OR is_super_admin()
    )
    WITH CHECK (
        EXISTS (
            SELECT 1
            FROM signature_requests sr
            JOIN organization_members om
              ON om.organization_id = sr.organization_id
            WHERE sr.id = e_signature_nonces.envelope_id
              AND om.user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
              AND om.role_type IN ('org_admin', 'manager')
        )
        OR is_super_admin()
    );
