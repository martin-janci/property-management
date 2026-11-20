package three.two.bit.ppt.reality.inquiry

import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.http.*
import three.two.bit.ppt.reality.api.HttpClientProvider
import three.two.bit.ppt.reality.api.asPathSegment

/**
 * Repository for inquiries and viewing requests.
 *
 * Epic 48 - Story 48.6: Portal Mobile Inquiries
 */
class InquiryRepository(
    private val baseUrl: String,
    private val sessionToken: String? = null,
    private val client: HttpClient = HttpClientProvider.client,
) {

    private fun HttpRequestBuilder.configureRequest() {
        sessionToken?.let { header(HttpHeaders.Authorization, "Bearer $it") }
    }

    // --- Inquiries ---

    /**
     * Get the signed-in buyer's own inquiries (`GET /api/v1/inquiries/mine`).
     *
     * The server filters by its own status vocabulary (new/read/responded/scheduled/closed);
     * [InquiryStatus.PENDING] covers several of those, so it is filtered client-side.
     */
    suspend fun getInquiries(
        page: Int = 1,
        pageSize: Int = 20,
        status: InquiryStatus? = null,
    ): Result<InquiriesResponse> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/inquiries/mine") {
                    configureRequest()
                    parameter("page", page)
                    parameter("limit", pageSize)
                    status?.serverFilter()?.let { parameter("status", it) }
                }

            if (response.status.isSuccess()) {
                val wire = response.body<WireInquiryListResponse>()
                val inquiries =
                    wire.inquiries
                        .map { it.toDomain() }
                        .filter { status == null || it.status == status }
                Result.success(
                    InquiriesResponse(
                        inquiries = inquiries,
                        total = wire.total.toIntClamped(),
                        page = wire.page,
                        pageSize = wire.limit,
                    ),
                )
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(InquiryException("Please sign in to view inquiries"))
            } else {
                Result.failure(InquiryException("Failed to load inquiries: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Get inquiries addressed to the signed-in realtor / agency. Backed by `GET
     * /api/v1/realtors/inquiries`, which is the realtor-side inbox — distinct from
     * `getInquiries()`, which returns the buyer's own outgoing inquiries.
     */
    suspend fun getRealtorInquiries(
        limit: Int = 20,
        offset: Int = 0,
        status: InquiryStatus? = null,
    ): Result<InquiriesResponse> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/realtors/inquiries") {
                    configureRequest()
                    parameter("limit", limit)
                    parameter("offset", offset)
                    status?.serverFilter()?.let { parameter("status", it) }
                }

            if (response.status.isSuccess()) {
                val wire = response.body<WireRealtorInquiriesResponse>()
                val inquiries =
                    wire.inquiries
                        .map { it.toDomain() }
                        .filter { status == null || it.status == status }
                Result.success(
                    InquiriesResponse(
                        inquiries = inquiries,
                        total = wire.total.toIntClamped(),
                        page = if (limit > 0) offset / limit + 1 else 1,
                        pageSize = limit,
                    ),
                )
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(InquiryException("Please sign in to view inquiries"))
            } else {
                Result.failure(InquiryException("Failed to load inquiries: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Get one of the signed-in buyer's inquiries with its message thread (`GET
     * /api/v1/inquiries/mine/{id}`). Another buyer's inquiry is a 404.
     */
    suspend fun getInquiry(inquiryId: String): Result<Inquiry> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/inquiries/mine/${inquiryId.asPathSegment()}") {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                val wire = response.body<WireInquiryDetailResponse>()
                Result.success(wire.inquiry.toDomain(wire.messages))
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(InquiryException("Inquiry not found"))
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(InquiryException("Please sign in to view inquiries"))
            } else {
                Result.failure(InquiryException("Failed to load inquiry: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Send a contact message about a listing (`POST /api/v1/inquiries/contact/{listing_id}`). The
     * server requires a name and an email, so a request without them fails before any network call.
     */
    suspend fun createInquiry(request: CreateInquiryRequest): Result<CreateInquiryResponse> {
        val name = request.name?.trim().orEmpty()
        val email = request.email?.trim().orEmpty()
        if (name.isEmpty() || email.isEmpty()) {
            return Result.failure(InquiryException("Name and email are required"))
        }
        return try {
            val response =
                client.post(
                    "$baseUrl/api/v1/inquiries/contact/${request.listingId.asPathSegment()}",
                ) {
                    configureRequest()
                    contentType(ContentType.Application.Json)
                    setBody(
                        WireContactMessageRequest(
                            name = name,
                            email = email,
                            phone = request.phone?.trim()?.takeIf { it.isNotEmpty() },
                            message = request.message,
                        ),
                    )
                }

            if (response.status.isSuccess()) {
                val created = response.body<WireInquiryCreated>()
                Result.success(
                    CreateInquiryResponse(
                        id = created.inquiryId,
                        listingId = request.listingId,
                        status = InquiryStatus.PENDING,
                        createdAt = "",
                    ),
                )
            } else if (response.status == HttpStatusCode.TooManyRequests) {
                Result.failure(InquiryException("Too many requests. Please try again in a minute."))
            } else {
                Result.failure(InquiryException("Failed to send inquiry: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Send a buyer follow-up on one of the signed-in buyer's inquiries (`POST
     * /api/v1/inquiries/mine/{id}/messages`). A closed inquiry no longer accepts replies (404).
     */
    suspend fun replyToInquiry(inquiryId: String, message: String): Result<InquiryResponse> {
        return try {
            val response =
                client.post(
                    "$baseUrl/api/v1/inquiries/mine/${inquiryId.asPathSegment()}/messages",
                ) {
                    configureRequest()
                    contentType(ContentType.Application.Json)
                    setBody(WireSendInquiryMessage(message))
                }

            if (response.status.isSuccess()) {
                val wire = response.body<WireInquiryMessage>()
                Result.success(
                    InquiryResponse(
                        id = wire.id,
                        inquiryId = inquiryId,
                        realtorId = "",
                        message = wire.message,
                        createdAt = wire.createdAt,
                    ),
                )
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(InquiryException("Please sign in to reply"))
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(InquiryException("This inquiry is closed or no longer available"))
            } else {
                Result.failure(InquiryException("Failed to send reply: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    // --- Viewing Requests ---

    /**
     * Get the buyer's viewing requests. The server stores them as inquiries with `inquiry_type =
     * 'viewing'`, so this reads `/inquiries/mine` and keeps only those.
     */
    suspend fun getViewings(status: ViewingStatus? = null): Result<ViewingsResponse> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/inquiries/mine") {
                    configureRequest()
                    parameter("limit", 100)
                }

            if (response.status.isSuccess()) {
                val viewings =
                    response
                        .body<WireInquiryListResponse>()
                        .inquiries
                        .filter { it.inquiryType == "viewing" }
                        .map { it.toViewing() }
                        .filter { status == null || it.status == status }
                Result.success(ViewingsResponse(viewings = viewings, total = viewings.size))
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(InquiryException("Please sign in to view viewing requests"))
            } else {
                Result.failure(InquiryException("Failed to load viewings: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Request a viewing (`POST /api/v1/inquiries/viewing/{listing_id}`). The requested and the
     * alternative slot are sent as free-text preferred times; the realtor confirms out of band.
     */
    suspend fun scheduleViewing(request: ScheduleViewingRequest): Result<ViewingRequest> {
        val name = request.name?.trim().orEmpty()
        val email = request.email?.trim().orEmpty()
        if (name.isEmpty() || email.isEmpty()) {
            return Result.failure(InquiryException("Name and email are required"))
        }
        val times =
            listOfNotNull(
                    "${request.preferredDate} ${request.preferredTime}".trim(),
                    request.alternativeDate?.let {
                        "$it ${request.alternativeTime.orEmpty()}".trim()
                    },
                )
                .filter { it.isNotEmpty() }
        return try {
            val response =
                client.post(
                    "$baseUrl/api/v1/inquiries/viewing/${request.listingId.asPathSegment()}",
                ) {
                    configureRequest()
                    contentType(ContentType.Application.Json)
                    setBody(
                        WireViewingRequest(
                            name = name,
                            email = email,
                            phone = request.phone?.trim()?.takeIf { it.isNotEmpty() },
                            preferredTimes = times.ifEmpty { null },
                            message = request.message,
                        ),
                    )
                }

            if (response.status.isSuccess()) {
                val created = response.body<WireInquiryCreated>()
                Result.success(
                    ViewingRequest(
                        id = created.inquiryId,
                        listingId = request.listingId,
                        userId = "",
                        preferredDate = request.preferredDate,
                        preferredTime = request.preferredTime,
                        alternativeDate = request.alternativeDate,
                        alternativeTime = request.alternativeTime,
                        message = request.message,
                        status = ViewingStatus.PENDING,
                        createdAt = "",
                    ),
                )
            } else if (response.status == HttpStatusCode.TooManyRequests) {
                Result.failure(InquiryException("Too many requests. Please try again in a minute."))
            } else {
                Result.failure(InquiryException("Failed to schedule viewing: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Cancel one of the buyer's viewing requests (`POST /api/v1/inquiries/mine/{id}/cancel`). */
    suspend fun cancelViewing(viewingId: String): Result<Unit> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/inquiries/mine/${viewingId.asPathSegment()}/cancel") {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(InquiryException("Please sign in to manage viewings"))
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(InquiryException("Viewing request not found"))
            } else {
                Result.failure(InquiryException("Failed to cancel viewing: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    private fun InquiryStatus.serverFilter(): String? =
        when (this) {
            InquiryStatus.PENDING -> null
            InquiryStatus.RESPONDED -> "responded"
            InquiryStatus.CLOSED -> "closed"
        }

    private fun Long.toIntClamped(): Int = coerceAtMost(Int.MAX_VALUE.toLong()).toInt()
}

/** The server does not offer this operation yet. UI can check for this type and hide the action. */
class UnsupportedFeatureException(message: String) : Exception(message)

/** Inquiry-specific exception. */
class InquiryException(message: String) : Exception(message)
