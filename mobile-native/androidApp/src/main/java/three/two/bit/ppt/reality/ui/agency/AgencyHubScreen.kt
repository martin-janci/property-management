package three.two.bit.ppt.reality.ui.agency

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.ui.theme.Brand500
import three.two.bit.ppt.reality.ui.theme.Brand800

/**
 * Agency hub screen — KMP / Compose M3 redesign matching the design (`KmpAgencyHubScreen`). Agency
 * identity row (gradient RP square + "Agentúra · Reality Profi s.r.o." + bell with red dot), 2×2
 * stats grid (Active listings / New inquiries [accent] / Avg response / Conversion) with delta
 * deltas, "Recent activity" section card with leading-icon tiles + status pills, FAB "Add listing".
 *
 * UC-49 — Agency management.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AgencyHubScreen(
    onBackClick: () -> Unit,
    onInquiriesClick: () -> Unit,
    onMyListingsClick: () -> Unit,
    onCreateListingClick: () -> Unit,
    onAnalyticsClick: () -> Unit,
    onNotificationsClick: () -> Unit = onInquiriesClick,
) {
    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Box(modifier = Modifier.fillMaxSize()) {
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(bottom = 100.dp),
            ) {
                item { AgencyHeader(onNotificationsClick = onNotificationsClick) }
                item {
                    StatsGrid(
                        onInquiriesClick = onInquiriesClick,
                        onAnalyticsClick = onAnalyticsClick,
                    )
                }
                item { Spacer(modifier = Modifier.height(8.dp)) }
                item {
                    SectionTitle(
                        text = stringResource(R.string.agency_actions_title),
                        action = null,
                    )
                }
                item {
                    AgencyActions(
                        onInquiriesClick = onInquiriesClick,
                        onMyListingsClick = onMyListingsClick,
                        onCreateListingClick = onCreateListingClick,
                        onAnalyticsClick = onAnalyticsClick,
                    )
                }
                item { Spacer(modifier = Modifier.height(8.dp)) }
                item {
                    SectionTitle(
                        text = stringResource(R.string.agency_activity_title),
                        action = stringResource(R.string.agency_activity_see_all),
                    )
                }
                items(activitySamples()) { activity -> ActivityRow(activity = activity) }
            }
            // FAB
            ExtendedFloatingActionButton(
                onClick = onCreateListingClick,
                modifier = Modifier.align(Alignment.BottomEnd).padding(end = 16.dp, bottom = 24.dp),
                containerColor = MaterialTheme.colorScheme.primary,
                contentColor = Color.White,
                icon = { Icon(Icons.Default.Add, contentDescription = null) },
                text = { Text(stringResource(R.string.agency_add_listing)) },
            )
        }
    }
}

@Composable
private fun AgencyHeader(onNotificationsClick: () -> Unit) {
    Row(
        modifier =
            Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 14.dp, bottom = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier =
                Modifier.size(40.dp).clip(RoundedCornerShape(10.dp)).drawBehind {
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
            Text(text = "RP", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 16.sp)
        }
        Spacer(modifier = Modifier.width(10.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = stringResource(R.string.agency_role_label).uppercase(),
                style =
                    MaterialTheme.typography.labelSmall.copy(
                        fontWeight = FontWeight.Bold,
                        letterSpacing = 0.06.sp,
                    ),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                text = stringResource(R.string.agency_company_placeholder),
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface,
            )
        }
        Box(modifier = Modifier.size(40.dp), contentAlignment = Alignment.Center) {
            IconButton(onClick = onNotificationsClick) {
                Icon(
                    Icons.Default.Notifications,
                    contentDescription = stringResource(R.string.cd_notifications),
                    tint = MaterialTheme.colorScheme.onSurface,
                )
            }
            Box(
                modifier =
                    Modifier.padding(top = 8.dp, end = 8.dp)
                        .size(8.dp)
                        .clip(CircleShape)
                        .background(MaterialTheme.colorScheme.error)
                        .align(Alignment.TopEnd)
            )
        }
    }
}

@Composable
private fun StatsGrid(onInquiriesClick: () -> Unit, onAnalyticsClick: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            StatCard(
                value = "42",
                label = stringResource(R.string.agency_stat_active),
                delta = "+3",
                accent = false,
                modifier = Modifier.weight(1f).clickable(onClick = onAnalyticsClick),
            )
            StatCard(
                value = "18",
                label = stringResource(R.string.agency_stat_new_inquiries),
                delta = "+5",
                accent = true,
                modifier = Modifier.weight(1f).clickable(onClick = onInquiriesClick),
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            StatCard(
                value = "2.4h",
                label = stringResource(R.string.agency_stat_response),
                delta = "-12%",
                accent = false,
                modifier = Modifier.weight(1f),
            )
            StatCard(
                value = "14%",
                label = stringResource(R.string.agency_stat_conversion),
                delta = "+1.2%",
                accent = false,
                modifier = Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun StatCard(
    value: String,
    label: String,
    delta: String,
    accent: Boolean,
    modifier: Modifier = Modifier,
) {
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(12.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors =
            CardDefaults.cardColors(
                containerColor =
                    if (accent) MaterialTheme.colorScheme.primary
                    else MaterialTheme.colorScheme.surface
            ),
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            Text(
                text = label,
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.SemiBold,
                color =
                    if (accent) Color.White.copy(alpha = 0.85f)
                    else MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(6.dp))
            Row(verticalAlignment = Alignment.Bottom) {
                Text(
                    text = value,
                    style = MaterialTheme.typography.headlineMedium.copy(fontSize = 26.sp),
                    fontWeight = FontWeight.Bold,
                    color = if (accent) Color.White else MaterialTheme.colorScheme.onSurface,
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = delta,
                    style = MaterialTheme.typography.labelSmall,
                    fontWeight = FontWeight.SemiBold,
                    color = if (accent) Color.White.copy(alpha = 0.9f) else Color(0xFF10B981),
                )
            }
        }
    }
}

@Composable
private fun SectionTitle(text: String, action: String?) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.weight(1f),
        )
        action?.let {
            Text(
                text = it,
                style = MaterialTheme.typography.labelMedium,
                fontWeight = FontWeight.Medium,
                color = MaterialTheme.colorScheme.primary,
            )
        }
    }
}

@Composable
private fun AgencyActions(
    onInquiriesClick: () -> Unit,
    onMyListingsClick: () -> Unit,
    onCreateListingClick: () -> Unit,
    onAnalyticsClick: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
        shape = RoundedCornerShape(12.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column {
            ActionRow(
                icon = Icons.Default.Email,
                title = stringResource(R.string.agency_section_inquiries),
                subtitle = stringResource(R.string.agency_section_inquiries_sub),
                onClick = onInquiriesClick,
            )
            ActionDivider()
            ActionRow(
                icon = Icons.Default.Home,
                title = stringResource(R.string.agency_section_listings),
                subtitle = stringResource(R.string.agency_section_listings_sub),
                onClick = onMyListingsClick,
            )
            ActionDivider()
            ActionRow(
                icon = Icons.Default.Add,
                title = stringResource(R.string.agency_section_create),
                subtitle = stringResource(R.string.agency_section_create_sub),
                onClick = onCreateListingClick,
            )
            ActionDivider()
            ActionRow(
                icon = Icons.Default.QueryStats,
                title = stringResource(R.string.agency_section_analytics),
                subtitle = stringResource(R.string.agency_section_analytics_sub),
                onClick = onAnalyticsClick,
            )
        }
    }
}

@Composable
private fun ActionRow(icon: ImageVector, title: String, subtitle: String, onClick: () -> Unit) {
    Row(
        modifier =
            Modifier.fillMaxWidth()
                .clickable(onClick = onClick)
                .padding(horizontal = 16.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier =
                Modifier.size(40.dp)
                    .clip(RoundedCornerShape(10.dp))
                    .background(MaterialTheme.colorScheme.surfaceVariant),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                modifier = Modifier.size(20.dp),
                tint = MaterialTheme.colorScheme.onSurface,
            )
        }
        Spacer(modifier = Modifier.width(14.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Icon(
            Icons.Default.ChevronRight,
            contentDescription = null,
            modifier = Modifier.size(18.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun ActionDivider() {
    HorizontalDivider(
        modifier = Modifier.padding(start = 70.dp),
        color = MaterialTheme.colorScheme.outlineVariant,
    )
}

@Composable
private fun ActivityRow(activity: ActivitySample) {
    Row(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp)) {
        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp),
            elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        ) {
            Row(modifier = Modifier.padding(14.dp), verticalAlignment = Alignment.Top) {
                Box(
                    modifier =
                        Modifier.size(36.dp)
                            .clip(RoundedCornerShape(10.dp))
                            .background(MaterialTheme.colorScheme.surfaceVariant),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = activity.icon,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.size(18.dp),
                    )
                }
                Spacer(modifier = Modifier.width(12.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = activity.title,
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.SemiBold,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Text(
                        text = activity.subtitle,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                activity.tag?.let { tag ->
                    Surface(shape = RoundedCornerShape(999.dp), color = tag.bg) {
                        Text(
                            text = tag.label.uppercase(),
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                            style =
                                MaterialTheme.typography.labelSmall.copy(
                                    fontWeight = FontWeight.Bold,
                                    letterSpacing = 0.04.sp,
                                    fontSize = 10.sp,
                                ),
                            color = tag.ink,
                        )
                    }
                }
            }
        }
    }
}

private data class ActivitySample(
    val icon: ImageVector,
    val title: String,
    val subtitle: String,
    val tag: ActivityTag?,
)

private data class ActivityTag(val label: String, val bg: Color, val ink: Color)

private fun activitySamples(): List<ActivitySample> =
    listOf(
        ActivitySample(
            icon = Icons.Default.Email,
            title = "New inquiry on 3-room · Ružinov",
            subtitle = "Katarína Molnár · 4 min ago",
            tag = ActivityTag("New", Color(0xFFDBEAFE), Color(0xFF1E40AF)),
        ),
        ActivitySample(
            icon = Icons.Default.Visibility,
            title = "Listing Penthouse · Nové Mesto",
            subtitle = "12 new views · 2h ago",
            tag = null,
        ),
        ActivitySample(
            icon = Icons.Default.CheckCircle,
            title = "Viewing confirmed · 2-room Petržalka",
            subtitle = "Peter Varga · Tue 16:00",
            tag = ActivityTag("Confirmed", Color(0xFFD1FAE5), Color(0xFF065F46)),
        ),
        ActivitySample(
            icon = Icons.Default.Favorite,
            title = "New favorite · 4-room Dúbravka",
            subtitle = "3 favorites · today",
            tag = null,
        ),
        ActivitySample(
            icon = Icons.Default.Flag,
            title = "Listing Studio · expires in 2 days",
            subtitle = "Extend or edit",
            tag = ActivityTag("Action", Color(0xFFFEF3C7), Color(0xFF92400E)),
        ),
    )
