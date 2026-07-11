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

    /**
     * Convert InquiryStatus enum to API query parameter value. Uses explicit mapping to
     * match @SerialName annotations and avoid fragile .name.lowercase().
     */
    private fun InquiryStatus.toQueryParam(): String =
        when (this) {
            InquiryStatus.PENDING -> "pending"
            InquiryStatus.RESPONDED -> "responded"
            InquiryStatus.CLOSED -> "closed"
        }

    /**
     * Convert ViewingStatus enum to API query parameter value. Uses explicit mapping to
     * match @SerialName annotations and avoid fragile .name.lowercase().
     */
    private fun ViewingStatus.toQueryParam(): String =
        when (this) {
            ViewingStatus.PENDING -> "pending"
            ViewingStatus.CONFIRMED -> "confirmed"
            ViewingStatus.COMPLETED -> "completed"
            ViewingStatus.CANCELLED -> "cancelled"
        }

    // --- Inquiries ---

    /** Get user's inquiries. */
    suspend fun getInquiries(
        page: Int = 1,
        pageSize: Int = 20,
        status: InquiryStatus? = null,
    ): Result<InquiriesResponse> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/inquiries") {
                    configureRequest()
                    parameter("page", page)
                    parameter("page_size", pageSize)
                    status?.let { parameter("status", it.toQueryParam()) }
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
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
     * `getInquiries()`, which returns the resident's own outgoing inquiries.
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
                    status?.let { parameter("status", it.toQueryParam()) }
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(InquiryException("Please sign in to view inquiries"))
            } else {
                Result.failure(InquiryException("Failed to load inquiries: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Get inquiry by ID. */
    suspend fun getInquiry(inquiryId: String): Result<Inquiry> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/inquiries/${inquiryId.asPathSegment()}") {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
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

    /** Create a new inquiry for a listing. */
    suspend fun createInquiry(request: CreateInquiryRequest): Result<CreateInquiryResponse> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/inquiries") {
                    configureRequest()
                    setBody(request)
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else {
                Result.failure(InquiryException("Failed to send inquiry: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Reply to an inquiry (for authenticated users). */
    suspend fun replyToInquiry(inquiryId: String, message: String): Result<InquiryResponse> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/inquiries/${inquiryId.asPathSegment()}/replies") {
                    configureRequest()
                    setBody(ReplyToInquiryRequest(message))
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(InquiryException("Please sign in to reply"))
            } else {
                Result.failure(InquiryException("Failed to send reply: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    // --- Viewing Requests ---

    /** Get user's viewing requests. */
    suspend fun getViewings(status: ViewingStatus? = null): Result<ViewingsResponse> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/viewings") {
                    configureRequest()
                    status?.let { parameter("status", it.toQueryParam()) }
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(InquiryException("Please sign in to view viewing requests"))
            } else {
                Result.failure(InquiryException("Failed to load viewings: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Schedule a viewing for a listing. */
    suspend fun scheduleViewing(request: ScheduleViewingRequest): Result<ViewingRequest> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/viewings") {
                    configureRequest()
                    setBody(request)
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else {
                Result.failure(InquiryException("Failed to schedule viewing: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Cancel a viewing request. */
    suspend fun cancelViewing(viewingId: String): Result<Unit> {
        return try {
            val response =
                client.delete("$baseUrl/api/v1/viewings/${viewingId.asPathSegment()}") {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(InquiryException("Please sign in to cancel viewing"))
            } else {
                Result.failure(InquiryException("Failed to cancel viewing: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}

/** Inquiry-specific exception. */
class InquiryException(message: String) : Exception(message)
