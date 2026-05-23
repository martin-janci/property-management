package three.two.bit.ppt.reality.models

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Shared models for Reality Portal.
 *
 * These will be generated from OpenAPI spec.
 */
@Serializable
data class User(
    val id: String,
    val email: String,
    @SerialName("display_name") val displayName: String,
    @SerialName("avatar_url") val avatarUrl: String? = null,
)

@Serializable
data class TenantContext(
    @SerialName("tenant_id") val tenantId: String,
    @SerialName("tenant_name") val tenantName: String,
    val role: String,
)

@Serializable
data class LoginRequest(
    val email: String,
    val password: String,
    @SerialName("two_factor_code") val twoFactorCode: String? = null,
)

@Serializable
data class LoginResponse(
    @SerialName("access_token") val accessToken: String,
    @SerialName("refresh_token") val refreshToken: String,
    @SerialName("expires_in") val expiresIn: Int,
    val user: User,
    val tenants: List<TenantMembership>,
)

@Serializable
data class TenantMembership(
    @SerialName("tenant_id") val tenantId: String,
    @SerialName("tenant_name") val tenantName: String,
    val role: String,
)

@Serializable
data class ErrorResponse(
    val code: String,
    val message: String,
    @SerialName("request_id") val requestId: String? = null,
    val timestamp: String,
)
