package three.two.bit.ppt.reality.inquiry

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import three.two.bit.ppt.reality.listing.ListingSummary
import three.two.bit.ppt.reality.listing.RealtorInfo

/**
 * Inquiry models for Reality Portal mobile app.
 *
 * Epic 48 - Story 48.6: Portal Mobile Inquiries
 */

/** Inquiry status. */
@Serializable
enum class InquiryStatus {
    @SerialName("pending") PENDING,
    @SerialName("responded") RESPONDED,
    @SerialName("closed") CLOSED,
}

/** Inquiry entry. */
@Serializable
data class Inquiry(
    val id: String,
    @SerialName("listing_id") val listingId: String,
    @SerialName("user_id") val userId: String,
    val message: String,
    val status: InquiryStatus,
    @SerialName("created_at") val createdAt: String,
    @SerialName("updated_at") val updatedAt: String,
    val listing: ListingSummary? = null,
    val realtor: RealtorInfo? = null,
    val responses: List<InquiryResponse> = emptyList(),
)

/** Inquiry response from realtor. */
@Serializable
data class InquiryResponse(
    val id: String,
    @SerialName("inquiry_id") val inquiryId: String,
    @SerialName("realtor_id") val realtorId: String,
    val message: String,
    @SerialName("created_at") val createdAt: String,
    val realtor: RealtorInfo? = null,
)

/** User inquiries response. */
@Serializable
data class InquiriesResponse(
    val inquiries: List<Inquiry>,
    val total: Int,
    val page: Int,
    @SerialName("page_size") val pageSize: Int,
)

/** Create inquiry request. */
@Serializable
data class CreateInquiryRequest(
    @SerialName("listing_id") val listingId: String,
    val message: String,
    val name: String? = null,
    val email: String? = null,
    val phone: String? = null,
)

/** Create inquiry response. */
@Serializable
data class CreateInquiryResponse(
    val id: String,
    @SerialName("listing_id") val listingId: String,
    val status: InquiryStatus,
    @SerialName("created_at") val createdAt: String,
)

/** Reply to inquiry request. */
@Serializable data class ReplyToInquiryRequest(val message: String)

/** Schedule viewing request. */
@Serializable
data class ScheduleViewingRequest(
    @SerialName("listing_id") val listingId: String,
    @SerialName("preferred_date") val preferredDate: String,
    @SerialName("preferred_time") val preferredTime: String,
    @SerialName("alternative_date") val alternativeDate: String? = null,
    @SerialName("alternative_time") val alternativeTime: String? = null,
    val message: String? = null,
    val name: String? = null,
    val email: String? = null,
    val phone: String? = null,
)

/** Viewing request. */
@Serializable
data class ViewingRequest(
    val id: String,
    @SerialName("listing_id") val listingId: String,
    @SerialName("user_id") val userId: String,
    @SerialName("preferred_date") val preferredDate: String,
    @SerialName("preferred_time") val preferredTime: String,
    @SerialName("alternative_date") val alternativeDate: String? = null,
    @SerialName("alternative_time") val alternativeTime: String? = null,
    val message: String? = null,
    val status: ViewingStatus,
    @SerialName("confirmed_date") val confirmedDate: String? = null,
    @SerialName("confirmed_time") val confirmedTime: String? = null,
    @SerialName("created_at") val createdAt: String,
    val listing: ListingSummary? = null,
)

/** Viewing status. */
@Serializable
enum class ViewingStatus {
    @SerialName("pending") PENDING,
    @SerialName("confirmed") CONFIRMED,
    @SerialName("completed") COMPLETED,
    @SerialName("cancelled") CANCELLED,
}

/** User viewings response. */
@Serializable data class ViewingsResponse(val viewings: List<ViewingRequest>, val total: Int)
