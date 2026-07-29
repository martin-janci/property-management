package three.two.bit.ppt.reality.ui.listing

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import three.two.bit.ppt.reality.layout.ResolvedLayoutScreen
import three.two.bit.ppt.reality.listing.*
import three.two.bit.ppt.reality.ui.listing.ListingSectionRegistry.sectionItems

// ─── Content ────────────────────────────────────────────────────────────────

@OptIn(ExperimentalFoundationApi::class)
@Composable
internal fun ListingContent(
    listing: ListingDetail,
    layout: ResolvedLayoutScreen,
    isFavorite: Boolean,
    onBackClick: () -> Unit,
    onShareClick: () -> Unit,
    onFavoriteClick: () -> Unit,
    onCallClick: () -> Unit,
    onMessageClick: () -> Unit,
    onInquiryClick: () -> Unit,
) {
    var selectedTab by remember { mutableIntStateOf(0) }

    // features.v1 contract: the feature chips inside description.v1's tab 0 render only when
    // the resolved layout contains a VISIBLE (non-placeholder) features.v1 section.
    val showFeatures = layout.sections.any { it.type == "features.v1" && !it.isPlaceholder }

    val ctx =
        ListingSectionContext(
            listing = listing,
            isFavorite = isFavorite,
            selectedTab = selectedTab,
            onBackClick = onBackClick,
            onShareClick = onShareClick,
            onFavoriteClick = onFavoriteClick,
            onCallClick = onCallClick,
            onTabSelect = { selectedTab = it },
            showFeatures = showFeatures,
        )

    // Determine whether the agent-contact section is active (visible, non-placeholder).
    // StickyAgentBar and BottomActionBar positions are FIXED — agent-contact.v1 controls
    // visibility only, not render position (position-independent bar placement).
    val showAgentBars = ListingSectionRegistry.sectionAgentContactIsActive(layout.sections)

    // True when a visible (non-placeholder) gallery section is present in the layout.
    val hasVisibleGallery = layout.sections.any { it.type == "gallery.v1" && !it.isPlaceholder }

    Box(modifier = Modifier.fillMaxSize()) {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(bottom = if (showAgentBars) 110.dp else 16.dp),
        ) {
            // When no visible gallery is present but agent bars are active, StickyAgentBar
            // renders at the very top of the list (traditional fallback position).
            if (showAgentBars && !hasVisibleGallery) {
                item(key = "agent-contact-bar-top") {
                    StickyAgentBar(listing = ctx.listing, onCallClick = ctx.onCallClick)
                }
            }

            for ((index, section) in layout.sections.withIndex()) {
                sectionItems(section = section, index = index, ctx = ctx)
                // After the gallery item, insert StickyAgentBar at its traditional fixed slot.
                // This is position-independent: agent-contact.v1 controls visibility but NOT
                // the render position — so the bar always appears right after the gallery
                // regardless of where agent-contact.v1 lands in the server layout order.
                // Index suffix keeps the key unique even if duplicate gallery sections slip
                // through (defense-in-depth; the repository already dedupes by type).
                if (section.type == "gallery.v1" && !section.isPlaceholder && showAgentBars) {
                    item(key = "agent-contact-bar-$index") {
                        StickyAgentBar(listing = ctx.listing, onCallClick = ctx.onCallClick)
                    }
                }
            }
            item { Spacer(modifier = Modifier.height(16.dp)) }
        }

        if (showAgentBars) {
            BottomActionBar(
                modifier = Modifier.align(Alignment.BottomCenter),
                onMessageClick = onMessageClick,
                onInquiryClick = onInquiryClick,
            )
        }
    }
}
