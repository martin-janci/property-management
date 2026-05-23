package three.two.bit.ppt.reality.auth

import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.client.statement.*
import io.ktor.http.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import three.two.bit.ppt.reality.api.ApiConfig
import three.two.bit.ppt.reality.api.HttpClientProvider

/**
 * SSO Service for Reality Portal (Epic 10A-SSO).
 *
 * Handles mobile deep-link SSO flow:
 * 1. PM app calls createMobileToken() with PM access token
 * 2. PM app opens Reality Portal via deep-link: reality://sso?token=xxx
 * 3. Reality Portal validates token and creates session
 *
 * Note: Uses shared HttpClientProvider to avoid resource leaks. The baseUrl is obtained from
 * ApiConfig which must be initialized at app startup.
 */
class SsoService {
    private val client = HttpClientProvider.client
    private val baseUrl: String
        get() = ApiConfig.requireBaseUrl()

    private val _authState = MutableStateFlow<AuthState>(AuthState.Unauthenticated)
    val authState: StateFlow<AuthState> = _authState.asStateFlow()

    private var sessionToken: String? = null

    /**
     * Create a mobile SSO token from PM access token.
     *
     * Call this from PM app to get a short-lived token for SSO.
     */
    suspend fun createMobileToken(pmAccessToken: String): Result<MobileSsoTokenResponse> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/sso/mobile/token") {
                    setBody(CreateMobileSsoTokenRequest(pmAccessToken))
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else {
                val error: SsoError = response.body()
                Result.failure(SsoException(error.error, error.errorDescription))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Validate SSO token from deep-link and create session.
     *
     * Call this when Reality Portal receives the deep-link.
     */
    suspend fun validateAndLogin(ssoToken: String): Result<SessionResponse> {
        _authState.value = AuthState.Loading

        return try {
            val response =
                client.post("$baseUrl/api/v1/sso/mobile/validate") {
                    setBody(ValidateMobileSsoTokenRequest(ssoToken))
                }

            if (response.status.isSuccess()) {
                val session: SessionResponse = response.body()
                sessionToken = session.sessionToken
                _authState.value = AuthState.Authenticated(session.user, session.sessionToken)
                Result.success(session)
            } else {
                val error: SsoError = response.body()
                _authState.value = AuthState.Error(error.errorDescription ?: error.error)
                Result.failure(SsoException(error.error, error.errorDescription))
            }
        } catch (e: Exception) {
            _authState.value = AuthState.Error(e.message ?: "Unknown error")
            Result.failure(e)
        }
    }

    /** Get current session information. */
    suspend fun getSession(): Result<SessionInfo> {
        val token =
            sessionToken ?: return Result.failure(SsoException("no_session", "Not authenticated"))

        return try {
            val response =
                client.get("$baseUrl/api/v1/sso/session") {
                    header(HttpHeaders.Authorization, "Bearer $token")
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else {
                if (response.status == HttpStatusCode.Unauthorized) {
                    logout()
                }
                val error: SsoError = response.body()
                Result.failure(SsoException(error.error, error.errorDescription))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Refresh the current session. */
    suspend fun refreshSession(): Result<SessionInfo> {
        val token =
            sessionToken ?: return Result.failure(SsoException("no_session", "Not authenticated"))

        return try {
            val response =
                client.post("$baseUrl/api/v1/sso/refresh") {
                    header(HttpHeaders.Authorization, "Bearer $token")
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else {
                if (response.status == HttpStatusCode.Unauthorized) {
                    logout()
                }
                val error: SsoError = response.body()
                Result.failure(SsoException(error.error, error.errorDescription))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Logout and clear session. */
    fun logout() {
        sessionToken = null
        _authState.value = AuthState.Unauthenticated
    }

    /**
     * Sign in with email and password (UC-47.2).
     *
     * Calls reality-server `POST /api/v1/users/login` and updates `authState` on success. The
     * backend returns a session token + user info; both are fed into `AuthState.Authenticated` so
     * downstream consumers (HomeScreen, FavoritesScreen, etc.) react automatically.
     */
    suspend fun loginWithPassword(email: String, password: String): Result<AuthLoginResponse> {
        _authState.value = AuthState.Loading
        return try {
            val response =
                client.post("$baseUrl/api/v1/users/login") {
                    setBody(AuthLoginRequest(email, password))
                }
            if (response.status.isSuccess()) {
                val payload: AuthLoginResponse = response.body()
                sessionToken = payload.token
                _authState.value =
                    AuthState.Authenticated(
                        user =
                            SsoUserInfo(
                                userId = payload.user.id,
                                email = payload.user.email,
                                name = payload.user.name,
                                avatarUrl = payload.user.profileImageUrl,
                            ),
                        sessionToken = payload.token,
                    )
                Result.success(payload)
            } else {
                val message =
                    runCatching { response.body<SsoError>() }
                        .map { it.errorDescription ?: it.error }
                        .getOrElse { "Sign in failed (HTTP ${response.status.value})" }
                _authState.value = AuthState.Error(message)
                Result.failure(SsoException("login_failed", message))
            }
        } catch (e: Exception) {
            _authState.value = AuthState.Error(e.message ?: "Unknown error")
            Result.failure(e)
        }
    }

    /**
     * Register a new account (UC-47.1).
     *
     * Calls reality-server `POST /api/v1/users/register`. The server does not issue a session here
     * — the caller should send the user through the login flow after success.
     */
    suspend fun register(email: String, password: String, name: String): Result<Unit> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/users/register") {
                    setBody(AuthRegisterRequest(email, password, name))
                }
            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else {
                val message =
                    runCatching { response.body<SsoError>() }
                        .map { it.errorDescription ?: it.error }
                        .getOrElse { "Registration failed (HTTP ${response.status.value})" }
                Result.failure(SsoException("register_failed", message))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Request a password reset email (UC-44.3 step 1).
     *
     * Calls reality-server `POST /api/v1/users/password-reset`. The server always returns 200 with
     * a generic message regardless of whether the email is registered (intentional — it would
     * otherwise leak account existence), so we treat any 2xx response as success.
     */
    suspend fun requestPasswordReset(email: String): Result<Unit> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/users/password-reset") {
                    setBody(PasswordResetRequest(email = email))
                }
            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else {
                Result.failure(
                    SsoException(
                        "password_reset_failed",
                        "Server rejected the password reset request: ${response.status}",
                    )
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Confirm a password reset using the emailed token (UC-44.3 step 2).
     *
     * Calls reality-server `POST /api/v1/users/password-reset/confirm`. The server returns 400 with
     * a human-readable message for invalid / expired tokens or weak passwords; we surface those
     * verbatim so the UI can display them.
     */
    suspend fun confirmPasswordReset(token: String, newPassword: String): Result<Unit> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/users/password-reset/confirm") {
                    setBody(PasswordResetConfirm(token = token, newPassword = newPassword))
                }
            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else {
                val message =
                    runCatching { response.bodyAsText() }
                        .getOrDefault("Reset failed: ${response.status}")
                Result.failure(SsoException("password_reset_failed", message))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Update the signed-in user's profile (UC-47.7).
     *
     * Calls reality-server `PUT /api/v1/users/me`. On success the updated user is pushed back into
     * `authState`.
     */
    suspend fun updateProfile(name: String): Result<SsoUserInfo> {
        val token =
            sessionToken ?: return Result.failure(SsoException("no_session", "Not authenticated"))
        return try {
            val response =
                client.put("$baseUrl/api/v1/users/me") {
                    header(HttpHeaders.Authorization, "Bearer $token")
                    setBody(ProfileUpdateRequest(name = name))
                }
            if (response.status.isSuccess()) {
                val updated: PortalUser = response.body()
                val ssoUser =
                    SsoUserInfo(
                        userId = updated.id,
                        email = updated.email,
                        name = updated.name,
                        avatarUrl = updated.profileImageUrl,
                    )
                val current = _authState.value
                if (current is AuthState.Authenticated) {
                    _authState.value = current.copy(user = ssoUser)
                }
                Result.success(ssoUser)
            } else {
                val message =
                    runCatching { response.body<SsoError>() }
                        .map { it.errorDescription ?: it.error }
                        .getOrElse { "Profile update failed (HTTP ${response.status.value})" }
                Result.failure(SsoException("update_failed", message))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Check if user is authenticated. */
    fun isAuthenticated(): Boolean = sessionToken != null

    /** Get the current session token for API calls. */
    fun getSessionToken(): String? = sessionToken

    /**
     * Restore session from stored token (e.g., from SharedPreferences/UserDefaults).
     *
     * Call this on app startup with the stored token.
     */
    suspend fun restoreSession(token: String): Boolean {
        sessionToken = token
        _authState.value = AuthState.Loading

        return getSession()
            .fold(
                onSuccess = { session ->
                    _authState.value =
                        AuthState.Authenticated(
                            user =
                                SsoUserInfo(
                                    userId = session.userId,
                                    email = session.email,
                                    name = session.name,
                                ),
                            sessionToken = token,
                        )
                    true
                },
                onFailure = {
                    sessionToken = null
                    _authState.value = AuthState.Unauthenticated
                    false
                },
            )
    }
}

/** SSO-specific exception. */
class SsoException(val error: String, val errorDescription: String?) :
    Exception(errorDescription ?: error)
