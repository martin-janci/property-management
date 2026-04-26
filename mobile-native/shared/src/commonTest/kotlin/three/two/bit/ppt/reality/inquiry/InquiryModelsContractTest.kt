package three.two.bit.ppt.reality.inquiry

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Contract tests for the inquiry + viewing-request DTOs.
 *
 * Pins the JSON shape that the reality-server returns for the
 * /api/v1/inquiries and /api/v1/viewings endpoints.
 */
class InquiryModelsContractTest {

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = false
    }

    // -------------------------------------------------------------------
    // InquiryStatus / ViewingStatus enum SerialNames
    // -------------------------------------------------------------------

    @Test
    fun inquiryStatus_uses_lowercase_serial_names() {
        assertEquals("\"pending\"", json.encodeToString(InquiryStatus.PENDING))
        assertEquals("\"responded\"", json.encodeToString(InquiryStatus.RESPONDED))
        assertEquals("\"closed\"", json.encodeToString(InquiryStatus.CLOSED))
    }

    @Test
    fun viewingStatus_uses_lowercase_serial_names() {
        assertEquals("\"pending\"", json.encodeToString(ViewingStatus.PENDING))
        assertEquals("\"confirmed\"", json.encodeToString(ViewingStatus.CONFIRMED))
        assertEquals("\"completed\"", json.encodeToString(ViewingStatus.COMPLETED))
        assertEquals("\"cancelled\"", json.encodeToString(ViewingStatus.CANCELLED))
    }

    @Test
    fun inquiryStatus_round_trips() {
        for (s in InquiryStatus.entries) {
            val decoded: InquiryStatus = json.decodeFromString(json.encodeToString(s))
            assertEquals(s, decoded)
        }
    }

    // -------------------------------------------------------------------
    // CreateInquiryRequest / Response
    // -------------------------------------------------------------------

    @Test
    fun createInquiryRequest_encodes_listing_id_in_snake_case() {
        val req =
            CreateInquiryRequest(
                listingId = "lst-1",
                message = "Is this available?",
                name = "Jane",
                email = "jane@example.com"
            )
        val encoded = json.encodeToString(req)
        assertTrue(encoded.contains("\"listing_id\":\"lst-1\""), "missing snake_case: $encoded")
        assertTrue(encoded.contains("\"message\":\"Is this available?\""))
        assertTrue(encoded.contains("\"email\":\"jane@example.com\""))
        // phone is null and should be omitted.
        assertTrue(!encoded.contains("\"phone\""), "phone should be omitted: $encoded")
    }

    @Test
    fun createInquiryResponse_decodes_pending_status() {
        val raw =
            """
            {
              "id": "inq-1",
              "listing_id": "lst-1",
              "status": "pending",
              "created_at": "2026-04-26T10:00:00Z"
            }
            """
                .trimIndent()
        val resp: CreateInquiryResponse = json.decodeFromString(raw)
        assertEquals("inq-1", resp.id)
        assertEquals("lst-1", resp.listingId)
        assertEquals(InquiryStatus.PENDING, resp.status)
    }

    // -------------------------------------------------------------------
    // Inquiry
    // -------------------------------------------------------------------

    @Test
    fun inquiry_decodes_with_realtor_responses_and_listing() {
        val raw =
            """
            {
              "id": "inq-1",
              "listing_id": "lst-1",
              "user_id": "u-1",
              "message": "Hello",
              "status": "responded",
              "created_at": "2026-04-26T10:00:00Z",
              "updated_at": "2026-04-26T11:00:00Z",
              "responses": [
                {
                  "id": "rsp-1",
                  "inquiry_id": "inq-1",
                  "realtor_id": "r-1",
                  "message": "Yes, still available",
                  "created_at": "2026-04-26T11:00:00Z"
                }
              ]
            }
            """
                .trimIndent()
        val inquiry: Inquiry = json.decodeFromString(raw)
        assertEquals("inq-1", inquiry.id)
        assertEquals(InquiryStatus.RESPONDED, inquiry.status)
        assertEquals(1, inquiry.responses.size)
        val response = inquiry.responses[0]
        assertEquals("inq-1", response.inquiryId)
        assertEquals("r-1", response.realtorId)
        assertNull(response.realtor)
    }

    @Test
    fun inquiry_decodes_with_empty_responses_default() {
        val raw =
            """
            {
              "id": "inq-2",
              "listing_id": "lst-2",
              "user_id": "u-1",
              "message": "Hi",
              "status": "pending",
              "created_at": "2026-04-26T10:00:00Z",
              "updated_at": "2026-04-26T10:00:00Z"
            }
            """
                .trimIndent()
        val inquiry: Inquiry = json.decodeFromString(raw)
        assertTrue(inquiry.responses.isEmpty())
    }

    @Test
    fun inquiriesResponse_maps_pagination() {
        val raw =
            """
            {
              "inquiries": [],
              "total": 0,
              "page": 1,
              "page_size": 20
            }
            """
                .trimIndent()
        val response: InquiriesResponse = json.decodeFromString(raw)
        assertEquals(0, response.total)
        assertEquals(20, response.pageSize)
    }

    // -------------------------------------------------------------------
    // ScheduleViewingRequest
    // -------------------------------------------------------------------

    @Test
    fun scheduleViewingRequest_uses_snake_case_for_dates() {
        val req =
            ScheduleViewingRequest(
                listingId = "lst-1",
                preferredDate = "2026-05-01",
                preferredTime = "14:00",
                alternativeDate = "2026-05-02",
                alternativeTime = "10:00",
                message = "Looking forward"
            )
        val encoded = json.encodeToString(req)
        assertTrue(encoded.contains("\"listing_id\":\"lst-1\""))
        assertTrue(encoded.contains("\"preferred_date\":\"2026-05-01\""))
        assertTrue(encoded.contains("\"preferred_time\":\"14:00\""))
        assertTrue(encoded.contains("\"alternative_date\":\"2026-05-02\""))
        assertTrue(encoded.contains("\"alternative_time\":\"10:00\""))
    }

    // -------------------------------------------------------------------
    // ViewingRequest decoding
    // -------------------------------------------------------------------

    @Test
    fun viewingRequest_decodes_confirmed_with_dates() {
        val raw =
            """
            {
              "id": "v-1",
              "listing_id": "lst-1",
              "user_id": "u-1",
              "preferred_date": "2026-05-01",
              "preferred_time": "14:00",
              "alternative_date": "2026-05-02",
              "alternative_time": "10:00",
              "status": "confirmed",
              "confirmed_date": "2026-05-01",
              "confirmed_time": "14:00",
              "created_at": "2026-04-26T10:00:00Z"
            }
            """
                .trimIndent()
        val viewing: ViewingRequest = json.decodeFromString(raw)
        assertEquals(ViewingStatus.CONFIRMED, viewing.status)
        assertEquals("2026-05-01", viewing.confirmedDate)
        assertEquals("14:00", viewing.confirmedTime)
        assertEquals("2026-05-02", viewing.alternativeDate)
    }

    @Test
    fun viewingRequest_decodes_pending_with_optional_nulls() {
        val raw =
            """
            {
              "id": "v-2",
              "listing_id": "lst-2",
              "user_id": "u-1",
              "preferred_date": "2026-05-10",
              "preferred_time": "09:00",
              "status": "pending",
              "created_at": "2026-04-26T10:00:00Z"
            }
            """
                .trimIndent()
        val viewing: ViewingRequest = json.decodeFromString(raw)
        assertEquals(ViewingStatus.PENDING, viewing.status)
        assertNull(viewing.alternativeDate)
        assertNull(viewing.confirmedDate)
        assertNull(viewing.message)
    }

    @Test
    fun viewingsResponse_decodes_collection_with_total() {
        val raw =
            """
            {
              "viewings": [
                {
                  "id": "v-1",
                  "listing_id": "lst-1",
                  "user_id": "u-1",
                  "preferred_date": "2026-05-01",
                  "preferred_time": "14:00",
                  "status": "pending",
                  "created_at": "2026-04-26T10:00:00Z"
                }
              ],
              "total": 1
            }
            """
                .trimIndent()
        val response: ViewingsResponse = json.decodeFromString(raw)
        assertEquals(1, response.total)
        assertEquals(1, response.viewings.size)
        val viewing = assertNotNull(response.viewings.firstOrNull())
        assertEquals("v-1", viewing.id)
    }

    // -------------------------------------------------------------------
    // ReplyToInquiryRequest
    // -------------------------------------------------------------------

    @Test
    fun replyToInquiryRequest_encodes_message() {
        val request = ReplyToInquiryRequest("Please call me back")
        val encoded = json.encodeToString(request)
        assertEquals("{\"message\":\"Please call me back\"}", encoded)
    }
}
