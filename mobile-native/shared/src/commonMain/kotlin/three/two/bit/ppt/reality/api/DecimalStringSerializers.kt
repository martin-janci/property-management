package three.two.bit.ppt.reality.api

import kotlinx.serialization.KSerializer
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.PrimitiveSerialDescriptor
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlinx.serialization.json.JsonDecoder
import kotlinx.serialization.json.jsonPrimitive

/**
 * Wire-tolerant serializers for reality-server monetary fields.
 *
 * The backend enables `rust_decimal` with the `serde-str` feature (`backend/Cargo.toml`), so every
 * `rust_decimal::Decimal` field serializes as a **JSON string** with its full fractional scale —
 * e.g. a `DECIMAL(15,2)` price is `"200000.00"` and a `DECIMAL(5,2)` percentage is `"-7.50"`, NOT a
 * JSON number.
 *
 * The favorites/alerts endpoints (`GET /api/v1/favorites`, `GET /api/v1/favorites/alerts`)
 * serialize the DB models (`PortalFavoriteWithListing`, `FavoriteAlert`) directly, so their
 * monetary fields arrive as these scaled strings. kotlinx-serialization's `isLenient = true`
 * tolerates quoted *integral* literals for `Long`/`Double`, but a fractional string like
 * `"200000.00"` throws `JsonDecodingException` against a bare `Long` — which silently killed the
 * whole price-tracking payload decode (see issue #2331).
 *
 * These serializers read the raw JSON primitive and accept **either** a JSON number or a
 * (optionally fractional) JSON string, so a field annotated with them decodes both the real server
 * wire and the historical numeric fixtures. On the encode side they emit a plain number so
 * client-authored round-trips stay compact.
 *
 * NOTE: these are JSON-only (they cast to [JsonDecoder]); the shared client only ever speaks JSON.
 */

/**
 * Decodes a whole-currency amount from a JSON number or a decimal string, truncating any fractional
 * scale (`"200000.00"` -> `200000`, `200000` -> `200000`). Prices in the portal models are
 * whole-currency `Long`s, so the sub-unit scale carried by `DECIMAL(15,2)` is dropped.
 */
object DecimalAsLongSerializer : KSerializer<Long> {
    override val descriptor: SerialDescriptor =
        PrimitiveSerialDescriptor("DecimalAsLong", PrimitiveKind.LONG)

    override fun deserialize(decoder: Decoder): Long {
        val raw = (decoder as JsonDecoder).decodeJsonElement().jsonPrimitive.content
        return raw.substringBefore('.').toLong()
    }

    override fun serialize(encoder: Encoder, value: Long) = encoder.encodeLong(value)
}

/**
 * Decodes a signed fractional amount (e.g. a percentage) from a JSON number or a decimal string
 * (`"-7.50"` -> `-7.5`, `-7.5` -> `-7.5`).
 */
object DecimalAsDoubleSerializer : KSerializer<Double> {
    override val descriptor: SerialDescriptor =
        PrimitiveSerialDescriptor("DecimalAsDouble", PrimitiveKind.DOUBLE)

    override fun deserialize(decoder: Decoder): Double {
        val raw = (decoder as JsonDecoder).decodeJsonElement().jsonPrimitive.content
        return raw.toDouble()
    }

    override fun serialize(encoder: Encoder, value: Double) = encoder.encodeDouble(value)
}
