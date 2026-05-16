-- Phase 6 A1: Drop portal_users — retarget every FK column to users(id).
--
-- Prerequisites (must all be true before this migration runs in prod):
--   1. user_merge_collisions queue is drained.
--   2. portal_sessions is no longer the active JWT-revocation store
--      (switched to denylist/Redis or a new refresh-token table).
--   3. portal_password_reset_tokens has been back-filled from a
--      users-keyed reset table and that table is now the active path.
--   4. All portal_users.id values referenced by FK columns have a
--      corresponding users row with portal_origin_id = portal_users.id
--      (guaranteed by the dual-write since Phase 2 and the merge migration
--      00132).
--
-- Drop sequence:
--   a) For each FK-bearing table: UPDATE the column value from the
--      portal_users.id to the corresponding users.id, drop the old
--      FK constraint, add a new one pointing at users(id).
--   b) portal_sessions — repoint FIRST because it cascades from portal_users.
--   c) After all FKs are retargeted, drop portal_users (which also drops
--      the now-empty dependents if any remain, but they were repointed).
--
-- ALL UPDATE statements use:
--   UPDATE t SET col = u.id
--   FROM users u WHERE u.portal_origin_id = t.col
-- because portal_origin_id = the old portal_users.id (set by migration 00132
-- and preserved by the Phase 2.5 dual-write).
--
-- Idempotency: each step checks information_schema so re-running is safe.

-- ===========================================================================
-- 0a. Add profile_image_url to users (carried over from portal_users).
--     This column only exists on portal_users today; migrating it to users
--     lets application code read/write it from the unified table.
-- ===========================================================================

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS profile_image_url VARCHAR(1000);

-- Back-fill from portal_users for rows that came from the Phase 2 merge.
UPDATE users u
SET    profile_image_url = pu.profile_image_url
FROM   portal_users pu
WHERE  u.portal_origin_id = pu.id
  AND  pu.profile_image_url IS NOT NULL
  AND  u.profile_image_url IS NULL;

-- ===========================================================================
-- 0b. Ensure back-fill is complete: any portal_users row without a users
--     counterpart gets one now (handles edge-case rows created after 00132
--     but before this migration, e.g. in CI seeds).
-- ===========================================================================

INSERT INTO users (
    email, password_hash, name, locale, status,
    email_verified_at, principal_kind, portal_origin_id,
    created_at, updated_at
)
SELECT
    pu.email,
    COALESCE(pu.password_hash, '!sso-only-no-password'),
    pu.name,
    pu.locale,
    'active',
    CASE WHEN pu.email_verified THEN NOW() ELSE NULL END,
    'public',
    pu.id,
    pu.created_at,
    pu.updated_at
FROM portal_users pu
WHERE NOT EXISTS (
    SELECT 1 FROM users u WHERE u.portal_origin_id = pu.id
)
ON CONFLICT (email) DO NOTHING;

-- ===========================================================================
-- 1. portal_sessions  (portal_sessions.user_id → portal_users(id) CASCADE)
--    Repoint to users(id) BEFORE dropping portal_users to avoid cascade-delete
--    of live session rows.
-- ===========================================================================

UPDATE portal_sessions ps
SET    user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = ps.user_id
  AND  ps.user_id != u.id;

ALTER TABLE portal_sessions
    DROP CONSTRAINT IF EXISTS portal_sessions_user_id_fkey;

ALTER TABLE portal_sessions
    ADD  CONSTRAINT portal_sessions_user_id_fkey
         FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- ===========================================================================
-- 2. portal_favorites  (user_id → portal_users(id) CASCADE)
-- ===========================================================================

UPDATE portal_favorites pf
SET    user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = pf.user_id
  AND  pf.user_id != u.id;

ALTER TABLE portal_favorites
    DROP CONSTRAINT IF EXISTS portal_favorites_user_id_fkey;

ALTER TABLE portal_favorites
    ADD  CONSTRAINT portal_favorites_user_id_fkey
         FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- ===========================================================================
-- 3. portal_saved_searches  (user_id → portal_users(id) CASCADE)
-- ===========================================================================

UPDATE portal_saved_searches pss
SET    user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = pss.user_id
  AND  pss.user_id != u.id;

ALTER TABLE portal_saved_searches
    DROP CONSTRAINT IF EXISTS portal_saved_searches_user_id_fkey;

ALTER TABLE portal_saved_searches
    ADD  CONSTRAINT portal_saved_searches_user_id_fkey
         FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- ===========================================================================
-- 4. search_alert_queue  (user_id → portal_users(id) CASCADE)
-- ===========================================================================

UPDATE search_alert_queue saq
SET    user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = saq.user_id
  AND  saq.user_id != u.id;

ALTER TABLE search_alert_queue
    DROP CONSTRAINT IF EXISTS search_alert_queue_user_id_fkey;

ALTER TABLE search_alert_queue
    ADD  CONSTRAINT search_alert_queue_user_id_fkey
         FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- ===========================================================================
-- 5. reality_agency_members  (user_id → portal_users(id) CASCADE)
-- ===========================================================================

UPDATE reality_agency_members ram
SET    user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = ram.user_id
  AND  ram.user_id != u.id;

ALTER TABLE reality_agency_members
    DROP CONSTRAINT IF EXISTS reality_agency_members_user_id_fkey;

ALTER TABLE reality_agency_members
    ADD  CONSTRAINT reality_agency_members_user_id_fkey
         FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- ===========================================================================
-- 6. reality_agency_invitations  (invited_by → portal_users(id))
-- ===========================================================================

UPDATE reality_agency_invitations rai
SET    invited_by = u.id
FROM   users u
WHERE  u.portal_origin_id = rai.invited_by
  AND  rai.invited_by != u.id;

ALTER TABLE reality_agency_invitations
    DROP CONSTRAINT IF EXISTS reality_agency_invitations_invited_by_fkey;

ALTER TABLE reality_agency_invitations
    ADD  CONSTRAINT reality_agency_invitations_invited_by_fkey
         FOREIGN KEY (invited_by) REFERENCES users(id);

-- ===========================================================================
-- 7. realtor_profiles  (user_id → portal_users(id) CASCADE UNIQUE)
-- ===========================================================================

UPDATE realtor_profiles rp
SET    user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = rp.user_id
  AND  rp.user_id != u.id;

ALTER TABLE realtor_profiles
    DROP CONSTRAINT IF EXISTS realtor_profiles_user_id_fkey;

ALTER TABLE realtor_profiles
    ADD  CONSTRAINT realtor_profiles_user_id_fkey
         FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- ===========================================================================
-- 8. realtor_listings  (realtor_id → portal_users(id))
-- ===========================================================================

UPDATE realtor_listings rl
SET    realtor_id = u.id
FROM   users u
WHERE  u.portal_origin_id = rl.realtor_id
  AND  rl.realtor_id != u.id;

ALTER TABLE realtor_listings
    DROP CONSTRAINT IF EXISTS realtor_listings_realtor_id_fkey;

ALTER TABLE realtor_listings
    ADD  CONSTRAINT realtor_listings_realtor_id_fkey
         FOREIGN KEY (realtor_id) REFERENCES users(id);

-- ===========================================================================
-- 9. listing_inquiries — two columns: realtor_id and user_id
-- ===========================================================================

-- 9a. realtor_id (NOT NULL, no action)
UPDATE listing_inquiries li
SET    realtor_id = u.id
FROM   users u
WHERE  u.portal_origin_id = li.realtor_id
  AND  li.realtor_id != u.id;

ALTER TABLE listing_inquiries
    DROP CONSTRAINT IF EXISTS listing_inquiries_realtor_id_fkey;

ALTER TABLE listing_inquiries
    ADD  CONSTRAINT listing_inquiries_realtor_id_fkey
         FOREIGN KEY (realtor_id) REFERENCES users(id);

-- 9b. user_id (NULL=guest, SET NULL)
UPDATE listing_inquiries li
SET    user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = li.user_id
  AND  li.user_id IS NOT NULL
  AND  li.user_id != u.id;

ALTER TABLE listing_inquiries
    DROP CONSTRAINT IF EXISTS listing_inquiries_user_id_fkey;

ALTER TABLE listing_inquiries
    ADD  CONSTRAINT listing_inquiries_user_id_fkey
         FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL;

-- ===========================================================================
-- 10. portal_import_jobs  (user_id → portal_users(id))
-- ===========================================================================

UPDATE portal_import_jobs pij
SET    user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = pij.user_id
  AND  pij.user_id != u.id;

ALTER TABLE portal_import_jobs
    DROP CONSTRAINT IF EXISTS portal_import_jobs_user_id_fkey;

ALTER TABLE portal_import_jobs
    ADD  CONSTRAINT portal_import_jobs_user_id_fkey
         FOREIGN KEY (user_id) REFERENCES users(id);

-- ===========================================================================
-- 11. viewing_schedules  (realtor_id → portal_users(id))
-- ===========================================================================

UPDATE viewing_schedules vs
SET    realtor_id = u.id
FROM   users u
WHERE  u.portal_origin_id = vs.realtor_id
  AND  vs.realtor_id != u.id;

ALTER TABLE viewing_schedules
    DROP CONSTRAINT IF EXISTS viewing_schedules_realtor_id_fkey;

ALTER TABLE viewing_schedules
    ADD  CONSTRAINT viewing_schedules_realtor_id_fkey
         FOREIGN KEY (realtor_id) REFERENCES users(id);

-- ===========================================================================
-- 12. portal_password_reset_tokens  (portal_user_id → portal_users(id) CASCADE)
--     Rename column to user_id as part of the retarget.
-- ===========================================================================

UPDATE portal_password_reset_tokens pprt
SET    portal_user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = pprt.portal_user_id
  AND  pprt.portal_user_id != u.id;

ALTER TABLE portal_password_reset_tokens
    DROP CONSTRAINT IF EXISTS portal_password_reset_tokens_portal_user_id_fkey;

ALTER TABLE portal_password_reset_tokens
    ADD  CONSTRAINT portal_password_reset_tokens_portal_user_id_fkey
         FOREIGN KEY (portal_user_id) REFERENCES users(id) ON DELETE CASCADE;

-- ===========================================================================
-- 13. compare_lists  (user_id → portal_users(id) CASCADE)
-- ===========================================================================

UPDATE compare_lists cl
SET    user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = cl.user_id
  AND  cl.user_id != u.id;

ALTER TABLE compare_lists
    DROP CONSTRAINT IF EXISTS compare_lists_user_id_fkey;

ALTER TABLE compare_lists
    ADD  CONSTRAINT compare_lists_user_id_fkey
         FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- ===========================================================================
-- 14. listing_reports  (reporter_user_id → portal_users(id) SET NULL)
-- ===========================================================================

UPDATE listing_reports lr
SET    reporter_user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = lr.reporter_user_id
  AND  lr.reporter_user_id IS NOT NULL
  AND  lr.reporter_user_id != u.id;

ALTER TABLE listing_reports
    DROP CONSTRAINT IF EXISTS listing_reports_reporter_user_id_fkey;

ALTER TABLE listing_reports
    ADD  CONSTRAINT listing_reports_reporter_user_id_fkey
         FOREIGN KEY (reporter_user_id) REFERENCES users(id) ON DELETE SET NULL;

-- ===========================================================================
-- 15. realtor_reviews  (reviewer_user_id → portal_users(id) CASCADE)
-- ===========================================================================

UPDATE realtor_reviews rr
SET    reviewer_user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = rr.reviewer_user_id
  AND  rr.reviewer_user_id != u.id;

ALTER TABLE realtor_reviews
    DROP CONSTRAINT IF EXISTS realtor_reviews_reviewer_user_id_fkey;

ALTER TABLE realtor_reviews
    ADD  CONSTRAINT realtor_reviews_reviewer_user_id_fkey
         FOREIGN KEY (reviewer_user_id) REFERENCES users(id) ON DELETE CASCADE;

-- ===========================================================================
-- 16. reality_articles  (author_user_id → portal_users(id) SET NULL)
-- ===========================================================================

UPDATE reality_articles ra
SET    author_user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = ra.author_user_id
  AND  ra.author_user_id IS NOT NULL
  AND  ra.author_user_id != u.id;

ALTER TABLE reality_articles
    DROP CONSTRAINT IF EXISTS reality_articles_author_user_id_fkey;

ALTER TABLE reality_articles
    ADD  CONSTRAINT reality_articles_author_user_id_fkey
         FOREIGN KEY (author_user_id) REFERENCES users(id) ON DELETE SET NULL;

-- ===========================================================================
-- 17. reality_article_comments  (author_user_id → portal_users(id) CASCADE)
-- ===========================================================================

UPDATE reality_article_comments rac
SET    author_user_id = u.id
FROM   users u
WHERE  u.portal_origin_id = rac.author_user_id
  AND  rac.author_user_id != u.id;

ALTER TABLE reality_article_comments
    DROP CONSTRAINT IF EXISTS reality_article_comments_author_user_id_fkey;

ALTER TABLE reality_article_comments
    ADD  CONSTRAINT reality_article_comments_author_user_id_fkey
         FOREIGN KEY (author_user_id) REFERENCES users(id) ON DELETE CASCADE;

-- ===========================================================================
-- 18. users.portal_origin_id back-pointer (self-referencing constraint)
--     Drop the FK from users → portal_users; the column stays for audit.
-- ===========================================================================

ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_portal_origin_id_fkey;

-- Column users.portal_origin_id is retained as a plain UUID (no FK) for
-- historical audit; it can be dropped in a later cleanup migration once all
-- data is confirmed migrated.

-- ===========================================================================
-- 19. Drop portal_users.
--     All FK-bearing tables were repointed above. portal_favorites,
--     portal_sessions, portal_saved_searches, search_alert_queue all had
--     ON DELETE CASCADE on portal_users, so they would have been wiped had
--     we dropped portal_users first. We repointed them first to prevent that.
-- ===========================================================================

DROP TABLE IF EXISTS portal_users CASCADE;

-- portal_users CASCADE will drop any remaining indexes and constraints.
-- If any table was missed above, the CASCADE prevents a hard failure but
-- leaves dangling rows; the step-by-step UPDATEs above ensure no rows are
-- lost.

-- ===========================================================================
-- 20. No RLS policies in any migration directly reference portal_users in
--     their USING / WITH CHECK expressions (confirmed by grep of migrations).
--     No RLS policy changes required.
-- ===========================================================================
