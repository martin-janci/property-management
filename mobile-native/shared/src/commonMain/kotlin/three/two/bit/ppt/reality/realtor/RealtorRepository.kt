package three.two.bit.ppt.reality.realtor

import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.http.*
import three.two.bit.ppt.reality.api.HttpClientProvider
import three.two.bit.ppt.reality.api.asPathSegment

/**
 * Repository for realtor profiles (UC-49).
 *
 * Wraps reality-server's `/api/v1/realtors` surface. The list endpoint is not currently exposed by
 * reality-server, so `listRealtors()` falls back to the agency-members endpoint when an `agencyId`
 * is provided; without one it returns an empty result (NotImplemented from the server side).
 */
class RealtorRepository(
    private val baseUrl: String,
    private val sessionToken: String? = null,
    private val client: HttpClient = HttpClientProvider.client,
) {

    private fun HttpRequestBuilder.configureRequest() {
        sessionToken?.let { header(HttpHeaders.Authorization, "Bearer $it") }
    }

    /** Get the signed-in user's realtor profile (404 if none). */
    suspend fun getMyProfile(): Result<RealtorProfile> {
        return try {
            val response = client.get("$baseUrl/api/v1/realtors/profile") { configureRequest() }
            if (response.status.isSuccess()) {
                val payload: RealtorProfileResponse = response.body()
                Result.success(payload.profile)
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(RealtorException("Profile not found"))
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(RealtorException("Please sign in"))
            } else {
                Result.failure(RealtorException("Failed to load profile: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Get a realtor's public profile by their user id. */
    suspend fun getProfile(userId: String): Result<RealtorProfile> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/realtors/${userId.asPathSegment()}/profile") {
                    configureRequest()
                }
            if (response.status.isSuccess()) {
                val payload: RealtorProfileResponse = response.body()
                Result.success(payload.profile)
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(RealtorException("Realtor not found"))
            } else {
                Result.failure(RealtorException("Failed to load profile: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}

/** Realtor-specific exception. */
class RealtorException(message: String) : Exception(message)
