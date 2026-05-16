-- Phase 2: Identity Unification — user_merge_collisions audit/queue table.
--
-- Defends leak #7 ("merge collision: portal_users→users merge collapses two
-- different humans sharing an email"). When the merge migration finds a
-- portal_user whose email is already present in `users`, the merge is REFUSED
-- and a row is written here for human review.
--
-- Read access is restricted to super-admin via RLS — collision rows expose
-- cross-org email associations and must not leak through tenant queries.

CREATE TABLE IF NOT EXISTS user_merge_collisions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_table  VARCHAR(64) NOT NULL CHECK (source_table IN ('portal_users', 'users')),
    source_id     UUID NOT NULL,
    target_email  VARCHAR(320) NOT NULL,
    payload       JSONB NOT NULL,
    status        VARCHAR(20) NOT NULL DEFAULT 'pending'
                      CHECK (status IN ('pending', 'merged', 'rejected')),
    reviewed_by   UUID REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at   TIMESTAMPTZ NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_merge_collisions_email
    ON user_merge_collisions (LOWER(target_email));
CREATE INDEX IF NOT EXISTS idx_user_merge_collisions_status
    ON user_merge_collisions (status);

-- RLS — super-admin ONLY. There is no per-org context where this is the
-- right answer, so the policy is unconditionally super-admin-gated.
ALTER TABLE user_merge_collisions ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_merge_collisions FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS user_merge_collisions_super_admin ON user_merge_collisions;
CREATE POLICY user_merge_collisions_super_admin ON user_merge_collisions
    FOR ALL
    USING      (is_super_admin())
    WITH CHECK (is_super_admin());

COMMENT ON TABLE user_merge_collisions IS
    'Phase 2: rows written by the portal_users → users merge migration when an email collision is detected. Defends leak #7 (no silent merge of two different humans).';
