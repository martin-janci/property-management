package three.two.bit.ppt.reality.ui.realtor

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import coil.request.ImageRequest
import three.two.bit.ppt.reality.R

/**
 * "My listings" screen — KMP / Compose M3 redesign matching the design (`KmpMyListingsScreen`).
 * Large-title header, status chip strip (All / Active / Paused / Sold / Drafts), 16:9 cards with
 * uppercase status badge top-left + white kebab pill top-right, price row with inline view +
 * inquiry counters, title, meta. Extended FAB "Add".
 *
 * UC-49 / UC-51.3 — Realtor listing management.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MyListingsScreen(
    listings: List<RealtorListing>,
    isLoading: Boolean,
    onBackClick: () -> Unit,
    onCreateClick: () -> Unit,
    onListingClick: (id: String) -> Unit,
) {
    var statusFilter by remember { mutableStateOf<String?>(null) }
    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Box(modifier = Modifier.fillMaxSize()) {
            Column(modifier = Modifier.fillMaxSize()) {
                Row(
                    modifier =
                        Modifier.fillMaxWidth()
                            .padding(start = 4.dp, end = 16.dp, top = 14.dp, bottom = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    IconButton(onClick = onBackClick) {
                        Icon(
                            Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = stringResource(R.string.cd_back),
                        )
                    }
                    Text(
                        text = stringResource(R.string.realtor_listings_title),
                        style = MaterialTheme.typography.headlineSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                }

                val statuses = listings.map { it.status }.distinct()
                LazyRow(
                    contentPadding = PaddingValues(horizontal = 16.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier.padding(vertical = 8.dp),
                ) {
                    item {
                        StatusChip(
                            label = stringResource(R.string.filter_all),
                            count = listings.size,
                            selected = statusFilter == null,
                            onClick = { statusFilter = null },
                        )
                    }
                    items(statuses) { status ->
                        StatusChip(
                            label = status,
                            count = listings.count { it.status == status },
                            selected = statusFilter == status,
                            onClick = { statusFilter = status },
                        )
                    }
                }

                when {
                    isLoading ->
                        Box(
                            modifier = Modifier.fillMaxSize(),
                            contentAlignment = Alignment.Center,
                        ) {
                            CircularProgressIndicator()
                        }
                    listings.isEmpty() -> EmptyState()
                    else -> {
                        val filtered = listings.filter {
                            statusFilter == null || it.status == statusFilter
                        }
                        LazyColumn(
                            modifier = Modifier.fillMaxSize(),
                            contentPadding =
                                PaddingValues(start = 16.dp, end = 16.dp, bottom = 110.dp),
                            verticalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            items(filtered, key = { it.id }) { listing ->
                                ListingCard(
                                    listing = listing,
                                    onClick = { onListingClick(listing.id) },
                                )
                            }
                        }
                    }
                }
            }

            ExtendedFloatingActionButton(
                onClick = onCreateClick,
                modifier = Modifier.align(Alignment.BottomEnd).padding(end = 16.dp, bottom = 24.dp),
                containerColor = MaterialTheme.colorScheme.primary,
                contentColor = Color.White,
                icon = { Icon(Icons.Default.Add, contentDescription = null) },
                text = { Text(stringResource(R.string.realtor_listings_add)) },
            )
        }
    }
}

@Composable
private fun StatusChip(label: String, count: Int, selected: Boolean, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(8.dp),
        color = if (selected) MaterialTheme.colorScheme.primaryContainer else Color.Transparent,
        border =
            if (selected) null
            else androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
    ) {
        Row(modifier = Modifier.padding(horizontal = 14.dp, vertical = 7.dp)) {
            Text(
                text = label,
                style = MaterialTheme.typography.labelMedium,
                fontWeight = FontWeight.Medium,
                color =
                    if (selected) MaterialTheme.colorScheme.onPrimaryContainer
                    else MaterialTheme.colorScheme.onSurface,
            )
            if (count > 0) {
                Text(
                    text = " · $count",
                    style = MaterialTheme.typography.labelMedium,
                    color =
                        if (selected)
                            MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.7f)
                        else MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun ListingCard(listing: RealtorListing, onClick: () -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick),
        shape = RoundedCornerShape(12.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column {
            Box(modifier = Modifier.fillMaxWidth().aspectRatio(16f / 9f)) {
                AsyncImage(
                    model =
                        ImageRequest.Builder(LocalContext.current)
                            .data(listing.imageUrl)
                            .crossfade(true)
                            .build(),
                    contentDescription = listing.title,
                    contentScale = ContentScale.Crop,
                    modifier =
                        Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surfaceVariant),
                )
                Surface(
                    modifier = Modifier.align(Alignment.TopStart).padding(10.dp),
                    shape = RoundedCornerShape(6.dp),
                    color = MaterialTheme.colorScheme.surface,
                ) {
                    Text(
                        text = listing.status.uppercase(),
                        modifier = Modifier.padding(horizontal = 9.dp, vertical = 3.dp),
                        style =
                            MaterialTheme.typography.labelSmall.copy(
                                fontWeight = FontWeight.Bold,
                                letterSpacing = 0.04.sp,
                                fontSize = 10.sp,
                            ),
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                }
                Box(
                    modifier =
                        Modifier.align(Alignment.TopEnd)
                            .padding(8.dp)
                            .size(34.dp)
                            .clip(CircleShape)
                            .background(Color.White.copy(alpha = 0.92f))
                            .clickable { /* menu */ },
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        Icons.Default.MoreVert,
                        contentDescription = stringResource(R.string.cd_more),
                        modifier = Modifier.size(18.dp),
                        tint = MaterialTheme.colorScheme.onSurface,
                    )
                }
            }
            Column(modifier = Modifier.padding(14.dp)) {
                Row(verticalAlignment = Alignment.Bottom, modifier = Modifier.fillMaxWidth()) {
                    Text(
                        text = listing.formattedPrice,
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.weight(1f),
                    )
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Counter(icon = Icons.Default.Visibility, value = listing.views)
                        Counter(icon = Icons.Default.Email, value = listing.inquiries)
                    }
                }
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = listing.title,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium,
                    color = MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = listing.meta,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun Counter(icon: androidx.compose.ui.graphics.vector.ImageVector, value: Int) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            modifier = Modifier.size(12.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.width(3.dp))
        Text(
            text = value.toString(),
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun EmptyState() {
    Column(
        modifier = Modifier.fillMaxSize().padding(horizontal = 32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Box(
            modifier =
                Modifier.size(72.dp)
                    .clip(CircleShape)
                    .background(MaterialTheme.colorScheme.surfaceVariant),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                Icons.Default.Home,
                contentDescription = null,
                modifier = Modifier.size(32.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Spacer(modifier = Modifier.height(18.dp))
        Text(
            text = stringResource(R.string.realtor_listings_empty_title),
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.realtor_listings_empty_body),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/** Lightweight VM-shaped data class fed into the screen. */
data class RealtorListing(
    val id: String,
    val title: String,
    val formattedPrice: String,
    val transactionType: String,
    val views: Int,
    val inquiries: Int,
    val status: String = "Active",
    val meta: String = "",
    val imageUrl: String? = null,
)
