package three.two.bit.ppt.reality.ui.listing

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import three.two.bit.ppt.reality.listing.*

// ─── Content ────────────────────────────────────────────────────────────────

@OptIn(ExperimentalFoundationApi::class)
@Composable
internal fun ListingContent(
    listing: ListingDetail,
    isFavorite: Boolean,
    onBackClick: () -> Unit,
    onShareClick: () -> Unit,
    onFavoriteClick: () -> Unit,
    onCallClick: () -> Unit,
    onMessageClick: () -> Unit,
    onInquiryClick: () -> Unit,
) {
    var selectedTab by remember { mutableIntStateOf(0) }
    Box(modifier = Modifier.fillMaxSize()) {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(bottom = 110.dp),
        ) {
            item {
                HeroGallery(
                    images = listing.images,
                    isFavorite = isFavorite,
                    onBackClick = onBackClick,
                    onShareClick = onShareClick,
                    onFavoriteClick = onFavoriteClick,
                )
            }
            item { StickyAgentBar(listing = listing, onCallClick = onCallClick) }
            item { HeaderSection(listing = listing) }
            item { Spacer(modifier = Modifier.height(18.dp)) }
            item { QuickStatsStrip(listing = listing) }
            item { TabStrip(selected = selectedTab, onSelect = { selectedTab = it }) }
            when (selectedTab) {
                0 -> {
                    item { DescriptionBody(description = listing.description) }
                    if (listing.features.isNotEmpty()) {
                        item { FeaturesChips(features = listing.features) }
                    }
                }
                1 -> item { BuildingPassportCard(listing = listing) }
                2 -> item { NearbyPreviewCard() }
                3 -> item { PriceHistoryCard(listing = listing) }
            }
            item { Spacer(modifier = Modifier.height(16.dp)) }
        }
        BottomActionBar(
            modifier = Modifier.align(Alignment.BottomCenter),
            onMessageClick = onMessageClick,
            onInquiryClick = onInquiryClick,
        )
    }
}
