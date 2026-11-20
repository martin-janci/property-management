package three.two.bit.ppt.reality.inquiry

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Wire-level DTOs for reality-server inquiry endpoints
 * (`backend/servers/reality-server/src/routes/inquiries.rs`, `realtors.rs`). [InquiryRepository]
 * decodes these and maps them onto the app's domain models.
 */

/** `ListingInquiry` row (db model `reality_portal::ListingInquiry`). */
@Serializable
internal data class WireListingInquiry(
    val id: String,
    @SerialName("listing_id") val listingId: String,
    @SerialName("realtor_id") val realtorId: String? = null,
    @SerialName("user_id") val userId: String? = null,
    val name: String = "",
    val email: String = "",
    val phone: String? = null,
    val message: String = "",
    @SerialName("inquiry_type") val inquiryType: String = "info",
    @SerialName("preferred_contact") val preferredContact: String? = null,
    @SerialName("preferred_time") val preferredTime: String? = null,
    val status: String = "new",
    @SerialName("read_at") val readAt: String? = null,
    @SerialName("responded_at") val respondedAt: String? = null,
    @SerialName("created_at") val createdAt: String = "",
)

/** `GET /api/v1/inquiries/mine` → `InquiryListResponse`. */
@Serializable
internal data class WireInquiryListResponse(
    val inquiries: List<WireListingInquiry> = emptyList(),
    val total: Long = 0,
    val page: Int = 1,
    val limit: Int = 20,
)

/** `GET /api/v1/realtors/inquiries` → `InquiriesResponse` (no paging echo). */
@Serializable
internal data class WireRealtorInquiriesResponse(
    val inquiries: List<WireListingInquiry> = emptyList(),
    val total: Long = 0,
)

/** One message of an inquiry thread. `sender_type` is `user` or `realtor`. */
@Serializable
internal data class WireInquiryMessage(
    val id: String,
    @SerialName("sender_type") val senderType: String,
    val message: String,
    @SerialName("created_at") val createdAt: String,
)

/** `GET /api/v1/inquiries/{id}` → `InquiryDetailResponse`. */
@Serializable
internal data class WireInquiryDetailResponse(
    val inquiry: WireListingInquiry,
    val messages: List<WireInquiryMessage> = emptyList(),
)

/** `POST /api/v1/inquiries/contact/{listing_id}` body. */
@Serializable
internal data class WireContactMessageRequest(
    val name: String,
    val email: String,
    val phone: String? = null,
    val message: String,
    @SerialName("preferred_contact") val preferredContact: String? = null,
)

/** `POST /api/v1/inquiries/viewing/{listing_id}` body. */
@Serializable
internal data class WireViewingRequest(
    val name: String,
    val email: String,
    val phone: String? = null,
    @SerialName("preferred_times") val preferredTimes: List<String>? = null,
    val message: String? = null,
)

/** Response of both contact and viewing submissions. */
@Serializable
internal data class WireInquiryCreated(
    val message: String = "",
    @SerialName("inquiry_id") val inquiryId: String,
)

/** `POST /api/v1/inquiries/{id}/respond` body (db `SendInquiryMessage`). */
@Serializable internal data class WireSendInquiryMessage(val message: String)

/** Server status values: new, read, responded, scheduled, closed. */
internal fun inquiryStatusToDomain(raw: String): InquiryStatus =
    when (raw.lowercase()) {
        "responded" -> InquiryStatus.RESPONDED
        "closed" -> InquiryStatus.CLOSED
        else -> InquiryStatus.PENDING
    }

internal fun viewingStatusToDomain(raw: String): ViewingStatus =
    when (raw.lowercase()) {
        "scheduled" -> ViewingStatus.CONFIRMED
        "closed" -> ViewingStatus.COMPLETED
        else -> ViewingStatus.PENDING
    }

internal fun WireListingInquiry.toDomain(
    messages: List<WireInquiryMessage> = emptyList(),
): Inquiry =
    Inquiry(
        id = id,
        listingId = listingId,
        userId = userId.orEmpty(),
        message = message,
        status = inquiryStatusToDomain(status),
        createdAt = createdAt,
        updatedAt = respondedAt ?: readAt ?: createdAt,
        responses =
            messages
                .filter { it.senderType == "realtor" }
                .map {
                    InquiryResponse(
                        id = it.id,
                        inquiryId = id,
                        realtorId = realtorId.orEmpty(),
                        message = it.message,
                        createdAt = it.createdAt,
                    )
                },
    )

/**
 * A viewing request is an inquiry with `inquiry_type = 'viewing'`; the requested slot is the
 * free-text `preferred_time` (the server stores the preferred times joined into one string).
 */
internal fun WireListingInquiry.toViewing(): ViewingRequest =
    ViewingRequest(
        id = id,
        listingId = listingId,
        userId = userId.orEmpty(),
        preferredDate = preferredTime.orEmpty(),
        preferredTime = "",
        message = message.takeIf { it.isNotBlank() },
        status = viewingStatusToDomain(status),
        createdAt = createdAt,
    )
