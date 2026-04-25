package three.two.bit.ppt.reality.auth

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * SSO models for Reality Portal (Epic 10A-SSO).
 *
 * These models support mobile deep-link SSO flow between PM app and Reality Portal.
 */

/** User info from SSO authentication. */
@Serializable
data class SsoUserInfo(
    @SerialName("user_id") val userId: String,
    val email: String,
    val name: String,
    @SerialName("avatar_url") val avatarUrl: String? = null
)

/** Request to create a mobile SSO token. */
@Serializable
data class CreateMobileSsoTokenRequest(@SerialName("pm_access_token") val pmAccessToken: String)

/** Response containing the mobile SSO token. */
@Serializable
data class MobileSsoTokenResponse(
    @SerialName("sso_token") val ssoToken: String,
    @SerialName("expires_in") val expiresIn: Long,
    @SerialName("deep_link") val deepLink: String
)

/** Request to validate a mobile SSO token. */
@Serializable
data class ValidateMobileSsoTokenRequest(@SerialName("sso_token") val ssoToken: String)

/** Response after validating SSO token with session info. */
@Serializable
data class SessionResponse(
    @SerialName("session_token") val sessionToken: String,
    val user: SsoUserInfo,
    @SerialName("expires_in") val expiresIn: Long
)

/** Current session information. */
@Serializable
data class SessionInfo(
    @SerialName("user_id") val userId: String,
    val email: String,
    val name: String,
    @SerialName("expires_at") val expiresAt: String
)

/** SSO error response. */
@Serializable
data class SsoError(
    val error: String,
    @SerialName("error_description") val errorDescription: String? = null
)

/** Email/password login request (UC-47.2). */
@Serializable data class AuthLoginRequest(val email: String, val password: String)

/**
 * Login response from reality-server `POST /api/v1/users/login`.
 *
 * The reality-server returns a session token, expiry timestamp and user info. Fields with
 * snake_case names are mapped via `@SerialName`.
 */
@Serializable
data class AuthLoginResponse(
    val token: String,
    @SerialName("expires_at") val expiresAt: String,
    val user: PortalUser,
)

/** User info returned by reality-server in login + `/users/me` responses. */
@Serializable
data class PortalUser(
    val id: String,
    val email: String,
    val name: String,
    @SerialName("profile_image_url") val profileImageUrl: String? = null,
    @SerialName("is_linked_to_pm") val isLinkedToPm: Boolean = false,
)

/** Registration request (UC-47.1). Matches `POST /api/v1/users/register`. */
@Serializable
data class AuthRegisterRequest(
    val email: String,
    val password: String,
    val name: String,
)

/** Registration response — only contains a success message and the new user id. */
@Serializable
data class AuthRegisterResponse(
    val message: String,
    @SerialName("user_id") val userId: String,
)

/**
 * Password reset request (UC-47.4 step 1).
 *
 * Note: reality-server does not expose password-reset endpoints yet. The model is kept so the UI
 * can surface a clear error when the server eventually implements them.
 */
@Serializable data class PasswordResetRequest(val email: String)

/** Password reset confirmation (UC-47.4 step 2). See note on PasswordResetRequest. */
@Serializable data class PasswordResetConfirm(val token: String, val newPassword: String)

/**
 * Profile update payload (UC-47.7). Matches `PUT /api/v1/users/me` — reality-server accepts any of
 * `name`, `profile_image_url` and `locale`.
 */
@Serializable
data class ProfileUpdateRequest(
    val name: String? = null,
    @SerialName("profile_image_url") val profileImageUrl: String? = null,
    val locale: String? = null,
)

/** Authentication state for the app. */
sealed class AuthState {
    /** Not authenticated. */
    data object Unauthenticated : AuthState()

    /** Checking authentication status. */
    data object Loading : AuthState()

    /** Authenticated with session. */
    data class Authenticated(val user: SsoUserInfo, val sessionToken: String) : AuthState()

    /** Authentication error. */
    data class Error(val message: String) : AuthState()
}
