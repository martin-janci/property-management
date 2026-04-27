package three.two.bit.ppt.reality.agency

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Agency models for Reality Portal mobile clients.
 *
 * Mirrors `RealityAgency` from reality-server (`backend/crates/db/src/models/reality_portal.rs`).
 * Only the subset of fields the mobile UI actually displays is exposed; the server may carry more
 * fields and the @SerialName-annotated client deserializer skips unknown keys.
 */
@Serializable
data class Agency(
    val id: String,
    val name: String,
    val slug: String,
    val email: String,
    val phone: String? = null,
    val website: String? = null,
    val street: String? = null,
    val city: String? = null,
    @SerialName("postal_code") val postalCode: String? = null,
    val country: String = "SK",
    @SerialName("logo_url") val logoUrl: String? = null,
    @SerialName("banner_url") val bannerUrl: String? = null,
    @SerialName("primary_color") val primaryColor: String? = null,
    val description: String? = null,
    @SerialName("created_at") val createdAt: String? = null,
)

/** Wraps a single agency for `GET /api/v1/agencies/{id}` and friends. */
@Serializable data class AgencyResponse(val agency: Agency)

/** Agency directory list response. */
@Serializable data class AgencyListResponse(val agencies: List<Agency>, val total: Int = 0)

/** A single member of an agency (used by the members tab on the agency detail screen). */
@Serializable
data class AgencyMember(
    val id: String,
    @SerialName("agency_id") val agencyId: String,
    @SerialName("user_id") val userId: String,
    val role: String,
    val name: String? = null,
    val email: String? = null,
    @SerialName("photo_url") val photoUrl: String? = null,
)

/** Agency members list response. */
@Serializable data class AgencyMembersResponse(val members: List<AgencyMember>, val total: Int = 0)
