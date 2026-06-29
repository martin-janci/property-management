-- Migration: 00204_acc_share_link_token_hash
-- ACC security hardening (UC-ACC-05.11): store share-link capability tokens as a
-- SHA-256 hash at rest instead of the raw token. The raw token is the credential
-- shown to the user once on creation; only its hash is persisted, and the public
-- share endpoint hashes the presented token to look the row up. A DB read no
-- longer leaks usable share credentials.
--
-- Pure rename of acc_share_link.token -> token_hash (added in 00202). Existing
-- rows (if any) carry raw tokens; this is a dev/pre-release table so no value
-- backfill is performed — those links simply won't resolve (acceptable: share
-- links are short-lived, revocable capabilities).

ALTER TABLE acc_share_link RENAME COLUMN token TO token_hash;

-- The unique constraint + lookup index defined on (tenant_id, token) / (token)
-- in 00202 follow the renamed column automatically (Postgres updates index
-- definitions on RENAME COLUMN), so no index changes are required here.

-- ---------------------------------------------------------------------------
-- Capability-based public read (UC-ACC-05.11).
--
-- The public, unauthenticated share endpoint (`GET /api/v1/share/{token}`)
-- resolves a link with NO tenant RLS context — it cannot know the tenant until
-- it has read the row. Under the 00202 FORCE-RLS `tenant_isolation` policy
-- (`tenant_id = get_current_org_id()`), an uncontexted SELECT matches zero rows,
-- so the endpoint would 404 every valid token when the app connects as a
-- non-superuser (production) role.
--
-- Add a permissive SELECT policy so the lookup-by-token_hash works without a
-- tenant context. Security model: the capability is the high-entropy raw token
-- (256-bit); only its SHA-256 hash is stored and the caller must present the
-- token to compute the hash. The row itself exposes no secret (tenant_id,
-- invoice_id, the hash, timestamps). Writes (INSERT/UPDATE/DELETE) remain
-- governed solely by the tenant_isolation FOR ALL policy — this policy grants
-- SELECT only.
CREATE POLICY acc_share_link_capability_read ON acc_share_link
    FOR SELECT USING (true);
