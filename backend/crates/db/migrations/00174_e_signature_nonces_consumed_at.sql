-- Issue #761 part 2: ship the `/sign` consumer endpoint with consume-on-use.
--
-- Migration 00170 created `e_signature_nonces` with a UNIQUE (envelope_id,
-- nonce) constraint and recorded a row at link-ISSUE time (`used_at` = when
-- the invitation/reminder email was generated). That gave replay-prevention
-- at the "issue twice" boundary but did NOT model single-use *consumption*:
-- the row existed before the signer ever clicked, so the consumer could not
-- tell "issued but not yet signed" from "already signed".
--
-- This migration splits the two states by adding a nullable `consumed_at`:
--   * row INSERTed at issue  -> consumed_at IS NULL  (issued, awaiting signer)
--   * /sign POST consumes it -> consumed_at = NOW()  (single-use spent)
--
-- The consumer flips NULL -> NOW() atomically
--   (UPDATE ... WHERE envelope_id=$1 AND nonce=$2 AND consumed_at IS NULL),
-- so a replay finds zero rows to update and is rejected with 409 CONFLICT.
-- The existing UNIQUE (envelope_id, nonce) still rejects double-issue.
ALTER TABLE e_signature_nonces
    ADD COLUMN IF NOT EXISTS consumed_at TIMESTAMPTZ;

COMMENT ON COLUMN e_signature_nonces.consumed_at IS
    'When the /sign consumer atomically consumed this nonce (single-use). NULL = issued (via build_signing_url) but not yet spent by a signer. Set NULL -> NOW() exactly once; a replay updates zero rows and is rejected 409.';

-- Partial index over the hot consumer predicate: "is there an unconsumed nonce
-- for this envelope?" Keeps the atomic UPDATE's WHERE clause index-backed
-- without bloating the index with already-spent rows.
CREATE INDEX IF NOT EXISTS idx_e_signature_nonces_unconsumed
    ON e_signature_nonces (envelope_id, nonce)
    WHERE consumed_at IS NULL;
