package three.two.bit.ppt.reality.inquiry

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.request.HttpRequestData
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpMethod
import io.ktor.http.HttpStatusCode
import io.ktor.http.content.OutgoingContent
import io.ktor.http.headersOf
import io.ktor.serialization.kotlinx.json.json
import io.ktor.utils.io.ByteReadChannel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/**
 * Pins [InquiryRepository] to reality-server's inquiry contract (`routes/inquiries.rs`): buyer list
 * is `/inquiries/mine`, contact is `POST /inquiries/contact/{listing_id}` (name + email required),
 * viewings are `POST /inquiries/viewing/{listing_id}` and come back as inquiries with `inquiry_type
 * = 'viewing'`, and server statuses (new/read/responded/scheduled/closed) map onto the app's enums
 * instead of failing to decode.
 */
class InquiryRepositoryServerContractTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private class Captured {
        var method: HttpMethod? = null
        var path: String? = null
        var body: String? = null
        var calls = 0
    }

    private fun bodyText(request: HttpRequestData): String? =
        (request.body as? OutgoingContent.ByteArrayContent)?.bytes()?.decodeToString()

    private fun repo(cap: Captured, status: HttpStatusCode, body: String): InquiryRepository {
        val engine = MockEngine { request ->
            cap.calls++
            cap.method = request.method
            cap.path = request.url.encodedPath
            cap.body = bodyText(request)
            respond(
                content = ByteReadChannel(body),
                status = status,
                headers = headersOf(HttpHeaders.ContentType, "application/json"),
            )
        }
        val client = HttpClient(engine) { install(ContentNegotiation) { json(json) } }
        return InquiryRepository(baseUrl = "https://example.test", "tok", client)
    }

    private fun inquiry(id: String, type: String, status: String) =
        """
        {"id":"$id","listing_id":"lst-1","realtor_id":"r-1","user_id":null,"name":"Jana",
         "email":"jana@example.test","phone":null,"message":"Hello","inquiry_type":"$type",
         "preferred_contact":"email","preferred_time":null,"status":"$status","read_at":null,
         "responded_at":null,"source":null,"utm_source":null,"utm_medium":null,
         "created_at":"2026-04-26T10:00:00Z"}
        """

    @Test
    fun getInquiries_reads_mine_and_maps_server_statuses() = runTest {
        val cap = Captured()
        val body =
            """{"inquiries":[${inquiry("i-1", "info", "new")},${inquiry("i-2", "info", "responded")}],"total":2,"page":1,"limit":20}"""

        val result = assertNotNull(repo(cap, HttpStatusCode.OK, body).getInquiries().getOrNull())

        assertEquals("/api/v1/inquiries/mine", cap.path)
        assertEquals(
            listOf(InquiryStatus.PENDING, InquiryStatus.RESPONDED),
            result.inquiries.map { it.status },
        )
        assertEquals("", result.inquiries.first().userId)
        assertEquals(2, result.total)
    }

    @Test
    fun createInquiry_posts_contact_with_name_and_email() = runTest {
        val cap = Captured()
        val created = """{"message":"sent","inquiry_id":"i-9"}"""

        val result =
            repo(cap, HttpStatusCode.Created, created)
                .createInquiry(
                    CreateInquiryRequest(
                        listingId = "lst-1",
                        message = "Is it available?",
                        name = "Jana",
                        email = "jana@example.test",
                    ),
                )

        assertEquals("i-9", result.getOrNull()?.id)
        assertEquals(HttpMethod.Post, cap.method)
        assertEquals("/api/v1/inquiries/contact/lst-1", cap.path)
        val sent = json.parseToJsonElement(assertNotNull(cap.body)).jsonObject
        assertEquals("Jana", sent["name"]?.jsonPrimitive?.content)
        assertEquals("jana@example.test", sent["email"]?.jsonPrimitive?.content)
        assertEquals("Is it available?", sent["message"]?.jsonPrimitive?.content)
    }

    @Test
    fun createInquiry_without_contact_details_fails_before_network() = runTest {
        val cap = Captured()

        val result =
            repo(cap, HttpStatusCode.OK, "{}")
                .createInquiry(CreateInquiryRequest(listingId = "lst-1", message = "hi"))

        assertTrue(result.isFailure)
        assertEquals(0, cap.calls)
    }

    @Test
    fun getViewings_keeps_only_viewing_inquiries() = runTest {
        val cap = Captured()
        val body =
            """{"inquiries":[${inquiry("i-1", "info", "new")},${inquiry("v-1", "viewing", "scheduled")}],"total":2,"page":1,"limit":100}"""

        val result = assertNotNull(repo(cap, HttpStatusCode.OK, body).getViewings().getOrNull())

        assertEquals("/api/v1/inquiries/mine", cap.path)
        val viewing = result.viewings.single()
        assertEquals("v-1", viewing.id)
        assertEquals(ViewingStatus.CONFIRMED, viewing.status)
    }

    @Test
    fun scheduleViewing_posts_preferred_times() = runTest {
        val cap = Captured()

        val result =
            repo(cap, HttpStatusCode.Created, """{"message":"ok","inquiry_id":"v-2"}""")
                .scheduleViewing(
                    ScheduleViewingRequest(
                        listingId = "lst-1",
                        preferredDate = "2026-05-01",
                        preferredTime = "10:00",
                        name = "Jana",
                        email = "jana@example.test",
                    ),
                )

        assertEquals("v-2", result.getOrNull()?.id)
        assertEquals("/api/v1/inquiries/viewing/lst-1", cap.path)
        val sent = json.parseToJsonElement(assertNotNull(cap.body)).jsonObject
        assertEquals("2026-05-01 10:00", sent["preferred_times"].toString().trim('[', ']', '"'))
    }

    @Test
    fun getInquiry_maps_detail_thread_replies() = runTest {
        val cap = Captured()
        val body =
            """{"inquiry":${inquiry("i-1", "info", "responded")},"messages":[
               {"id":"m-1","sender_type":"user","message":"Hello","created_at":"2026-04-26T10:00:00Z"},
               {"id":"m-2","sender_type":"realtor","message":"Yes","created_at":"2026-04-26T11:00:00Z"}]}"""

        val result = assertNotNull(repo(cap, HttpStatusCode.OK, body).getInquiry("i-1").getOrNull())

        assertEquals(listOf("Yes"), result.responses.map { it.message })
        assertNull(result.listing)
    }
}
