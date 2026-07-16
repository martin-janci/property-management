package three.two.bit.ppt.reality.api

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json

/**
 * Cross-surface wire-contract guard for reality-server `rust_decimal::Decimal` fields.
 *
 * reality-server builds `rust_decimal` with the `serde-str` feature (`backend/Cargo.toml`), so
 * EVERY `Decimal` field it serializes **directly** from a DB model arrives on the wire as a JSON
 * **string** carrying its full fractional scale — `"200000.00"`, `"72.50"`, `"-7.50"` — never a
 * JSON number. A KMP model that maps such a field to a bare `Long`/`Double` throws a
 * `JsonDecodingException` the moment a fractional value shows up (issue #2331 / PR #2350).
 *
 * PR #2350 fixed the favorites/alerts surface. This test is the **follow-up guard** (issue #2364):
 * it pins the real `serde-str` shape for the *sibling* Decimal-backed surfaces that are not yet
 * consumed by the KMP app, so the day a future model maps them the failure surfaces here at CI time
 * rather than at runtime on a device:
 *
 *  - `PortalListingResponse.price` / `sizeSqm` — `reality-server/src/routes/portal_listings.rs`
 *    (camelCase serde).
 *  - `ListingPriceHistory` and `PriceChangeAlert` `old_price`/`new_price`/`change_percentage`
 *    — `backend/crates/db/src/models/reality_portal.rs` (default snake_case serde).
 *
 * The fixture data classes below are deliberately minimal mirrors of those structs. They double as a
 * **copy-paste template**: any future model mapping a reality-server Decimal-backed column MUST
 * annotate the field with [DecimalAsLongSerializer] or [DecimalAsDoubleSerializer] — exactly as
 * shown here — and never a bare `Long`/`Double`.
 */
class DecimalWireContractTest {

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = false
    }

    // --- PortalListingResponse (camelCase, portal_listings.rs) --------------------------------

    @Test
    fun portalListingResponse_decodes_real_serde_str_string_wire() {
        // price: DECIMAL(15,2) -> "200000.00"; sizeSqm: DECIMAL -> "72.50" (fractional scale).
        val raw =
            """
            {
              "id": "00000000-0000-0000-0000-000000000001",
              "price": "200000.00",
              "sizeSqm": "72.50"
            }
            """
                .trimIndent()
        val fixture: PortalListingResponseFixture = json.decodeFromString(raw)
        assertEquals(200_000L, fixture.price)
        assertEquals(72L, fixture.sizeSqm) // whole-unit truncation is the documented Long contract
    }

    @Test
    fun portalListingResponse_tolerates_numeric_wire_and_null_size() {
        val raw =
            """
            {
              "id": "00000000-0000-0000-0000-000000000001",
              "price": 200000
            }
            """
                .trimIndent()
        val fixture: PortalListingResponseFixture = json.decodeFromString(raw)
        assertEquals(200_000L, fixture.price)
        assertNull(fixture.sizeSqm)
    }

    // --- ListingPriceHistory / PriceChangeAlert (snake_case, reality_portal.rs) ---------------

    @Test
    fun listingPriceHistory_decodes_real_serde_str_string_wire() {
        val raw =
            """
            {
              "id": "00000000-0000-0000-0000-000000000009",
              "listing_id": "00000000-0000-0000-0000-000000000001",
              "old_price": "200000.00",
              "new_price": "185000.00",
              "currency": "EUR",
              "change_percentage": "-7.50",
              "changed_at": "2026-07-10T10:00:00Z"
            }
            """
                .trimIndent()
        val fixture: ListingPriceHistoryFixture = json.decodeFromString(raw)
        assertEquals(200_000L, fixture.old_price)
        assertEquals(185_000L, fixture.new_price)
        assertEquals(-7.5, fixture.change_percentage)
    }

    @Test
    fun listingPriceHistory_decodes_null_change_percentage() {
        val raw =
            """
            {
              "id": "00000000-0000-0000-0000-000000000009",
              "old_price": "200000.00",
              "new_price": "185000.00",
              "currency": "EUR",
              "changed_at": "2026-07-10T10:00:00Z"
            }
            """
                .trimIndent()
        val fixture: ListingPriceHistoryFixture = json.decodeFromString(raw)
        assertNull(fixture.change_percentage)
    }

    @Test
    fun priceChangeAlert_decodes_real_serde_str_string_wire() {
        val raw =
            """
            {
              "listing_id": "00000000-0000-0000-0000-000000000001",
              "title": "Bright 2br in Old Town",
              "old_price": "200000.00",
              "new_price": "185000.00",
              "currency": "EUR",
              "change_percentage": "-7.50",
              "changed_at": "2026-07-10T10:00:00Z"
            }
            """
                .trimIndent()
        val fixture: PriceChangeAlertFixture = json.decodeFromString(raw)
        assertEquals(200_000L, fixture.old_price)
        assertEquals(185_000L, fixture.new_price)
        assertEquals(-7.5, fixture.change_percentage)
    }

    @Test
    fun priceChangeAlert_tolerates_numeric_wire() {
        val raw =
            """
            {
              "listing_id": "00000000-0000-0000-0000-000000000001",
              "old_price": 200000,
              "new_price": 185000,
              "change_percentage": -7.5
            }
            """
                .trimIndent()
        val fixture: PriceChangeAlertFixture = json.decodeFromString(raw)
        assertEquals(200_000L, fixture.old_price)
        assertEquals(185_000L, fixture.new_price)
        assertEquals(-7.5, fixture.change_percentage)
    }
}

/**
 * Minimal mirror of `PortalListingResponse` (camelCase serde) — Decimal-backed fields only. `price`
 * is `DECIMAL(15,2)` (required); `sizeSqm` is `DECIMAL` (nullable). Both map through
 * [DecimalAsLongSerializer].
 */
@Serializable
private data class PortalListingResponseFixture(
    val id: String,
    @Serializable(with = DecimalAsLongSerializer::class) val price: Long = 0,
    @Serializable(with = DecimalAsLongSerializer::class) val sizeSqm: Long? = null,
)

/**
 * Minimal mirror of `ListingPriceHistory` (snake_case serde) — nullable `change_percentage`.
 * Property names are snake_case to match the wire keys directly (a real model would use camelCase +
 * `@SerialName`); the load-bearing part is the serializer annotation on each Decimal-backed field.
 */
@Serializable
private data class ListingPriceHistoryFixture(
    @Serializable(with = DecimalAsLongSerializer::class) val old_price: Long,
    @Serializable(with = DecimalAsLongSerializer::class) val new_price: Long,
    @Serializable(with = DecimalAsDoubleSerializer::class) val change_percentage: Double? = null,
)

/** Minimal mirror of `PriceChangeAlert` (snake_case serde) — required `change_percentage`. */
@Serializable
private data class PriceChangeAlertFixture(
    @Serializable(with = DecimalAsLongSerializer::class) val old_price: Long,
    @Serializable(with = DecimalAsLongSerializer::class) val new_price: Long,
    @Serializable(with = DecimalAsDoubleSerializer::class) val change_percentage: Double,
)
