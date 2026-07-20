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
        )

    // Determine whether the agent-contact section is active (visible, non-placeholder).
    // This controls BottomActionBar visibility outside the LazyColumn.
    val showActionBar = ListingSectionRegistry.sectionAgentContactIsActive(layout.sections)

    Box(modifier = Modifier.fillMaxSize()) {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(bottom = if (showActionBar) 110.dp else 16.dp),
        ) {
            for (section in layout.sections) {
                sectionItems(section = section, ctx = ctx)
            }
            item { Spacer(modifier = Modifier.height(16.dp)) }
        }

        if (showActionBar) {
            BottomActionBar(
                modifier = Modifier.align(Alignment.BottomCenter),
                onMessageClick = onMessageClick,
                onInquiryClick = onInquiryClick,
            )
        }
    }
}
