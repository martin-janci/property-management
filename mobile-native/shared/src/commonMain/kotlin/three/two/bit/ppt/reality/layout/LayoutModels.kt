package three.two.bit.ppt.reality.layout

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonObject

/**
 * Layout models for Reality Portal mobile app.
 *
 * Presentation is a String (not an enum) to guard against the enum-unknown-value hazard: unknown
 * presentation strings from the server would crash an enum decoder. Instead, defensive helpers
 * [isPlaceholder] and [isVisible] interpret the value safely — unknown strings map to visible.
 */

/** A single resolved layout section with type, optional mode/props, and a presentation hint. */
@Serializable
data class ResolvedLayoutSection(
    val type: String,
    val mode: String? = null,
    @SerialName("props") val props: JsonObject? = null,
    val presentation: String = "visible",
) {
    /** True when this section is explicitly a placeholder (hidden slot in the layout). */
    val isPlaceholder: Boolean
        get() = presentation == "placeholder"

    /**
     * True when this section should be rendered. Unknown presentation strings are treated as
     * visible (defensive — future server values must not suppress content silently).
     */
    val isVisible: Boolean
        get() = !isPlaceholder
}

/** A fully resolved layout screen containing an ordered list of sections. */
@Serializable
data class ResolvedLayoutScreen(
    val screen: String,
    val version: Int,
    val sections: List<ResolvedLayoutSection>,
)
