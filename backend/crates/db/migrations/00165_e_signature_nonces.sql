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
