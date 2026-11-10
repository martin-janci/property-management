-- Scope the `payment_webhook_dead_letters` RLS "manage" policy to managers
-- (#3341, follow-up to PR #3300 / #3284).
--
-- Background
-- ----------
-- The create migration (00240) shipped a `payment_webhook_dead_letters_manage`
-- policy whose comment promised "Admins/managers of the org (and super admins)
-- can manage (resolve) them", but whose USING / WITH CHECK predicate only
-- checked `organization_members` *membership* -- identical to the read policy.
-- The effect: ANY active org member (including a plain `resident`/`member`)
-- could resolve / mutate dead-letter rows, not just managers. Dead letters are
-- a financial reconciliation artifact (a record of a collected-but-unsettled
-- payment), so resolving one is a privileged operator action that must be
-- manager-gated, exactly like every comparable `*_manage` policy in the
-- financial schema (`invoices_manage`, `payments_manage`,
-- `financial_accounts_manage`, `fee_schedules_manage`, ... all carry
-- `AND om.role_type IN ('org_admin', 'manager')`).
--
-- This migration rewrites only the `manage` policy to add the manager role
-- check, mirroring the canonical financial manage-policy predicate. The read
-- (`_select`) policy is intentionally left untouched: any org member may still
-- *view* their org's dead letters; only managers may *resolve* them.
--
-- The unauthenticated webhook-receiver INSERT path is unaffected: the table is
-- ENABLE (not FORCE) RLS and the receiver writes through the owner role on a
-- plain pool with no `app.current_user_id` GUC set, so the owner continues to
-- bypass the policy on INSERT (see the 00240 header).

DROP POLICY IF EXISTS payment_webhook_dead_letters_manage ON payment_webhook_dead_letters;

-- Only org admins/managers of the org (and super admins) can manage (resolve)
-- dead letters -- mirrors `invoices_manage` / `payments_manage` (00040).
CREATE POLICY payment_webhook_dead_letters_manage ON payment_webhook_dead_letters
    FOR ALL
    USING (
        EXISTS (
            SELECT 1 FROM organization_members om
            WHERE om.organization_id = payment_webhook_dead_letters.organization_id
            AND om.user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
            AND om.role_type IN ('org_admin', 'manager')
        )
        OR is_super_admin()
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM organization_members om
            WHERE om.organization_id = payment_webhook_dead_letters.organization_id
            AND om.user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
            AND om.role_type IN ('org_admin', 'manager')
        )
        OR is_super_admin()
    );
