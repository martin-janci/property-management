package three.two.bit.ppt.reality.agency

import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.http.*
import three.two.bit.ppt.reality.api.HttpClientProvider
import three.two.bit.ppt.reality.api.asPathSegment

/**
 * Repository for the agency directory (UC-51).
 *
 * Wraps the reality-server's `/api/v1/agencies` surface so the mobile UIs (Android Compose
 * `AgenciesView`, iOS SwiftUI `AgenciesView`) can read agency metadata, branding, and membership
 * through a single coroutine API instead of duplicating Ktor calls per platform.
 */
class AgencyRepository(
    private val baseUrl: String,
    private val sessionToken: String? = null,
    private val client: HttpClient = HttpClientProvider.client,
) {

    private fun HttpRequestBuilder.configureRequest() {
        sessionToken?.let { header(HttpHeaders.Authorization, "Bearer $it") }
    }

    /** List public agencies. */
    suspend fun listAgencies(): Result<AgencyListResponse> {
        return try {
            val response = client.get("$baseUrl/api/v1/agencies") { configureRequest() }
            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else {
                Result.failure(AgencyException("Failed to load agencies: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Get a single agency by id. */
    suspend fun getAgency(agencyId: String): Result<Agency> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/agencies/${agencyId.asPathSegment()}") {
                    configureRequest()
                }
            if (response.status.isSuccess()) {
                val payload: AgencyResponse = response.body()
                Result.success(payload.agency)
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(AgencyException("Agency not found"))
            } else {
                Result.failure(AgencyException("Failed to load agency: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Get an agency by its URL slug (used by deep-link handlers). */
    suspend fun getAgencyBySlug(slug: String): Result<Agency> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/agencies/by-slug/${slug.asPathSegment()}") {
                    configureRequest()
                }
            if (response.status.isSuccess()) {
                val payload: AgencyResponse = response.body()
                Result.success(payload.agency)
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(AgencyException("Agency not found"))
            } else {
                Result.failure(AgencyException("Failed to load agency: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** List members (realtors) of an agency. */
    suspend fun listMembers(agencyId: String): Result<AgencyMembersResponse> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/agencies/${agencyId.asPathSegment()}/members") {
                    configureRequest()
                }
            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(AgencyException("Please sign in to view agency members"))
            } else {
                Result.failure(AgencyException("Failed to load members: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}

/** Agency-specific exception. */
class AgencyException(message: String) : Exception(message)
