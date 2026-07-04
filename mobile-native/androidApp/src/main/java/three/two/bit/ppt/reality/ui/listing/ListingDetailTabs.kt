package three.two.bit.ppt.reality.ui.listing

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.listing.*

// ─── Tab strip ──────────────────────────────────────────────────────────────

@Composable
internal fun TabStrip(selected: Int, onSelect: (Int) -> Unit) {
    val tabs =
        listOf(
            stringResource(R.string.detail_tab_overview),
            stringResource(R.string.detail_tab_building),
            stringResource(R.string.detail_tab_nearby),
            stringResource(R.string.detail_tab_price_history),
        )
    Row(
        modifier =
            Modifier.fillMaxWidth().padding(top = 18.dp).horizontalScroll(rememberScrollState())
    ) {
        tabs.forEachIndexed { index, label ->
            val isOn = selected == index
            Column(
                modifier =
                    Modifier.clickable { onSelect(index) }
                        .padding(start = if (index == 0) 16.dp else 0.dp, end = 12.dp)
            ) {
                Text(
                    text = label,
                    modifier = Modifier.padding(vertical = 10.dp),
                    style = MaterialTheme.typography.labelLarge,
                    fontWeight = if (isOn) FontWeight.SemiBold else FontWeight.Medium,
                    color =
                        if (isOn) MaterialTheme.colorScheme.primary
                        else MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Box(
                    modifier =
                        Modifier.height(2.dp)
                            .width(if (isOn) 44.dp else 0.dp)
                            .background(MaterialTheme.colorScheme.primary)
                )
            }
        }
    }
    HorizontalDivider(thickness = 1.dp, color = MaterialTheme.colorScheme.outlineVariant)
}

// ─── Description + features ─────────────────────────────────────────────────

@Composable
internal fun DescriptionBody(description: String) {
    Text(
        text = description,
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 14.dp),
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurface,
    )
}

@Composable
internal fun FeaturesChips(features: List<String>) {
    Column(modifier = Modifier.padding(horizontal = 16.dp)) {
        Text(
            text = stringResource(R.string.features),
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(modifier = Modifier.height(8.dp))
        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            items(features) { feature ->
                AssistChip(
                    onClick = {},
                    label = { Text(feature) },
                    leadingIcon = {
                        Icon(
                            Icons.Default.Check,
                            contentDescription = null,
                            modifier = Modifier.size(16.dp),
                        )
                    },
                )
            }
        }
    }
}

// ─── Building passport tab ──────────────────────────────────────────────────

@Composable
internal fun BuildingPassportCard(listing: ListingDetail) {
    val rows = buildList {
        listing.yearBuilt?.let { add(stringResource(R.string.detail_year_built) to "$it") }
        listing.yearRenovated?.let { add(stringResource(R.string.detail_year_renovated) to "$it") }
        listing.floor?.let {
            val floorText = listing.totalFloors?.let { tf -> "$it/$tf" } ?: "$it"
            add(stringResource(R.string.detail_floor) to floorText)
        }
        listing.usableAreaSqm?.let {
            add(stringResource(R.string.detail_usable_area) to "${it.toInt()} m²")
        }
        listing.landAreaSqm?.let {
            add(stringResource(R.string.detail_land_area) to "${it.toInt()} m²")
        }
        listing.energyRating?.let { add(stringResource(R.string.detail_energy_rating) to it) }
        listing.heatingType?.let { add(stringResource(R.string.detail_heating) to it) }
        listing.parking?.let { add(stringResource(R.string.detail_parking) to it) }
    }
    if (rows.isEmpty()) {
        PlaceholderSection(stringResource(R.string.detail_building_no_data))
        return
    }
    Card(
        modifier = Modifier.fillMaxWidth().padding(16.dp),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            rows.chunked(2).forEachIndexed { rowIdx, pair ->
                if (rowIdx > 0) Spacer(modifier = Modifier.height(14.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                    pair.forEach { (label, value) ->
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = label.uppercase(),
                                style =
                                    MaterialTheme.typography.labelSmall.copy(
                                        fontWeight = FontWeight.SemiBold,
                                        letterSpacing = 0.04.sp,
                                        fontSize = 10.sp,
                                    ),
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Spacer(modifier = Modifier.height(2.dp))
                            Text(
                                text = value,
                                style = MaterialTheme.typography.bodyMedium,
                                fontWeight = FontWeight.SemiBold,
                                color = MaterialTheme.colorScheme.onSurface,
                            )
                        }
                    }
                    if (pair.size == 1) Spacer(modifier = Modifier.weight(1f))
                }
            }
        }
    }
}

// ─── Nearby tab placeholder ─────────────────────────────────────────────────

@Composable
internal fun NearbyPreviewCard() {
    Box(
        modifier =
            Modifier.fillMaxWidth()
                .padding(16.dp)
                .clip(RoundedCornerShape(16.dp))
                .background(MaterialTheme.colorScheme.surfaceVariant)
                .height(160.dp),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Icon(
                Icons.Default.Map,
                contentDescription = null,
                modifier = Modifier.size(40.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = stringResource(R.string.map_placeholder),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun PlaceholderSection(text: String) {
    Box(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 32.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
