-- Phase 2: Identity Unification — principal_kind escalation guards.
--
-- Defends leaks #8 ("public row's kind flipped to staff via mass-assignment")
-- and #12 ("escalation to platform = escalation to God"). The flip from one
-- kind to another is a security-critical state transition; it must NEVER be
-- a side effect of a generic UPDATE on the users table.
--
-- Design:
--   1. A row-level BEFORE UPDATE trigger on `users` REJECTS any change to
--      `principal_kind` unless the session has explicitly opted in by setting
--      a per-session GUC `app.principal_kind_change_authorized = 'true'`.
--   2. A SECURITY DEFINER function `set_principal_kind()` is the ONLY safe
--      caller — it sets the GUC, performs the UPDATE in the same transaction,
--      writes an `audit_logs` audit row, and clears the GUC.

-- ---------------------------------------------------------------------------
-- 1) Row-level guard trigger.
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION users_principal_kind_guard()
RETURNS TRIGGER AS $$
DECLARE
    v_authorized TEXT;
BEGIN
    -- No-op if the column is unchanged.
    IF NEW.principal_kind IS NOT DISTINCT FROM OLD.principal_kind THEN
        RETURN NEW;
    END IF;

    -- Validate the new value (defense-in-depth: the CHECK constraint already
    -- enforces this, but we want a clear, single point of failure).
    IF NEW.principal_kind NOT IN ('public', 'staff', 'platform') THEN
        RAISE EXCEPTION 'principal_kind must be one of public|staff|platform, got %', NEW.principal_kind
            USING ERRCODE = 'check_violation';
    END IF;

    -- The ONLY way through is via set_principal_kind() (which sets the GUC).
    -- A session-local GUC means a malicious caller cannot pre-arm this from
    -- a pooled connection — RlsConnection clears all `app.*` GUCs on release.
    v_authorized := COALESCE(current_setting('app.principal_kind_change_authorized', TRUE), 'false');
    IF v_authorized <> 'true' THEN
        RAISE EXCEPTION 'principal_kind change must go through set_principal_kind() (leak #8/#12 guard)'
            USING ERRCODE = 'insufficient_privilege';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS users_principal_kind_guard_trg ON users;
CREATE TRIGGER users_principal_kind_guard_trg
    BEFORE UPDATE OF principal_kind ON users
    FOR EACH ROW
    EXECUTE FUNCTION users_principal_kind_guard();

-- ---------------------------------------------------------------------------
-- 2) Audited mutator: the one and only path that authorizes the trigger.
--
--    SECURITY DEFINER so callers do not need direct UPDATE privilege on the
--    `users` table — they invoke this function instead and it performs the
--    write under the function-owner's privileges.
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION set_principal_kind(
    target_user UUID,
    new_kind    VARCHAR,
    actor       UUID,
    reason      TEXT
)
RETURNS users AS $$
DECLARE
    v_old_kind VARCHAR;
    v_user     users;
BEGIN
    IF new_kind NOT IN ('public', 'staff', 'platform') THEN
        RAISE EXCEPTION 'invalid principal_kind: %', new_kind
            USING ERRCODE = 'check_violation';
    END IF;

    SELECT principal_kind INTO v_old_kind FROM users WHERE id = target_user;
    IF v_old_kind IS NULL THEN
        RAISE EXCEPTION 'user % not found', target_user
            USING ERRCODE = 'no_data_found';
    END IF;

    -- Arm the guard for the duration of THIS UPDATE only. Use a transaction-
    -- local GUC (third arg = TRUE) so it cannot leak across statements or
    -- requests on a pooled connection.
    PERFORM set_config('app.principal_kind_change_authorized', 'true', TRUE);

    UPDATE users
       SET principal_kind = new_kind,
           updated_at     = NOW()
     WHERE id = target_user
     RETURNING * INTO v_user;

    -- Belt-and-braces: clear the flag immediately even within this transaction.
    PERFORM set_config('app.principal_kind_change_authorized', 'false', TRUE);

    -- Audit row. We use the existing immutable `audit_logs` infra so the
    -- principal_kind change shows up in the same trail as every other
    -- account-state mutation.
    INSERT INTO audit_logs (
        user_id, action, resource_type, resource_id, org_id, details,
        old_values, new_values
    )
    VALUES (
        actor,
        'account_updated'::audit_action,
        'users.principal_kind',
        target_user,
        NULL,
        jsonb_build_object(
            'reason',     reason,
            'target',     target_user,
            'old_kind',   v_old_kind,
            'new_kind',   new_kind,
            'leak_guard', 'principal_kind_change'
        ),
        jsonb_build_object('principal_kind', v_old_kind),
        jsonb_build_object('principal_kind', new_kind)
    );

    RETURN v_user;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

COMMENT ON FUNCTION set_principal_kind(UUID, VARCHAR, UUID, TEXT) IS
    'Phase 2: the only audited path to mutate users.principal_kind. Defends leaks #8 and #12. Writes an audit_logs row on every transition.';
