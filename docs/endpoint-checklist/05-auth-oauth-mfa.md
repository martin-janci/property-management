# Auth, OAuth & MFA endpoints

Group covers `routes/auth.rs`, `routes/oauth.rs` (public `/api/v1/oauth` + admin `/api/v1/admin/oauth`), `routes/mfa.rs` (`/api/v1/auth/mfa` + recovery at `/api/v1/users/me/mfa/recovery-codes`), and `routes/admin/mfa/*` (nested under `/api/v1/admin/mfa`). All routers are mounted in `src/lib.rs`; none are stubs.

| Method + Path | Handler | Status | Test | Notes |
|---|---|---|---|---|
| `POST /api/v1/auth/register` | `auth.rs:register` | done | `auth_enumeration_tests.rs` | Asserts 201 generic + enum safety |
| `GET /api/v1/auth/verify-email` | `auth.rs:verify_email` | partial | — | Real logic, no test |
| `POST /api/v1/auth/resend-verification` | `auth.rs:resend_verification` | partial | — | Real logic, no test |
| `POST /api/v1/auth/login` | `auth.rs:login` | done | `auth_enumeration_tests.rs` | Asserts 200/401 + accessToken |
| `POST /api/v1/auth/refresh` | `auth.rs:refresh_token` | done | `endpoints_smoke_tests.rs` | Empty-token rejection asserted |
| `POST /api/v1/auth/logout` | `auth.rs:logout` | partial | — | Real logic, no test |
| `POST /api/v1/auth/forgot-password` | `auth.rs:forgot_password` | partial | — | Real logic, no test |
| `POST /api/v1/auth/reset-password` | `auth.rs:reset_password` | partial | — | Real logic, no test |
| `GET /api/v1/auth/sessions` | `auth.rs:list_sessions` | partial | — | Real logic, no test |
| `POST /api/v1/auth/sessions/revoke` | `auth.rs:revoke_session` | partial | — | Real logic, no test |
| `POST /api/v1/auth/sessions/revoke-all` | `auth.rs:revoke_all_sessions` | partial | — | Real logic, no test |
| `GET /api/v1/auth/me` | `auth.rs:get_me` | partial | — | Real logic, no test |
| `PATCH /api/v1/auth/me` | `auth.rs:update_me` | partial | — | Real logic, no test |
| `GET /api/v1/oauth/authorize` | `oauth.rs:authorize_get` | done | `oauth_authorization_server_test.rs` | Consent page; auth enforced |
| `POST /api/v1/oauth/authorize` | `oauth.rs:authorize_post` | done | `oauth_integration_tests.rs` | Consent -> code |
| `POST /api/v1/oauth/token` | `oauth.rs:token` | done | `oauth_authorization_server_test.rs` | Code + refresh grants |
| `POST /api/v1/oauth/revoke` | `oauth.rs:revoke` | done | `oauth_authz_server_edge_tests.rs` | RFC 7009 client auth |
| `POST /api/v1/oauth/introspect` | `oauth.rs:introspect` | done | `oauth_token_introspection_rotation_test.rs` | RFC 7662 |
| `GET /api/v1/oauth/grants` | `oauth.rs:list_user_grants` | done | `oauth_integration_tests.rs` | Lists user grants |
| `DELETE /api/v1/oauth/grants/{client_id}` | `oauth.rs:revoke_user_grant` | done | `oauth_integration_tests.rs` | Includes 404 path |
| `POST /api/v1/admin/oauth/clients` | `oauth.rs:register_client` | done | `oauth_client_registration_test.rs` | Capability-gated |
| `GET /api/v1/admin/oauth/clients` | `oauth.rs:list_clients` | done | `oauth_client_registration_test.rs` | |
| `GET /api/v1/admin/oauth/clients/{id}` | `oauth.rs:get_client` | done | `oauth_client_registration_test.rs` | |
| `PATCH /api/v1/admin/oauth/clients/{id}` | `oauth.rs:update_client` | done | `oauth_client_registration_test.rs` | |
| `DELETE /api/v1/admin/oauth/clients/{id}` | `oauth.rs:revoke_client` | done | `oauth_client_registration_test.rs` | |
| `POST /api/v1/admin/oauth/clients/{id}/regenerate-secret` | `oauth.rs:regenerate_client_secret` | done | `oauth_client_registration_test.rs` | |
| `POST /api/v1/auth/mfa/setup` | `mfa.rs:setup_mfa` | partial | — | Real logic, no test |
| `POST /api/v1/auth/mfa/verify` | `mfa.rs:verify_mfa_setup` | partial | — | Setup completion; not exercised |
| `POST /api/v1/auth/mfa/disable` | `mfa.rs:disable_mfa` | done | `mfa_disable_rls_scope_tests.rs` | RLS scope asserted |
| `GET /api/v1/auth/mfa/status` | `mfa.rs:mfa_status` | partial | — | Real logic, no test |
| `POST /api/v1/auth/mfa/backup-codes/regenerate` | `mfa.rs:regenerate_backup_codes` | partial | — | Real logic, no test |
| `POST /api/v1/users/me/mfa/recovery-codes/verify` | `mfa.rs:verify_recovery_code` | done | `mfa_recovery_cross_user_idor_tests.rs` | Cross-user IDOR asserted |
| `POST /api/v1/admin/mfa/enroll/start` | `admin/mfa/enroll.rs:start_enroll` | partial | — | Real logic, no test |
| `POST /api/v1/admin/mfa/enroll/verify` | `admin/mfa/enroll.rs:verify_enroll` | partial | — | Real logic, no test |
| `POST /api/v1/admin/mfa/verify` | `admin/mfa/verify.rs:verify_step_up` | done | `admin_mfa_step_up_tests.rs` | TOTP step-up |
| `POST /api/v1/admin/mfa/recovery/use` | `admin/mfa/recovery.rs:use_recovery` | done | `admin_mfa_recovery_tests.rs` | Single-use code |
| `POST /api/v1/admin/mfa/disable` | `admin/mfa/disable.rs:disable_mfa` | done | `admin_mfa_disable_tests.rs` | TOTP/recovery confirm |

## Tally
done: 19  partial: 17  stub: 0  missing: 0  total: 36
