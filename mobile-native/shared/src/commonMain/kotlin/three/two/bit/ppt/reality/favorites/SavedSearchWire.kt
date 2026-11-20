package three.two.bit.ppt.reality.favorites

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
import kotlinx.serialization.json.put

/**
 * Wire DTOs for `/api/v1/saved-searches` (db `PortalSavedSearch`, `CreatePortalSavedSearch`,
 * `UpdatePortalSavedSearch`). The server stores the search as `criteria`, a JSON object with the
 * same keys as the public listing search (`q`, `property_type`, `transaction_type`, `city`,
 * `price_min`, `price_max`, `rooms_min`, ...), so the domain [SavedSearch] is mapped onto it.
 */
@Serializable
internal data class WirePortalSavedSearch(
    val id: String,
    val name: String,
    val criteria: JsonObject = JsonObject(emptyMap()),
    @SerialName("alerts_enabled") val alertsEnabled: Boolean = false,
    @SerialName("alert_frequency") val alertFrequency: String? = null,
    @SerialName("last_alert_sent_at") val lastAlertSentAt: String? = null,
    @SerialName("created_at") val createdAt: String = "",
)

@Serializable
internal data class WireSavedSearchesResponse(
    val searches: List<WirePortalSavedSearch> = emptyList(),
)

@Serializable
internal data class WireCreateSavedSearch(
    val name: String,
    val criteria: JsonObject,
    @SerialName("alerts_enabled") val alertsEnabled: Boolean,
)

@Serializable
internal data class WireUpdateSavedSearch(
    val name: String? = null,
    @SerialName("alerts_enabled") val alertsEnabled: Boolean? = null,
)

/** App category keys that differ from the server's `property_type` vocabulary. */
private fun categoryToServer(category: String): String =
    when (category.lowercase()) {
        "garage" -> "parking"
        "warehouse" -> "storage"
        "office" -> "commercial"
        else -> category.lowercase()
    }

private fun categoryFromServer(propertyType: String): String =
    when (propertyType) {
        "parking" -> "garage"
        "storage" -> "warehouse"
        else -> propertyType
    }

internal fun criteriaOf(query: String?, filters: SavedSearchFilters?): JsonObject =
    buildJsonObject {
        query?.takeIf { it.isNotBlank() }?.let { put("q", it) }
        filters?.type?.let { put("transaction_type", it.lowercase()) }
        filters?.category?.let { put("property_type", categoryToServer(it)) }
        filters?.city?.takeIf { it.isNotBlank() }?.let { put("city", it) }
        filters?.minPrice?.let { put("price_min", it) }
        filters?.maxPrice?.let { put("price_max", it) }
        filters?.minRooms?.let { put("rooms_min", it) }
    }

private fun JsonObject.string(key: String): String? =
    (this[key] as? JsonPrimitive)?.takeIf { it.isString }?.contentOrNull

internal fun WirePortalSavedSearch.toDomain(): SavedSearch {
    val filters =
        SavedSearchFilters(
            type = criteria.string("transaction_type"),
            category = criteria.string("property_type")?.let(::categoryFromServer),
            city = criteria.string("city"),
            minPrice = criteria["price_min"]?.jsonPrimitive?.longOrNull,
            maxPrice = criteria["price_max"]?.jsonPrimitive?.longOrNull,
            minRooms = criteria["rooms_min"]?.jsonPrimitive?.intOrNull,
        )
    val hasFilters = filters != SavedSearchFilters()
    return SavedSearch(
        id = id,
        name = name,
        query = criteria.string("q"),
        filters = if (hasFilters) filters else null,
        alertEnabled = alertsEnabled,
        createdAt = createdAt,
        lastNotifiedAt = lastAlertSentAt,
    )
}
