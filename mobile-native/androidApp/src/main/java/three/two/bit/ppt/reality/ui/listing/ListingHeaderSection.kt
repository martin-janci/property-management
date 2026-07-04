package three.two.bit.ppt.reality.ui.listing

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.listing.*
import three.two.bit.ppt.reality.ui.theme.BadgeColors
import three.two.bit.ppt.reality.util.FormatUtils

// ─── Header (price + title + location + badges) ─────────────────────────────

@Composable
internal fun HeaderSection(listing: ListingDetail) {
    Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 14.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            if (listing.isFeatured) {
                UppercaseBadge(
                    text = stringResource(R.string.label_featured),
                    bg = BadgeColors.featuredBg,
                    ink = BadgeColors.featuredInk,
                )
            }
            UppercaseBadge(
                text =
                    when (listing.type) {
                        ListingType.SALE -> stringResource(R.string.for_sale)
                        ListingType.RENT -> stringResource(R.string.for_rent)
                    },
                bg =
                    when (listing.type) {
                        ListingType.SALE -> BadgeColors.saleBg
                        ListingType.RENT -> BadgeColors.rentBg
                    },
                ink =
                    when (listing.type) {
                        ListingType.SALE -> BadgeColors.saleInk
                        ListingType.RENT -> BadgeColors.rentInk
                    },
            )
        }
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = FormatUtils.formatPrice(listing.price, listing.currency),
            style = MaterialTheme.typography.headlineLarge,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface,
        )
        listing.pricePerSqm?.let {
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = "${FormatUtils.formatPrice(it, listing.currency)}/m²",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Spacer(modifier = Modifier.height(14.dp))
        Text(
            text = listing.title,
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(modifier = Modifier.height(4.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Icon(
                Icons.Default.LocationOn,
                contentDescription = null,
                modifier = Modifier.size(13.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.width(4.dp))
            Text(
                text = FormatUtils.buildDetailedLocationString(listing.address),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun UppercaseBadge(text: String, bg: Color, ink: Color) {
    Surface(shape = RoundedCornerShape(6.dp), color = bg) {
        Text(
            text = text.uppercase(),
            modifier = Modifier.padding(horizontal = 9.dp, vertical = 3.dp),
            style =
                MaterialTheme.typography.labelSmall.copy(
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 0.04.sp,
                ),
            color = ink,
        )
    }
}

// ─── Quick stats strip ──────────────────────────────────────────────────────

@Composable
internal fun QuickStatsStrip(listing: ListingDetail) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        StatCard(
            icon = Icons.Default.SquareFoot,
            value = "${listing.areaSqm.toInt()} m²",
            label = stringResource(R.string.detail_stat_area),
            modifier = Modifier.weight(1f),
        )
        StatCard(
            icon = Icons.Default.Apartment,
            value = listing.yearBuilt?.toString() ?: "—",
            label = stringResource(R.string.detail_stat_built),
            modifier = Modifier.weight(1f),
        )
        StatCard(
            icon = Icons.Default.Bolt,
            value = listing.energyRating ?: "—",
            label = stringResource(R.string.detail_stat_energy),
            modifier = Modifier.weight(1f),
        )
        StatCard(
            icon = Icons.Default.Layers,
            value =
                listing.floor?.let { floor ->
                    listing.totalFloors?.let { "$floor/$it" } ?: "$floor"
                } ?: "—",
            label = stringResource(R.string.detail_stat_floor),
            modifier = Modifier.weight(1f),
        )
    }
}

@Composable
private fun StatCard(
    icon: ImageVector,
    value: String,
    label: String,
    modifier: Modifier = Modifier,
) {
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(12.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 6.dp, vertical = 10.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = value,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = label.uppercase(),
                style =
                    MaterialTheme.typography.labelSmall.copy(
                        fontWeight = FontWeight.SemiBold,
                        letterSpacing = 0.04.sp,
                        fontSize = 9.sp,
                    ),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
