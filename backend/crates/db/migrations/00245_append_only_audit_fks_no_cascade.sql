-- Drop referential actions from the append-only audit trails (gap audit R6 P1-4).
--
-- Background
-- ----------
-- `audit_logs`, `vote_audit_log` and `layout_change_events` are append-only:
-- BEFORE UPDATE / BEFORE DELETE triggers reject every mutation
-- (`audit_logs_reject_mutation`, `prevent_audit_log_modification`,
-- `layout_change_events_immutable`). Their foreign keys, however, were
-- declared with `ON DELETE SET NULL` / `ON DELETE CASCADE`. Postgres executes
-- those referential actions as an UPDATE / DELETE on the audit table, which the
-- trigger rejects, so the *parent* delete fails:
--
--   * `DELETE FROM organizations` (tenant purge, `tenant-ops::purge_tenant`)
--     fails with "audit_logs is append-only: UPDATE rejected" for every org
--     that has ever written an audit row;
--   * any hard delete of a `users` row fails the same way (audit_logs,
--     vote_audit_log, layout_change_events);
--   * deleting a vote that has an audit entry fails because
--     `vote_audit_log.vote_id` cascades.
--
-- Fix
-- ---
-- An audit trail must outlive the rows it describes, so the ids it records
-- are historical references, not live relations. Drop the foreign keys and
-- keep the UUIDs as plain columns. The indexes on these columns are kept, so
-- lookups by user / org / vote are unchanged. Inserts keep working as before;
-- the application always writes ids that exist at insert time.

ALTER TABLE audit_logs DROP CONSTRAINT IF EXISTS audit_logs_user_id_fkey;
ALTER TABLE audit_logs DROP CONSTRAINT IF EXISTS audit_logs_org_id_fkey;

ALTER TABLE vote_audit_log DROP CONSTRAINT IF EXISTS vote_audit_log_user_id_fkey;
ALTER TABLE vote_audit_log DROP CONSTRAINT IF EXISTS vote_audit_log_vote_id_fkey;

ALTER TABLE layout_change_events DROP CONSTRAINT IF EXISTS layout_change_events_published_by_fkey;

COMMENT ON COLUMN audit_logs.user_id IS
    'Actor id at the time of the event. Historical reference, intentionally not a FK (append-only table, see 00245).';
COMMENT ON COLUMN audit_logs.org_id IS
    'Organization id at the time of the event. Historical reference, intentionally not a FK (append-only table, see 00245).';
COMMENT ON COLUMN vote_audit_log.vote_id IS
    'Vote id at the time of the event. Historical reference, intentionally not a FK (append-only table, see 00245).';
