package three.two.bit.ppt.reality.ui.agency

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
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Inbox
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.ui.theme.Brand500
import three.two.bit.ppt.reality.ui.theme.Brand800

/**
 * Agency inquiries screen — KMP / Compose M3 redesign matching the design
 * (`KmpAgencyInquiriesScreen`). Large-title header with back + search action; status filter chip
 * strip; flat thread rows with gradient avatar, name + time, 2-line preview, status uppercase
 * pill + response time (e.g. "Unanswered · 12 min" in red); FAB "Batch reply".
 *
 * UC-49.5 / UC-50 — Agency inquiry management.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AgencyInquiriesScreen(
    inquiries: List<AgencyInquiry>,
    isLoading: Boolean,
    onBackClick: () -> Unit,
    onInquiryClick: (id: String) -> Unit,
) {
    var statusFilter by remember { mutableStateOf<String?>(null) }
    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Box(modifier = Modifier.fillMaxSize()) {
            Column(modifier = Modifier.fillMaxSize()) {
                AgencyInquiriesHeader(onBackClick = onBackClick)
                StatusChips(
                    inquiries = inquiries,
                    selected = statusFilter,
                    onSelect = { statusFilter = it },
                )
                when {
                    isLoading ->
                        Box(
                            modifier = Modifier.fillMaxSize(),
                            contentAlignment = Alignment.Center,
                        ) {
                            CircularProgressIndicator()
                        }
                    inquiries.isEmpty() -> EmptyState()
                    else -> {
                        val filtered = inquiries.filter {
                            statusFilter == null ||
                                it.status.equals(statusFilter, ignoreCase = true)
                        }
                        LazyColumn(
                            modifier = Modifier.fillMaxSize(),
                            contentPadding = PaddingValues(bottom = 110.dp),
                        ) {
                            items(filtered, key = { it.id }) { inquiry ->
                                InquiryRow(
                                    inquiry = inquiry,
                                    onClick = { onInquiryClick(inquiry.id) },
                                )
                            }
                        }
                    }
                }
            }
            ExtendedFloatingActionButton(
                onClick = { /* batch reply */ },
                modifier = Modifier.align(Alignment.BottomEnd).padding(end = 16.dp, bottom = 24.dp),
                containerColor = MaterialTheme.colorScheme.primary,
                contentColor = Color.White,
                icon = { Icon(Icons.AutoMirrored.Filled.Send, contentDescription = null) },
                text = { Text(stringResource(R.string.agency_inquiries_batch_reply)) },
            )
        }
    }
}

@Composable
private fun AgencyInquiriesHeader(onBackClick: () -> Unit) {
    Row(
        modifier =
            Modifier.fillMaxWidth().padding(start = 4.dp, end = 8.dp, top = 14.dp, bottom = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        IconButton(onClick = onBackClick) {
            Icon(
                Icons.AutoMirrored.Filled.ArrowBack,
                contentDescription = stringResource(R.string.cd_back),
            )
        }
        Text(
            text = stringResource(R.string.inquiries),
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.weight(1f),
        )
        IconButton(onClick = { /* search */ }) {
            Icon(Icons.Default.Search, contentDescription = stringResource(R.string.cd_search))
        }
    }
}

@Composable
private fun StatusChips(
    inquiries: List<AgencyInquiry>,
    selected: String?,
    onSelect: (String?) -> Unit,
) {
    val statuses = inquiries.map { it.status }.distinct()
    LazyRow(
        contentPadding = PaddingValues(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier.padding(bottom = 12.dp),
    ) {
        item {
            FilterChipPill(
                label = stringResource(R.string.filter_all),
                count = inquiries.size,
                selected = selected == null,
                onClick = { onSelect(null) },
            )
        }
        items(statuses) { status ->
            val count = inquiries.count { it.status == status }
            FilterChipPill(
                label = status,
                count = count,
                selected = selected == status,
                onClick = { onSelect(status) },
            )
        }
    }
}

@Composable
private fun FilterChipPill(label: String, count: Int, selected: Boolean, onClick: () -> Unit) {
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
private fun InquiryRow(inquiry: AgencyInquiry, onClick: () -> Unit) {
    Box(
        modifier =
            Modifier.fillMaxWidth()
                .background(MaterialTheme.colorScheme.surface)
                .clickable(onClick = onClick)
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.Top,
        ) {
            val initials =
                remember(inquiry.contactName) {
                    inquiry.contactName
                        .split(" ")
                        .take(2)
                        .mapNotNull { it.firstOrNull()?.uppercase() }
                        .joinToString("")
                        .ifEmpty { "?" }
                }
            Box(
                modifier =
                    Modifier.size(44.dp).clip(CircleShape).drawBehind {
                        drawRect(
                            brush =
                                Brush.linearGradient(
                                    colors = listOf(Brand800, Brand500),
                                    start = Offset(0f, 0f),
                                    end = Offset(size.width, size.height),
                                )
                        )
                    },
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = initials,
                    style = MaterialTheme.typography.labelLarge,
                    fontWeight = FontWeight.SemiBold,
                    color = Color.White,
                )
            }
            Spacer(modifier = Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = inquiry.contactName,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(modifier = Modifier.height(3.dp))
                Text(
                    text = inquiry.listingTitle,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.primary,
                    fontWeight = FontWeight.Medium,
                )
                Spacer(modifier = Modifier.height(3.dp))
                Text(
                    text = inquiry.message,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(modifier = Modifier.height(6.dp))
                StatusPill(label = inquiry.status)
            }
        }
        HorizontalDivider(
            modifier = Modifier.align(Alignment.BottomCenter),
            color = MaterialTheme.colorScheme.outlineVariant,
        )
    }
}

@Composable
private fun StatusPill(label: String) {
    Surface(shape = RoundedCornerShape(999.dp), color = MaterialTheme.colorScheme.surfaceVariant) {
        Text(
            text = label.uppercase(),
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
            style =
                MaterialTheme.typography.labelSmall.copy(
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 0.04.sp,
                    fontSize = 10.sp,
                ),
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
        Icon(
            Icons.Default.Inbox,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = stringResource(R.string.agency_inquiries_empty_title),
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.agency_inquiries_empty_body),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/** Lightweight VM-shaped data class fed into the screen. */
data class AgencyInquiry(
    val id: String,
    val listingTitle: String,
    val contactName: String,
    val contactEmail: String,
    val message: String,
    val status: String,
)
