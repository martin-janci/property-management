package three.two.bit.ppt.reality.layout

/**
 * Compiled default layout for the Reality Portal listing-detail screen.
 *
 * Used as a fallback when the server is unreachable, returns a non-2xx status, returns a malformed
 * body, or echoes back a different screen identifier than expected. The section order mirrors
 * reality-web's DEFAULT_LISTING_DETAIL_LAYOUT (frontend/apps/reality-web/src/lib/layout.ts);
 * agent-contact.v1 is last in both.
 */
val DEFAULT_LISTING_DETAIL_LAYOUT =
    ResolvedLayoutScreen(
        screen = "reality/listing-detail",
        version = 0,
        sections =
            listOf(
                ResolvedLayoutSection(type = "gallery.v1"),
                ResolvedLayoutSection(type = "listing-header.v1"),
                ResolvedLayoutSection(type = "key-details.v1"),
                ResolvedLayoutSection(type = "description.v1"),
                ResolvedLayoutSection(type = "features.v1"),
                ResolvedLayoutSection(type = "additional-info.v1"),
                ResolvedLayoutSection(type = "resources.v1"),
                ResolvedLayoutSection(type = "agent-contact.v1"),
            ),
    )

/**
 * Stable Swift-accessible accessor for compiled default layouts.
 *
 * Kotlin `object` declarations surface to Swift as `LayoutDefaults.shared`, providing a stable
 * idiom (`LayoutDefaults.shared.listingDetail`) that survives Kotlin compiler renames of top-level
 * `val` bindings (which surface as `DefaultLayoutKt.DEFAULT_LISTING_DETAIL_LAYOUT` — an
 * implementation-detail name that breaks on refactor).
 */
object LayoutDefaults {
    /** Default resolved layout for the listing-detail screen. */
    val listingDetail: ResolvedLayoutScreen
        get() = DEFAULT_LISTING_DETAIL_LAYOUT
}
