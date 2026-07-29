package three.two.bit.ppt.reality.ui.listing

import android.util.Log
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.layout.ResolvedLayoutSection
import three.two.bit.ppt.reality.listing.ListingDetail

/**
 * Context object that bundles the parameters the individual listing-section composables need.
 * Passed through [ListingSectionRegistry.sectionItems] so callers do not need to thread each
 * individual callback down into the registry.
 */
data class ListingSectionContext(
    val listing: ListingDetail,
    val isFavorite: Boolean,
    val selectedTab: Int,
    val onBackClick: () -> Unit,
    val onShareClick: () -> Unit,
    val onFavoriteClick: () -> Unit,
    val onCallClick: () -> Unit,
    val onTabSelect: (Int) -> Unit,
    /**
     * True when the resolved layout contains a VISIBLE (non-placeholder) features.v1 section. Gates
     * the emission of the feature chips inside description.v1's tab 0 — features.v1 itself emits
     * nothing in-list (its content is folded into the description tab).
     */
    val showFeatures: Boolean = true,
)

/**
 * Dispatches [ResolvedLayoutSection] entries from the server-resolved layout onto the existing
 * listing-detail composables.
 *
 * Supported types:
 * - gallery.v1 → [HeroGallery]
 * - listing-header.v1 → [HeaderSection]
 * - key-details.v1 → [QuickStatsStrip]
 * - description.v1 → [TabStrip] + tab-gated description / building / nearby / price-history
 * - agent-contact.v1 → POSITION-INDEPENDENT: the VISIBILITY of [StickyAgentBar] and
 *   [BottomActionBar] is gated on this section being present and active, but the bars are rendered
 *   at fixed positions by the caller ([ListingContent]) — NOT at the section's list position. This
 *   ensures pixel-identical output regardless of where agent-contact.v1 appears in the server
 *   layout (e.g. last in [DEFAULT_LISTING_DETAIL_LAYOUT]). Only the placeholder case emits a card
 *   in-list.
 * - features.v1 → no-op in-list; its presence (visible, non-placeholder) gates the feature chips
 *   rendered inside description.v1's tab 0 (see [ListingSectionContext.showFeatures])
 * - additional-info.v1 → no-op
 * - resources.v1 → no-op
 * - unknown → no-op (+ log warn)
 *
 * Placeholder presentation → [PlaceholderSection] (neutral card, ~96dp) instead of the real
 * composable. For agent-contact.v1 with placeholder presentation: placeholder card in the list,
 * neither [StickyAgentBar] nor [BottomActionBar] renders.
 *
 * Under [DEFAULT_LISTING_DETAIL_LAYOUT] the output is pixel-identical to the prior fixed item {}
 * list — this is a pure restructure.
 */
object ListingSectionRegistry {

    private const val TAG = "ListingSectionRegistry"

    /** Unknown section types already warned about — warn once per type per process. */
    private val warnedUnknownTypes: MutableSet<String> = mutableSetOf()

    /** The eight section types the registry knows about. */
    val supportedTypes: Set<String> =
        setOf(
            "gallery.v1",
            "listing-header.v1",
            "key-details.v1",
            "description.v1",
            "agent-contact.v1",
            "features.v1",
            "additional-info.v1",
            "resources.v1",
        )

    /**
     * Emit LazyList items for a single [section]. [index] is the section's position in the resolved
     * sections list and is suffixed into every item key so that keys stay unique even if the server
     * (unexpectedly) sends duplicate section types — LazyColumn crashes on duplicate keys. The
     * shared LayoutRepository already dedupes by type; this is defense-in-depth.
     *
     * The [sectionAgentContactIsActive] helper lets the caller decide whether to render
     * [StickyAgentBar] and [BottomActionBar] at their fixed visual positions (after the gallery and
     * at screen-bottom respectively) without requiring a separate pass over the sections list.
     *
     * NOTE: agent-contact.v1 with a non-placeholder presentation emits NOTHING in the lazy list.
     * [StickyAgentBar] is placed by the caller immediately after the gallery item;
     * [BottomActionBar] is placed by the caller outside the LazyColumn at screen-bottom. This makes
     * bar position independent of where agent-contact.v1 appears in the server-resolved layout
     * order.
     */
    fun LazyListScope.sectionItems(
        section: ResolvedLayoutSection,
        index: Int,
        ctx: ListingSectionContext,
    ) {
        when (section.type) {
            "gallery.v1" -> {
                if (section.isPlaceholder) {
                    item(key = "placeholder-gallery-$index") { PlaceholderSection() }
                } else {
                    item(key = "gallery-$index") {
                        HeroGallery(
                            images = ctx.listing.images,
                            isFavorite = ctx.isFavorite,
                            onBackClick = ctx.onBackClick,
                            onShareClick = ctx.onShareClick,
                            onFavoriteClick = ctx.onFavoriteClick,
                        )
                    }
                }
            }

            "agent-contact.v1" -> {
                // POSITION-INDEPENDENT: when the section is active (non-placeholder), the bars
                // are rendered at fixed slots by the caller (StickyAgentBar after gallery,
                // BottomActionBar at screen-bottom). Nothing is emitted in-list for the active
                // case so that the bar position does not change with the layout order.
                if (section.isPlaceholder) {
                    item(key = "placeholder-agent-contact-$index") { PlaceholderSection() }
                }
                // else: no-op — StickyAgentBar and BottomActionBar rendered by ListingContent
            }

            "listing-header.v1" -> {
                if (section.isPlaceholder) {
                    item(key = "placeholder-listing-header-$index") { PlaceholderSection() }
                } else {
                    item(key = "listing-header-$index") { HeaderSection(listing = ctx.listing) }
                    // Pre-refactor layout had a dedicated 18dp gap after the header.
                    item(key = "listing-header-gap-$index") {
                        Spacer(modifier = Modifier.height(18.dp))
                    }
                }
            }

            "key-details.v1" -> {
                if (section.isPlaceholder) {
                    item(key = "placeholder-key-details-$index") { PlaceholderSection() }
                } else {
                    item(key = "key-details-$index") { QuickStatsStrip(listing = ctx.listing) }
                }
            }

            "description.v1" -> {
                if (section.isPlaceholder) {
                    item(key = "placeholder-description-$index") { PlaceholderSection() }
                } else {
                    item(key = "tab-strip-$index") {
                        TabStrip(selected = ctx.selectedTab, onSelect = ctx.onTabSelect)
                    }
                    when (ctx.selectedTab) {
                        0 -> {
                            item(key = "description-body-$index") {
                                DescriptionBody(description = ctx.listing.description)
                            }
                            // Feature chips render only when the resolved layout contains a
                            // VISIBLE features.v1 section (features.v1 contract) AND the
                            // listing actually has features.
                            if (ctx.showFeatures && ctx.listing.features.isNotEmpty()) {
                                item(key = "features-chips-$index") {
                                    FeaturesChips(features = ctx.listing.features)
                                }
                            }
                        }
                        1 ->
                            item(key = "building-passport-$index") {
                                BuildingPassportCard(listing = ctx.listing)
                            }
                        2 -> item(key = "nearby-preview-$index") { NearbyPreviewCard() }
                        3 ->
                            item(key = "price-history-$index") {
                                PriceHistoryCard(listing = ctx.listing)
                            }
                    }
                }
            }

            // features.v1, additional-info.v1, resources.v1 — folded into other sections
            "features.v1",
            "additional-info.v1",
            "resources.v1" -> {
                // intentional no-op: content rendered as part of other sections
            }

            else -> {
                // Unknown section type — warn once per type, then no-op.
                if (warnedUnknownTypes.add(section.type)) {
                    Log.w(TAG, "unknown section type '${section.type}' — skipping")
                }
            }
        }
    }

    /**
     * Returns true when the sections list contains an agent-contact.v1 section that is NOT a
     * placeholder (i.e. the bottom action bar should be rendered).
     */
    fun sectionAgentContactIsActive(sections: List<ResolvedLayoutSection>): Boolean = sections.any {
        it.type == "agent-contact.v1" && !it.isPlaceholder
    }
}

// ─── Placeholder ─────────────────────────────────────────────────────────────

/**
 * Neutral placeholder card shown when a section's presentation is "placeholder". Min height ~96dp,
 * uses surface-variant background and onSurfaceVariant text to stay neutral. String resource
 * [R.string.detail_section_unavailable] follows app string-resource conventions.
 */
@Composable
fun PlaceholderSection(modifier: Modifier = Modifier) {
    Card(
        modifier =
            modifier
                .fillMaxWidth()
                .heightIn(min = 96.dp)
                .padding(horizontal = 16.dp, vertical = 8.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
    ) {
        Box(
            modifier = Modifier.fillMaxWidth().heightIn(min = 96.dp),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = stringResource(R.string.detail_section_unavailable),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
