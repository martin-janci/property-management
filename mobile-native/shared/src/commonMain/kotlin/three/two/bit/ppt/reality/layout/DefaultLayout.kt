package three.two.bit.ppt.reality.layout

/**
 * Compiled default layout for the Reality Portal listing-detail screen.
 *
 * Used as a fallback when the server is unreachable, returns a non-2xx status, returns a malformed
 * body, or echoes back a different screen identifier than expected. The section order mirrors the
 * web base order defined in the design spec.
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
