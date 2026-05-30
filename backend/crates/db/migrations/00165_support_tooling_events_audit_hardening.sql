-- Migration 00165: Support Tooling Events — Audit Hardening (closes #699)
--
-- Hardens the support_tooling_events table (created in migration 00163) on
-- two axes that were flagged in code review of PR #654:
--
--   1. FK to users(id) used ON DELETE CASCADE — that lets a deleted admin
--      account silently take its audit trail with it, bypassing the
--      append-only immutability triggers (FK referential actions run before
--      row-level triggers). Audit rows must outlive their referent.
--
--      We switch to ON DELETE RESTRICT so user deletion now refuses while
--      events exist, forcing an explicit archive workflow. This is stronger
--      than SET NULL (which would lose attribution) and is the right default
--      for a tamper-evident admin activity log.
--
--   2. No RLS was enabled on the table. The system-scoped table is currently
--      guarded only at the application layer (extract_super_admin_token).
--      We add database-level defence-in-depth so any read-replica / analytics
--      / misconfigured service role hitting the DB directly is denied by
--      default. The api-server runs with BYPASSRLS or as the table owner;
--      this change does not affect application behaviour.
--
-- The migration is additive — schema is otherwise unchanged.

-- 1. Replace ON DELETE CASCADE with ON DELETE RESTRICT on admin_user_id.
ALTER TABLE support_tooling_events
    DROP CONSTRAINT support_tooling_events_admin_user_id_fkey,
    ADD CONSTRAINT support_tooling_events_admin_user_id_fkey
        FOREIGN KEY (admin_user_id) REFERENCES users(id) ON DELETE RESTRICT;

-- 2. Enable Row Level Security with a super-admin-only policy. Matches the
--    pattern used by audit_logs (migration 00025) for system-scoped admin
--    activity tables.
ALTER TABLE support_tooling_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE support_tooling_events FORCE ROW LEVEL SECURITY;

-- Super admin can read/write all support tooling events.
CREATE POLICY support_tooling_events_super_admin ON support_tooling_events
    FOR ALL
    USING (is_super_admin())
    WITH CHECK (is_super_admin());

COMMENT ON TABLE support_tooling_events IS
    'Append-only audit trail for admin support-tooling actions (Support Data page). '
    'RLS: super-admin only. FK to users uses ON DELETE RESTRICT to preserve attribution.';
