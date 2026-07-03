package three.two.bit.ppt.reality.ui.listing

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ShowChart
import androidx.compose.material.icons.automirrored.filled.TrendingDown
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.listing.*
import three.two.bit.ppt.reality.util.FormatUtils

// ─── Price history tab ──────────────────────────────────────────────────────
//
// The reality-server listing payload exposes a single `previousPrice` alongside
// `isPriceReduced` rather than a full change log, so we render the one known
// transition when present and fall back to a proper empty state otherwise.

@Composable
internal fun PriceHistoryCard(listing: ListingDetail) {
    val previous = listing.previousPrice
    if (!listing.isPriceReduced || previous == null) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Icon(
                Icons.AutoMirrored.Filled.ShowChart,
                contentDescription = null,
                modifier = Modifier.size(40.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = stringResource(R.string.price_history_empty_title),
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = stringResource(R.string.price_history_empty_body),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
        }
        return
    }

    val delta = previous - listing.price
    Card(
        modifier = Modifier.fillMaxWidth().padding(16.dp),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            PriceHistoryRow(
                label = stringResource(R.string.price_history_previous),
                value = FormatUtils.formatPrice(previous, listing.currency),
                muted = true,
            )
            Spacer(modifier = Modifier.height(12.dp))
            PriceHistoryRow(
                label = stringResource(R.string.price_history_current),
                value = FormatUtils.formatPrice(listing.price, listing.currency),
                muted = false,
            )
            if (delta > 0) {
                Spacer(modifier = Modifier.height(14.dp))
                Surface(shape = RoundedCornerShape(999.dp), color = Color(0xFFD1FAE5)) {
                    Row(
                        modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(
                            Icons.AutoMirrored.Filled.TrendingDown,
                            contentDescription = null,
                            tint = Color(0xFF065F46),
                            modifier = Modifier.size(14.dp),
                        )
                        Spacer(modifier = Modifier.width(4.dp))
                        Text(
                            text =
                                stringResource(
                                    R.string.price_history_reduced_by,
                                    FormatUtils.formatPrice(delta, listing.currency),
                                ),
                            style = MaterialTheme.typography.labelMedium,
                            fontWeight = FontWeight.SemiBold,
                            color = Color(0xFF065F46),
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun PriceHistoryRow(label: String, value: String, muted: Boolean) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            text = value,
            style =
                if (muted) MaterialTheme.typography.bodyMedium
                else MaterialTheme.typography.titleMedium,
            fontWeight = if (muted) FontWeight.Normal else FontWeight.Bold,
            color =
                if (muted) MaterialTheme.colorScheme.onSurfaceVariant
                else MaterialTheme.colorScheme.onSurface,
        )
    }
}
