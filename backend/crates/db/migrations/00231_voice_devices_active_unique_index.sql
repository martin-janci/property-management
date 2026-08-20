-- Enforce the "at most one active voice-assistant device per
-- (organization_id, user_id, platform)" invariant at the DATABASE level (#2794).
--
-- Background
-- ----------
-- The account-linking path (`oauth_token_exchange`,
-- api-server routes/voice_webhooks.rs) upserts the single active device for an
-- `(org, user, platform)` tuple: a re-link rotates the tokens on the existing
-- row rather than minting an independent new one (stale-token accumulation, see
-- #2662 / migration 00226). Until now that invariant was upheld only by the
-- handler's sequential SELECT -> branch -> INSERT-or-UPDATE path, which two
-- concurrent re-link requests can interleave to produce two active rows. This
-- adds the partial UNIQUE index that makes the DB the single writer of the
-- invariant, so the handler's `INSERT ... ON CONFLICT ... DO UPDATE` upsert can
-- rely on it as the conflict arbiter and the race is closed.
--
-- Pre-existing duplicates
-- -----------------------
-- The pre-#2662 handler minted a fresh row on every re-link, so populated
-- environments may already hold several active rows per tuple. A UNIQUE index
-- build fails on such data, so first collapse duplicates: keep the
-- most-recently-linked active row and deactivate the rest. Deactivation (rather
-- than deletion) preserves history and matches how the linking flow retires a
-- superseded device.
WITH ranked AS (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY organization_id, user_id, platform
               ORDER BY linked_at DESC, created_at DESC, id DESC
           ) AS rn
    FROM voice_assistant_devices
    WHERE is_active = TRUE
)
UPDATE voice_assistant_devices AS d
SET is_active = FALSE,
    updated_at = NOW()
FROM ranked
WHERE d.id = ranked.id
  AND ranked.rn > 1;

-- The single-writer of the invariant. Partial (WHERE is_active = TRUE) so that
-- retired rows don't participate: any number of inactive rows per tuple is fine,
-- only one active row is permitted. This predicate is also the ON CONFLICT
-- arbiter used by `upsert_active_voice_device`.
CREATE UNIQUE INDEX IF NOT EXISTS uniq_voice_devices_active_org_user_platform
    ON voice_assistant_devices (organization_id, user_id, platform)
    WHERE is_active = TRUE;
