-- PAP-171 (defense-in-depth follow-up to PAP-167 / PAP-150): put the
-- api-ecosystem catalog + developer-portal tables under FORCE ROW LEVEL
-- SECURITY so the app role — which connects as the table OWNER — is no longer
-- exempt from their RLS policies.
--
-- Background
-- ----------
-- Migration 00102 ENABLEd RLS on these tables and attached correct policies,
-- but never FORCEd them. PostgreSQL exempts a table's OWNER from RLS unless
-- FORCE is set, and the production api-server connects as the owner, so the
-- policies were never a real backstop: authorization was enforced ONLY in the
-- handlers (`is_platform_admin()` gate for catalog writes, `user_id` owner
-- check for developer rows). This is the same owner-bypass class PAP-62 fixed
-- for the org-scoped tables in migration 00179; this migration closes it for
-- the user-scoped / global-catalog tables PAP-167 left on `acquire_public`.
--
-- The existing 00102 policies already key on the CANONICAL GUCs
-- (`app.current_user_id` for developer rows, `app.is_super_admin` for catalog
-- writes — both set by `set_request_context()`), so — unlike 00179 — no policy
-- needs its GUC expression rewritten. We only add FORCE. The matching handler
-- change routes these handlers from `acquire_public` to `acquire_with_rls`
-- (user + super-admin context) so the policies are actually exercised.
--
-- `integration_ratings` (the marketplace ratings table) is intentionally NOT
-- here: it is org-scoped and migration 00179 already FORCEd it + fixed its GUC.
--
-- Rollback
-- --------
-- This is a pure RLS tightening. To restore the prior behavior, run the down
-- block at the bottom (commented) which drops FORCE from each table; the 00102
-- policies and ENABLE stay intact, returning the owner role to exempt.

-- Global catalog: public read, super-admin write (policies USING is_super_admin()).
ALTER TABLE marketplace_integrations FORCE ROW LEVEL SECURITY;
ALTER TABLE connectors FORCE ROW LEVEL SECURITY;
ALTER TABLE connector_actions FORCE ROW LEVEL SECURITY;
ALTER TABLE api_documentation FORCE ROW LEVEL SECURITY;
ALTER TABLE api_code_samples FORCE ROW LEVEL SECURITY;

-- Developer portal: user-scoped (policies USING user_id = app.current_user_id
-- OR is_super_admin). These hold credential-class data, so the policy backstop
-- matters most here.
ALTER TABLE developer_accounts FORCE ROW LEVEL SECURITY;
ALTER TABLE developer_api_keys FORCE ROW LEVEL SECURITY;
ALTER TABLE developer_sandboxes FORCE ROW LEVEL SECURITY;

-- ---------------------------------------------------------------------------
-- Down (manual rollback — sqlx migrations are forward-only):
--
-- ALTER TABLE marketplace_integrations NO FORCE ROW LEVEL SECURITY;
-- ALTER TABLE connectors NO FORCE ROW LEVEL SECURITY;
-- ALTER TABLE connector_actions NO FORCE ROW LEVEL SECURITY;
-- ALTER TABLE api_documentation NO FORCE ROW LEVEL SECURITY;
-- ALTER TABLE api_code_samples NO FORCE ROW LEVEL SECURITY;
-- ALTER TABLE developer_accounts NO FORCE ROW LEVEL SECURITY;
-- ALTER TABLE developer_api_keys NO FORCE ROW LEVEL SECURITY;
-- ALTER TABLE developer_sandboxes NO FORCE ROW LEVEL SECURITY;
-- ---------------------------------------------------------------------------
