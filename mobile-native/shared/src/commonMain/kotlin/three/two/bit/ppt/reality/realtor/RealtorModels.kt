package three.two.bit.ppt.reality.realtor

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Realtor models for Reality Portal mobile clients.
 *
 * Mirrors `RealtorProfile` from reality-server (`backend/crates/db/src/models/reality_portal.rs`).
 * Many backend fields are optional; the client uses sensible defaults so UI rows can render even
 * when individual realtors haven't filled out their full profile.
 */
@Serializable
data class RealtorProfile(
    val id: String,
    @SerialName("user_id") val userId: String,
    val name: String? = null,
    val email: String? = null,
    @SerialName("photo_url") val photoUrl: String? = null,
    val bio: String? = null,
    val tagline: String? = null,
    val specializations: List<String>? = null,
    @SerialName("experience_years") val experienceYears: Int? = null,
    val languages: List<String>? = null,
    @SerialName("license_number") val licenseNumber: String? = null,
    val certifications: List<String>? = null,
    val phone: String? = null,
    val whatsapp: String? = null,
    @SerialName("email_public") val emailPublic: String? = null,
    @SerialName("linkedin_url") val linkedinUrl: String? = null,
    @SerialName("agency_id") val agencyId: String? = null,
    @SerialName("created_at") val createdAt: String? = null,
)

/** Wraps a single profile for `GET /api/v1/realtors/profile` and `/{user_id}/profile`. */
@Serializable data class RealtorProfileResponse(val profile: RealtorProfile)

/** Realtor directory response. */
@Serializable data class RealtorListResponse(val realtors: List<RealtorProfile>, val total: Int = 0)
