-- Defense N3: `set_principal_kind` actor verification.
--
-- Migration 00129 introduced the SECURITY DEFINER function
-- `set_principal_kind(target_user, new_kind, actor, reason)`. The `actor`
-- argument was caller-supplied with NO server-side cross-check against the
-- authenticated session — meaning a malicious caller could attribute the
-- transition to ANY user UUID, forging the audit trail (defends against the
-- N3 audit-attribution forgery leak).
--
-- This migration replaces the function body with one that asserts the
-- supplied `actor` matches the session's `app.current_user_id` GUC (set by
-- `set_request_context()` / `RlsConnection`). Anything else aborts the
-- transaction before the UPDATE or the audit row is written.
--
-- The rest of the body is identical to 00129's; we copy it verbatim and
-- prepend the actor check.

CREATE OR REPLACE FUNCTION set_principal_kind(
    target_user UUID,
    new_kind    VARCHAR,
    actor       UUID,
    reason      TEXT
)
RETURNS users AS $$
DECLARE
    v_old_kind        VARCHAR;
    v_user            users;
    v_session_user_id UUID;
    v_raw_setting     TEXT;
BEGIN
    -- ---- N3 actor check ----------------------------------------------------
    -- The session must have an `app.current_user_id` GUC set (the per-request
    -- RLS plumbing always sets this via `set_request_context()`), AND the
    -- supplied `actor` argument must equal it. Otherwise abort: the caller is
    -- either bypassing the RLS plumbing entirely or trying to forge audit
    -- attribution.
    --
    -- `current_setting(name, true)` returns NULL when the GUC is unset
    -- (instead of raising), so we explicitly NULL-check then cast.
    v_raw_setting := current_setting('app.current_user_id', TRUE);
    IF v_raw_setting IS NULL OR v_raw_setting = '' THEN
        RAISE EXCEPTION
            'set_principal_kind: app.current_user_id is not set (call must go through RlsConnection / set_request_context)'
            USING ERRCODE = 'insufficient_privilege';
    END IF;

    BEGIN
        v_session_user_id := v_raw_setting::UUID;
    EXCEPTION WHEN OTHERS THEN
        RAISE EXCEPTION
            'set_principal_kind: app.current_user_id is not a valid UUID (%)', v_raw_setting
            USING ERRCODE = 'insufficient_privilege';
    END;

    IF actor IS NULL OR actor <> v_session_user_id THEN
        RAISE EXCEPTION
            'set_principal_kind: actor (%) must equal session user (%) — audit-attribution forgery rejected (N3)',
            actor, v_session_user_id
            USING ERRCODE = 'insufficient_privilege';
    END IF;

    -- ---- 00129 body (verbatim) ---------------------------------------------
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

    -- Audit row.
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
            'leak_guard', 'principal_kind_change',
            'actor_check', 'session_verified'
        ),
        jsonb_build_object('principal_kind', v_old_kind),
        jsonb_build_object('principal_kind', new_kind)
    );

    RETURN v_user;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

COMMENT ON FUNCTION set_principal_kind(UUID, VARCHAR, UUID, TEXT) IS
    'Phase 2 + N3: only audited path to mutate users.principal_kind. Rejects '
    'any call where `actor` does not match the session''s app.current_user_id '
    '(set by RlsConnection / set_request_context). Defends leaks #8, #12, and '
    'the N3 audit-attribution forgery vector.';
