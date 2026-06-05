package three.two.bit.ppt.reality.ui.account

import android.util.Log
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Help
import androidx.compose.material.icons.automirrored.filled.Login
import androidx.compose.material.icons.automirrored.filled.Logout
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
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import coil.request.ImageRequest
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.BuildConfig
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.account.ProfileStats
import three.two.bit.ppt.reality.account.ProfileStatsLoader
import three.two.bit.ppt.reality.api.ApiConfig
import three.two.bit.ppt.reality.auth.AuthState
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.auth.SsoUserInfo
import three.two.bit.ppt.reality.favorites.FavoritesRepository
import three.two.bit.ppt.reality.inquiry.InquiryRepository
import three.two.bit.ppt.reality.notifications.NotificationPreferences
import three.two.bit.ppt.reality.notifications.NotificationRepository
import three.two.bit.ppt.reality.ui.theme.Brand500
import three.two.bit.ppt.reality.ui.theme.Brand800

private const val TAG = "AccountScreen"

/**
 * Account / Profile screen — KMP / Compose M3 redesign matching the design bundle
 * (`guest-registration-v2-design-system/project/ui_kits/mobile-native/ screens.jsx` →
 * `KmpProfileScreen`).
 *
 * Layout (top → bottom):
 * 1. Top bar — large title "Profil" + settings cog (right).
 * 2. Hero — 88dp avatar (gradient circle with initials), name + verified pill, email; centered.
 * 3. 3-card stat strip — Favorites / Searches / Inquiries with large numeric values and uppercase
 *    labels.
 * 4. Section list — single rounded card grouping rows: Saved searches, Comparison, Notifications
 *    (toggle), Privacy, About. Each row uses a 40dp icon tile (surface-variant fill) + title +
 *    sub + chev / switch.
 * 5. Notification preferences sub-section — preserved from previous implementation. Lives under the
 *    section card and is hidden behind the master Notifications toggle. Reveals the detailed list
 *    of preferences (new listings, price drops, inquiry responses, listing updates, marketing) so
 *    existing behavior is not lost while the primary visual surface matches the design.
 * 6. Sign out card — danger-tinted icon tile + label, full-width.
 *
 * Epic 48 - Story 48.5: Portal Mobile Account.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AccountScreen(
    ssoService: SsoService,
    onBackClick: () -> Unit,
    onLogout: () -> Unit,
    onProfileEditClick: () -> Unit = {},
    onSavedSearchesClick: () -> Unit = {},
    onSignInClick: () -> Unit = {},
) {
    val scope = rememberCoroutineScope()
    val authState by ssoService.authState.collectAsState()

    var showLogoutDialog by remember { mutableStateOf(false) }
    var notificationPrefs by remember { mutableStateOf<NotificationPreferences?>(null) }
    var notificationsExpanded by remember { mutableStateOf(false) }
    var profileStats by remember { mutableStateOf(ProfileStats.UNKNOWN) }

    val notificationRepository =
        remember(authState) {
            val token = (authState as? AuthState.Authenticated)?.sessionToken
            NotificationRepository(baseUrl = ApiConfig.requireBaseUrl(), sessionToken = token)
        }
    val statsLoader =
        remember(authState) {
            val token = (authState as? AuthState.Authenticated)?.sessionToken
            val baseUrl = ApiConfig.requireBaseUrl()
            ProfileStatsLoader(
                favoritesRepository = FavoritesRepository(baseUrl = baseUrl, sessionToken = token),
                inquiryRepository = InquiryRepository(baseUrl = baseUrl, sessionToken = token),
            )
        }

    LaunchedEffect(authState) {
        if (authState is AuthState.Authenticated) {
            notificationRepository
                .getPreferences()
                .fold(
                    onSuccess = { prefs -> notificationPrefs = prefs },
                    onFailure = { error ->
                        Log.e(TAG, "Failed to load notification preferences", error)
                    },
                )
            profileStats = statsLoader.load()
        } else {
            profileStats = ProfileStats.UNKNOWN
        }
    }

    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        when (val state = authState) {
            is AuthState.Unauthenticated,
            is AuthState.Error -> {
                NotSignedInContent(onSignInClick = onSignInClick)
            }
            is AuthState.Loading -> {
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            }
            is AuthState.Authenticated -> {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(bottom = 100.dp),
                ) {
                    item { ProfileTopBar(onEditProfileClick = onProfileEditClick) }
                    item { ProfileHero(user = state.user) }
                    item { Spacer(modifier = Modifier.height(8.dp)) }
                    // Stats are derived from the `total` field of the favorites,
                    // saved-searches, and inquiries list endpoints (reality-server has
                    // no single /me/stats endpoint). A failed call leaves that field
                    // null so StatCard renders an em-dash instead of a fake 0.
                    item {
                        StatsRow(
                            favorites = profileStats.favorites,
                            searches = profileStats.searches,
                            inquiries = profileStats.inquiries,
                        )
                    }
                    item { Spacer(modifier = Modifier.height(24.dp)) }
                    item {
                        SectionList(
                            notificationsEnabled =
                                notificationPrefs?.let {
                                    it.newListings ||
                                        it.priceDrops ||
                                        it.inquiryResponses ||
                                        it.listingUpdates ||
                                        it.marketing
                                } ?: false,
                            onNotificationsToggle = {
                                notificationsExpanded = !notificationsExpanded
                            },
                            notificationsExpanded = notificationsExpanded,
                            onSavedSearchesClick = onSavedSearchesClick,
                        )
                    }
                    if (notificationsExpanded && notificationPrefs != null) {
                        item {
                            NotificationPreferencesCard(
                                preferences = notificationPrefs!!,
                                onPreferenceChange = { newPrefs ->
                                    scope.launch {
                                        notificationRepository
                                            .updatePreferences(newPrefs)
                                            .fold(
                                                onSuccess = { notificationPrefs = it },
                                                onFailure = {
                                                    Log.e(
                                                        TAG,
                                                        "Failed to update notification preferences",
                                                        it,
                                                    )
                                                },
                                            )
                                    }
                                },
                            )
                        }
                    }
                    item { Spacer(modifier = Modifier.height(8.dp)) }
                    item { AppSettingsCard() }
                    item { Spacer(modifier = Modifier.height(8.dp)) }
                    item { AboutCard() }
                    item { Spacer(modifier = Modifier.height(12.dp)) }
                    item { SignOutCard(onClick = { showLogoutDialog = true }) }
                    item { Spacer(modifier = Modifier.height(16.dp)) }
                }
            }
        }
    }

    if (showLogoutDialog) {
        AlertDialog(
            onDismissRequest = { showLogoutDialog = false },
            title = { Text(stringResource(R.string.sign_out)) },
            text = { Text(stringResource(R.string.confirm_sign_out_message)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        showLogoutDialog = false
                        onLogout()
                    },
                    colors =
                        ButtonDefaults.textButtonColors(
                            contentColor = MaterialTheme.colorScheme.error
                        ),
                ) {
                    Text(stringResource(R.string.sign_out))
                }
            },
            dismissButton = {
                TextButton(onClick = { showLogoutDialog = false }) {
                    Text(stringResource(R.string.cancel))
                }
            },
        )
    }
}

// ─── Top bar ───────────────────────────────────────────────

@Composable
private fun ProfileTopBar(onEditProfileClick: () -> Unit) {
    Row(
        modifier =
            Modifier.fillMaxWidth().padding(start = 16.dp, end = 8.dp, top = 14.dp, bottom = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = stringResource(R.string.profile_title),
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.weight(1f),
        )
        IconButton(onClick = onEditProfileClick) {
            Icon(
                Icons.Default.Edit,
                contentDescription = stringResource(R.string.edit_profile),
                tint = MaterialTheme.colorScheme.onSurface,
            )
        }
    }
}

// ─── Hero ────────────────────────────────────────────────

@Composable
private fun ProfileHero(user: SsoUserInfo) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        val initials =
            remember(user) {
                user.name
                    .split(" ")
                    .take(2)
                    .mapNotNull { it.firstOrNull()?.uppercase() }
                    .joinToString("")
                    .ifEmpty { user.email.take(2).uppercase() }
            }
        if (user.avatarUrl != null) {
            AsyncImage(
                model =
                    ImageRequest.Builder(LocalContext.current)
                        .data(user.avatarUrl)
                        .crossfade(true)
                        .build(),
                contentDescription = user.name,
                contentScale = ContentScale.Crop,
                modifier =
                    Modifier.size(88.dp)
                        .clip(CircleShape)
                        .background(MaterialTheme.colorScheme.surfaceVariant),
            )
        } else {
            Box(
                modifier =
                    Modifier.size(88.dp).clip(CircleShape).drawBehind {
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
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = Color.White,
                )
            }
        }
        Spacer(modifier = Modifier.height(14.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = user.name,
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(modifier = Modifier.width(6.dp))
            VerifiedPill()
        }
        Spacer(modifier = Modifier.height(2.dp))
        Text(
            text = user.email,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun VerifiedPill() {
    Surface(shape = RoundedCornerShape(999.dp), color = Color(0xFFD1FAE5)) {
        Row(
            modifier = Modifier.padding(horizontal = 7.dp, vertical = 2.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                Icons.Default.Check,
                contentDescription = null,
                tint = Color(0xFF065F46),
                modifier = Modifier.size(10.dp),
            )
            Spacer(modifier = Modifier.width(2.dp))
            Text(
                text = stringResource(R.string.profile_verified).uppercase(),
                style =
                    MaterialTheme.typography.labelSmall.copy(
                        fontWeight = FontWeight.Bold,
                        letterSpacing = 0.04.sp,
                        fontSize = 10.sp,
                    ),
                color = Color(0xFF065F46),
            )
        }
    }
}

// ─── Stats row ─────────────────────────────────────────────

@Composable
private fun StatsRow(favorites: Int?, searches: Int?, inquiries: Int?) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        StatCard(
            value = favorites?.toString() ?: "—",
            label = stringResource(R.string.profile_stat_favorites),
            modifier = Modifier.weight(1f),
        )
        StatCard(
            value = searches?.toString() ?: "—",
            label = stringResource(R.string.profile_stat_searches),
            modifier = Modifier.weight(1f),
        )
        StatCard(
            value = inquiries?.toString() ?: "—",
            label = stringResource(R.string.profile_stat_inquiries),
            modifier = Modifier.weight(1f),
        )
    }
}

@Composable
private fun StatCard(value: String, label: String, modifier: Modifier = Modifier) {
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(vertical = 14.dp, horizontal = 6.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = value,
                style = MaterialTheme.typography.headlineSmall,
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
                    ),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// ─── Section list ──────────────────────────────────────────

@Composable
private fun SectionList(
    notificationsEnabled: Boolean,
    onNotificationsToggle: () -> Unit,
    notificationsExpanded: Boolean,
    onSavedSearchesClick: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column {
            SectionRow(
                icon = Icons.Default.Search,
                title = stringResource(R.string.profile_section_searches),
                subtitle = stringResource(R.string.profile_section_searches_sub),
                trailing = SectionTrailing.Chevron,
                onClick = onSavedSearchesClick,
            )
            SectionDivider()
            SectionRow(
                icon = Icons.Default.Compare,
                title = stringResource(R.string.profile_section_compare),
                subtitle = stringResource(R.string.profile_section_compare_sub),
                trailing = SectionTrailing.Chevron,
                onClick = { /* nav to compare */ },
            )
            SectionDivider()
            SectionRow(
                icon = Icons.Default.NotificationsNone,
                title = stringResource(R.string.profile_section_notifications),
                subtitle = stringResource(R.string.profile_section_notifications_sub),
                trailing = SectionTrailing.Toggle(notificationsEnabled),
                onClick = onNotificationsToggle,
            )
            SectionDivider()
            SectionRow(
                icon = Icons.Default.Shield,
                title = stringResource(R.string.profile_section_privacy),
                subtitle = null,
                trailing = SectionTrailing.Chevron,
                onClick = { /* nav to privacy */ },
            )
            SectionDivider()
            SectionRow(
                icon = Icons.Default.Info,
                title = stringResource(R.string.profile_section_about),
                subtitle =
                    stringResource(R.string.profile_section_about_sub, BuildConfig.VERSION_NAME),
                trailing = SectionTrailing.Chevron,
                onClick = { /* nav to about */ },
            )
        }
    }
}

private sealed class SectionTrailing {
    object Chevron : SectionTrailing()

    data class Toggle(val checked: Boolean) : SectionTrailing()
}

@Composable
private fun SectionRow(
    icon: ImageVector,
    title: String,
    subtitle: String?,
    trailing: SectionTrailing,
    onClick: () -> Unit,
) {
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
            subtitle?.let {
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        when (trailing) {
            is SectionTrailing.Chevron ->
                Icon(
                    Icons.Default.ChevronRight,
                    contentDescription = null,
                    modifier = Modifier.size(18.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            is SectionTrailing.Toggle ->
                Switch(checked = trailing.checked, onCheckedChange = { onClick() })
        }
    }
}

@Composable
private fun SectionDivider() {
    HorizontalDivider(
        modifier = Modifier.padding(start = 70.dp),
        thickness = 1.dp,
        color = MaterialTheme.colorScheme.outlineVariant,
    )
}

// ─── Notification preferences (collapsible) ──────────────────────

@Composable
private fun NotificationPreferencesCard(
    preferences: NotificationPreferences,
    onPreferenceChange: (NotificationPreferences) -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = stringResource(R.string.notifications),
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
            )
            Spacer(modifier = Modifier.height(12.dp))
            NotificationPreferenceItem(
                title = stringResource(R.string.notification_new_listings),
                description = stringResource(R.string.notification_new_listings_desc),
                checked = preferences.newListings,
                onCheckedChange = { onPreferenceChange(preferences.copy(newListings = it)) },
            )
            HorizontalDivider()
            NotificationPreferenceItem(
                title = stringResource(R.string.notification_price_drops),
                description = stringResource(R.string.notification_price_drops_desc),
                checked = preferences.priceDrops,
                onCheckedChange = { onPreferenceChange(preferences.copy(priceDrops = it)) },
            )
            HorizontalDivider()
            NotificationPreferenceItem(
                title = stringResource(R.string.notification_inquiry_responses),
                description = stringResource(R.string.notification_inquiry_responses_desc),
                checked = preferences.inquiryResponses,
                onCheckedChange = { onPreferenceChange(preferences.copy(inquiryResponses = it)) },
            )
            HorizontalDivider()
            NotificationPreferenceItem(
                title = stringResource(R.string.notification_listing_updates),
                description = stringResource(R.string.notification_listing_updates_desc),
                checked = preferences.listingUpdates,
                onCheckedChange = { onPreferenceChange(preferences.copy(listingUpdates = it)) },
            )
            HorizontalDivider()
            NotificationPreferenceItem(
                title = stringResource(R.string.notification_marketing),
                description = stringResource(R.string.notification_marketing_desc),
                checked = preferences.marketing,
                onCheckedChange = { onPreferenceChange(preferences.copy(marketing = it)) },
            )
        }
    }
}

@Composable
private fun NotificationPreferenceItem(
    title: String,
    description: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(text = title, style = MaterialTheme.typography.bodyLarge)
            Text(
                text = description,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Switch(checked = checked, onCheckedChange = onCheckedChange)
    }
}

// ─── App settings + About (preserved, restyled) ────────────────────────

@Composable
private fun AppSettingsCard() {
    Card(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column {
            SettingsRow(
                icon = Icons.Default.Language,
                title = stringResource(R.string.setting_language),
                value = stringResource(R.string.language_english),
                onClick = { /* picker */ },
            )
            SectionDivider()
            SettingsRow(
                icon = Icons.Default.Euro,
                title = stringResource(R.string.setting_currency),
                value = stringResource(R.string.currency_eur),
                onClick = { /* picker */ },
            )
            SectionDivider()
            SettingsRow(
                icon = Icons.Default.Straighten,
                title = stringResource(R.string.setting_units),
                value = stringResource(R.string.units_metric),
                onClick = { /* picker */ },
            )
            SectionDivider()
            SettingsRow(
                icon = Icons.Default.DarkMode,
                title = stringResource(R.string.setting_theme),
                value = stringResource(R.string.theme_system),
                onClick = { /* picker */ },
            )
        }
    }
}

@Composable
private fun SettingsRow(icon: ImageVector, title: String, value: String, onClick: () -> Unit) {
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
        Text(
            text = title,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.weight(1f),
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.width(6.dp))
        Icon(
            Icons.Default.ChevronRight,
            contentDescription = null,
            modifier = Modifier.size(18.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun AboutCard() {
    Card(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column {
            AboutRow(
                icon = Icons.Default.Description,
                title = stringResource(R.string.terms_of_service),
                onClick = { /* open terms */ },
            )
            SectionDivider()
            AboutRow(
                icon = Icons.Default.PrivacyTip,
                title = stringResource(R.string.privacy_policy),
                onClick = { /* open privacy */ },
            )
            SectionDivider()
            AboutRow(
                icon = Icons.AutoMirrored.Filled.Help,
                title = stringResource(R.string.help_support),
                onClick = { /* open help */ },
            )
            SectionDivider()
            AboutRow(
                icon = Icons.Default.Feedback,
                title = stringResource(R.string.send_feedback),
                onClick = { /* open feedback */ },
            )
        }
    }
}

@Composable
private fun AboutRow(icon: ImageVector, title: String, onClick: () -> Unit) {
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
        Text(
            text = title,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.weight(1f),
        )
        Icon(
            Icons.Default.ChevronRight,
            contentDescription = null,
            modifier = Modifier.size(18.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

// ─── Sign out card ─────────────────────────────────────────

@Composable
private fun SignOutCard(onClick: () -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp).clickable(onClick = onClick),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                modifier =
                    Modifier.size(40.dp)
                        .clip(RoundedCornerShape(10.dp))
                        .background(MaterialTheme.colorScheme.errorContainer),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    Icons.AutoMirrored.Filled.Logout,
                    contentDescription = null,
                    modifier = Modifier.size(20.dp),
                    tint = MaterialTheme.colorScheme.onErrorContainer,
                )
            }
            Spacer(modifier = Modifier.width(14.dp))
            Text(
                text = stringResource(R.string.sign_out),
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onErrorContainer,
                modifier = Modifier.weight(1f),
            )
            Icon(
                Icons.Default.ChevronRight,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// ─── Not signed in ─────────────────────────────────────────

@Composable
private fun NotSignedInContent(onSignInClick: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Icon(
            Icons.Default.AccountCircle,
            contentDescription = null,
            modifier = Modifier.size(80.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(24.dp))
        Text(
            text = stringResource(R.string.sign_in_title),
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.Bold,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.sign_in_benefits),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(32.dp))
        Button(
            onClick = onSignInClick,
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(14.dp),
        ) {
            Icon(
                Icons.AutoMirrored.Filled.Login,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text(stringResource(R.string.sign_in_pm_app))
        }
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = stringResource(R.string.sign_in_redirect_notice),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
