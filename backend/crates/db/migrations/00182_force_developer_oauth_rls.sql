-- Close the remaining 00102 owner-bypass gap left by 00180/PAP-171:
-- developer_oauth_apps, developer_oauth_grants, and api_key_usage_logs already
-- carry correct user-scoped RLS policies but were never FORCEd, so the owner
-- app role remained exempt. This is the same bypass class 00180 addressed for
-- the other eight api-ecosystem tables; client_secret_hash in
-- developer_oauth_apps makes this the highest-value target of the three.
ALTER TABLE developer_oauth_apps   FORCE ROW LEVEL SECURITY;
ALTER TABLE developer_oauth_grants FORCE ROW LEVEL SECURITY;
ALTER TABLE api_key_usage_logs     FORCE ROW LEVEL SECURITY;
