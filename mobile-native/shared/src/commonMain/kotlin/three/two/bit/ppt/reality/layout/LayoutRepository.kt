package three.two.bit.ppt.reality.layout

import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.http.*
import kotlinx.coroutines.CancellationException
import three.two.bit.ppt.reality.api.ApiConfig
import three.two.bit.ppt.reality.api.HttpClientProvider

/**
 * Repository for layout operations.
 *
 * Fetches resolved layout configurations from the server. Never throws: any failure (network,
 * non-2xx, malformed body, screen-echo mismatch) returns [DEFAULT_LISTING_DETAIL_LAYOUT].
 *
 * The endpoint path `layout/resolved/reality/listing-detail` is built with literal `/` separators
 * so that the axum `{*screen}` catch-all route on the server matches correctly. Percent-encoding
 * the `/` would produce `reality%2Flisting-detail`, which does NOT match the wildcard route.
 */
class LayoutRepository(
    private val baseUrl: String = ApiConfig.baseUrl,
    private val client: HttpClient = HttpClientProvider.client,
) {

    /**
     * Fetch the resolved listing-detail layout for mobile.
     *
     * GET `$baseUrl/api/v1/layout/resolved/reality/listing-detail?platform=mobile`
     *
     * On any failure (non-2xx, network error, JSON decode error, or server echoes a different
     * screen identifier) returns [DEFAULT_LISTING_DETAIL_LAYOUT]. An empty sections list from a 200
     * response is accepted as-is. [CancellationException] is NOT swallowed — coroutine cancellation
     * always propagates.
     *
     * Fetched sections are sanitized: elements whose `type` duplicates an earlier element's type
     * are dropped (first occurrence kept). This protects consumers that key UI elements by section
     * type (e.g. Android's LazyColumn static keys) from server-sent duplicates.
     */
    suspend fun getListingDetailLayout(): ResolvedLayoutScreen {
        return try {
            val response =
                client.get("$baseUrl/api/v1/layout/resolved/reality/listing-detail") {
                    parameter("platform", "mobile")
                }

            if (!response.status.isSuccess()) {
                println(
                    "LayoutRepository: falling back to default layout: HTTP ${response.status.value}"
                )
                return DEFAULT_LISTING_DETAIL_LAYOUT
            }

            val layout = response.body<ResolvedLayoutScreen>()

            if (layout.screen != DEFAULT_LISTING_DETAIL_LAYOUT.screen) {
                println(
                    "LayoutRepository: falling back to default layout: " +
                        "unexpected screen echo '${layout.screen}'"
                )
                DEFAULT_LISTING_DETAIL_LAYOUT
            } else {
                layout.copy(sections = layout.sections.distinctBy { it.type })
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            println("LayoutRepository: falling back to default layout: ${e.message}")
            DEFAULT_LISTING_DETAIL_LAYOUT
        }
    }
}
