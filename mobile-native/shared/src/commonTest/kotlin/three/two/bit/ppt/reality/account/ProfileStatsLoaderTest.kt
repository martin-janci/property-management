package three.two.bit.ppt.reality.account

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.client.engine.mock.respondError
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import io.ktor.http.headersOf
import io.ktor.serialization.kotlinx.json.json
import io.ktor.utils.io.ByteReadChannel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json
import three.two.bit.ppt.reality.favorites.FavoritesRepository
import three.two.bit.ppt.reality.inquiry.InquiryRepository

/**
 * Unit tests for [ProfileStatsLoader] — the Account-screen stat-strip aggregator.
 *
 * Lives in `commonMain` / `commonTest` so the derivation logic (read `total` off each list endpoint,
 * fall back to `null` on failure) is verifiable on the JVM without an Android/Gradle device build.
 *
 * Epic 48 - Story 48.5 (gap-82-5): wire Account stats to reality-server list endpoints.
 */
class ProfileStatsLoaderTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    /**
     * Routes each request to a canned response keyed by the path's last meaningful segment, so a
     * single MockEngine can back both repositories the loader fans out to.
     */
    private fun loaderWith(
        favoritesStatus: HttpStatusCode = HttpStatusCode.OK,
        favoritesBody: String = """{"favorites":[],"total":0}""",
        savedSearchesStatus: HttpStatusCode = HttpStatusCode.OK,
        savedSearchesBody: String = """{"searches":[],"total":0}""",
        inquiriesStatus: HttpStatusCode = HttpStatusCode.OK,
        inquiriesBody: String = """{"inquiries":[],"total":0,"page":1,"page_size":20}""",
    ): ProfileStatsLoader {
        val engine = MockEngine { request ->
            val path = request.url.encodedPath
            when {
                path.contains("/saved-searches") ->
                    if (savedSearchesStatus.isSuccess()) {
                        respond(
                            content = ByteReadChannel(savedSearchesBody),
                            status = savedSearchesStatus,
                            headers =
                                headersOf(HttpHeaders.ContentType, "application/json"),
                        )
                    } else {
                        respondError(savedSearchesStatus)
                    }
                path.contains("/favorites") ->
                    if (favoritesStatus.isSuccess()) {
                        respond(
                            content = ByteReadChannel(favoritesBody),
                            status = favoritesStatus,
                            headers =
                                headersOf(HttpHeaders.ContentType, "application/json"),
                        )
                    } else {
                        respondError(favoritesStatus)
                    }
                path.contains("/inquiries") ->
                    if (inquiriesStatus.isSuccess()) {
                        respond(
                            content = ByteReadChannel(inquiriesBody),
                            status = inquiriesStatus,
                            headers =
                                headersOf(HttpHeaders.ContentType, "application/json"),
                        )
                    } else {
                        respondError(inquiriesStatus)
                    }
                else -> respondError(HttpStatusCode.NotFound)
            }
        }
        val client = HttpClient(engine) { install(ContentNegotiation) { json(json) } }
        return ProfileStatsLoader(
            favoritesRepository =
                FavoritesRepository("https://example.test", "tok", client),
            inquiryRepository = InquiryRepository("https://example.test", "tok", client),
        )
    }

    @Test
    fun load_reads_total_from_each_endpoint() = runTest {
        val loader =
            loaderWith(
                favoritesBody = """{"favorites":[],"total":7}""",
                savedSearchesBody = """{"searches":[],"total":3}""",
                inquiriesBody = """{"inquiries":[],"total":5,"page":1,"page_size":20}""",
            )
        val stats = loader.load()
        assertEquals(7, stats.favorites)
        assertEquals(3, stats.searches)
        assertEquals(5, stats.inquiries)
    }

    @Test
    fun load_falls_back_to_null_per_failing_endpoint() = runTest {
        val loader =
            loaderWith(
                favoritesStatus = HttpStatusCode.InternalServerError,
                savedSearchesBody = """{"searches":[],"total":2}""",
                inquiriesStatus = HttpStatusCode.Unauthorized,
            )
        val stats = loader.load()
        assertNull(stats.favorites, "favorites should be null when its call fails")
        assertEquals(2, stats.searches, "searches should still resolve independently")
        assertNull(stats.inquiries, "inquiries should be null when its call fails")
    }

    @Test
    fun load_clamps_oversized_favorites_total_to_int_max() = runTest {
        val loader =
            loaderWith(favoritesBody = """{"favorites":[],"total":3000000000}""")
        val stats = loader.load()
        assertEquals(Int.MAX_VALUE, stats.favorites)
    }
}
