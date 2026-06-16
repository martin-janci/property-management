package three.two.bit.ppt.reality.ui.inquiries

import android.util.Log
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
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
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import coil.request.ImageRequest
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.api.ApiConfig
import three.two.bit.ppt.reality.auth.AuthState
import three.two.bit.ppt.reality.auth.GuardDecision
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.auth.authGuardDecision
import three.two.bit.ppt.reality.inquiry.*
import three.two.bit.ppt.reality.listing.ListingRepository
import three.two.bit.ppt.reality.ui.theme.Brand500
import three.two.bit.ppt.reality.ui.theme.Brand800
import three.two.bit.ppt.reality.ui.theme.InquiryStatusColors
import three.two.bit.ppt.reality.ui.theme.ViewingStatusColors
import three.two.bit.ppt.reality.util.isNetworkError

private const val TAG = "InquiriesScreen"

/**
 * Inquiries screen — KMP / Compose M3 redesign matching the design bundle
 * (`guest-registration-v2-design-system/project/ui_kits/mobile-native/ screens.jsx` →
 * `KmpInquiriesScreen`).
 *
 * Layout (top → bottom):
 * 1. Large-title header "Dopyty".
 * 2. Status filter chip strip (All · count, Pending, Responded, Closed) — horizontal scroll.
 * 3. Tab row — Messages / Viewings (preserved, restyled as flat M3 TabRow). The KmpInquiriesScreen
 *    design only mocks the messages list; we keep viewings as a sibling tab so wired functionality
 *    (cancel viewing dialog) is not lost.
 * 4. Messages tab — flat list of thread rows (no card chrome). Each row: optional 3dp left unread
 *    bar, 48dp gradient avatar with the realtor initials, 22dp listing thumbnail overlay at
 *    bottom-right of the avatar, name (bold when unread) + relative time right- aligned, 2-line
 *    message preview, status uppercase pill.
 * 5. Viewings tab — restyled card list (icon + property + date/time + message + status pill +
 *    cancel action).
 * 6. Bottom nav reserved 96dp.
 *
 * Inline thread view (`KmpInquiryThreadScreen` in the design) is not wired here — the design
 * includes scheduling UI (calendar grid + time slots) that requires API support not yet present in
 * the inquiry domain. Tracked as a follow-up.
 *
 * Epic 48 - Story 48.6: Portal Mobile Inquiries.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InquiriesScreen(
    repository: ListingRepository,
    ssoService: SsoService,
    onListingClick: (String) -> Unit,
    onBackClick: () -> Unit,
    onSignInClick: () -> Unit = {},
) {
    val scope = rememberCoroutineScope()
    val authState by ssoService.authState.collectAsState()
    val networkErrorMsg = stringResource(R.string.error_network)
    val genericErrorMsg = stringResource(R.string.error_generic)
    fun friendlyError(e: Throwable) = if (e.isNetworkError()) networkErrorMsg else genericErrorMsg

    var selectedTab by remember { mutableIntStateOf(0) }
    var statusFilter by remember { mutableStateOf<InquiryStatus?>(null) }
    var inquiries by remember { mutableStateOf<List<Inquiry>>(emptyList()) }
    var viewings by remember { mutableStateOf<List<ViewingRequest>>(emptyList()) }
    var isLoading by remember { mutableStateOf(true) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    val inquiryRepository =
        remember(authState) {
            val token = (authState as? AuthState.Authenticated)?.sessionToken
            InquiryRepository(baseUrl = ApiConfig.requireBaseUrl(), sessionToken = token)
        }

    LaunchedEffect(authState) {
        if (authState is AuthState.Authenticated) {
            isLoading = true
            errorMessage = null
            inquiryRepository
                .getInquiries()
                .fold(
                    onSuccess = { inquiries = it.inquiries },
                    onFailure = { errorMessage = friendlyError(it) },
                )
            inquiryRepository
                .getViewings()
                .fold(
                    onSuccess = { viewings = it.viewings },
                    onFailure = { Log.e(TAG, "Failed to load viewings", it) },
                )
            isLoading = false
        } else {
            isLoading = false
        }
    }

    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        when (authGuardDecision(authState)) {
            GuardDecision.NOT_SIGNED_IN -> NotSignedInContent(onSignInClick = onSignInClick)
            GuardDecision.LOADING ->
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            GuardDecision.CONTENT -> {
                Column(modifier = Modifier.fillMaxSize()) {
                    InquiriesHeader()
                    StatusFilterStrip(
                        inquiries = inquiries,
                        selected = statusFilter,
                        onSelect = { statusFilter = it },
                    )
                    TabRow(
                        selectedTabIndex = selectedTab,
                        containerColor = MaterialTheme.colorScheme.background,
                        contentColor = MaterialTheme.colorScheme.primary,
                    ) {
                        Tab(
                            selected = selectedTab == 0,
                            onClick = { selectedTab = 0 },
                            text = { Text(stringResource(R.string.inquiries_tab_messages)) },
                        )
                        Tab(
                            selected = selectedTab == 1,
                            onClick = { selectedTab = 1 },
                            text = { Text(stringResource(R.string.inquiries_tab_viewings)) },
                        )
                    }
                    when {
                        isLoading ->
                            Box(
                                modifier = Modifier.fillMaxSize(),
                                contentAlignment = Alignment.Center,
                            ) {
                                CircularProgressIndicator()
                            }
                        errorMessage != null ->
                            ErrorContent(
                                message = errorMessage!!,
                                onRetry = {
                                    scope.launch {
                                        isLoading = true
                                        errorMessage = null
                                        inquiryRepository
                                            .getInquiries()
                                            .fold(
                                                onSuccess = { inquiries = it.inquiries },
                                                onFailure = { errorMessage = friendlyError(it) },
                                            )
                                        isLoading = false
                                    }
                                },
                            )
                        selectedTab == 0 ->
                            InquiriesList(
                                inquiries =
                                    inquiries.filter {
                                        statusFilter == null || it.status == statusFilter
                                    },
                                onInquiryClick = { onListingClick(it.listingId) },
                            )
                        selectedTab == 1 ->
                            ViewingsList(
                                viewings = viewings,
                                onViewingClick = { onListingClick(it.listingId) },
                                onCancelViewing = { viewingId ->
                                    scope.launch {
                                        inquiryRepository
                                            .cancelViewing(viewingId)
                                            .fold(
                                                onSuccess = {
                                                    viewings =
                                                        viewings.filter { it.id != viewingId }
                                                },
                                                onFailure = {
                                                    Log.e(TAG, "Failed to cancel viewing", it)
                                                    errorMessage = friendlyError(it)
                                                },
                                            )
                                    }
                                },
                            )
                    }
                }
            }
        }
    }
}

// ─── Header + filter strip ──────────────────────────────────────────────────

@Composable
private fun InquiriesHeader() {
    Text(
        text = stringResource(R.string.inquiries),
        modifier =
            Modifier.fillMaxWidth()
                .padding(start = 16.dp, end = 16.dp, top = 14.dp, bottom = 12.dp),
        style = MaterialTheme.typography.headlineSmall,
        fontWeight = FontWeight.Bold,
        color = MaterialTheme.colorScheme.onSurface,
    )
}

@Composable
private fun StatusFilterStrip(
    inquiries: List<Inquiry>,
    selected: InquiryStatus?,
    onSelect: (InquiryStatus?) -> Unit,
) {
    val chips =
        listOf<Pair<InquiryStatus?, Int>>(
            null to inquiries.size,
            InquiryStatus.PENDING to inquiries.count { it.status == InquiryStatus.PENDING },
            InquiryStatus.RESPONDED to inquiries.count { it.status == InquiryStatus.RESPONDED },
            InquiryStatus.CLOSED to inquiries.count { it.status == InquiryStatus.CLOSED },
        )
    LazyRow(
        contentPadding = PaddingValues(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier.fillMaxWidth().padding(bottom = 8.dp),
    ) {
        items(chips.size) { idx ->
            val (status, count) = chips[idx]
            val isOn = selected == status
            val label =
                status?.let {
                    when (it) {
                        InquiryStatus.PENDING -> stringResource(R.string.status_pending)
                        InquiryStatus.RESPONDED -> stringResource(R.string.status_responded)
                        InquiryStatus.CLOSED -> stringResource(R.string.status_closed)
                    }
                } ?: stringResource(R.string.filter_all)
            Surface(
                onClick = { onSelect(status) },
                shape = RoundedCornerShape(8.dp),
                color = if (isOn) MaterialTheme.colorScheme.primaryContainer else Color.Transparent,
                border =
                    if (isOn) null
                    else
                        androidx.compose.foundation.BorderStroke(
                            1.dp,
                            MaterialTheme.colorScheme.outline,
                        ),
            ) {
                Row(
                    modifier = Modifier.padding(horizontal = 14.dp, vertical = 7.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = label,
                        style = MaterialTheme.typography.labelMedium,
                        fontWeight = FontWeight.Medium,
                        color =
                            if (isOn) MaterialTheme.colorScheme.onPrimaryContainer
                            else MaterialTheme.colorScheme.onSurface,
                    )
                    if (count > 0) {
                        Text(
                            text = " · $count",
                            style = MaterialTheme.typography.labelMedium,
                            color =
                                if (isOn)
                                    MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.7f)
                                else MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }
}

// ─── Inquiries (messages) list ───────────────────────────────────────────────

@Composable
private fun InquiriesList(inquiries: List<Inquiry>, onInquiryClick: (Inquiry) -> Unit) {
    if (inquiries.isEmpty()) {
        EmptyInquiries()
        return
    }
    LazyColumn(modifier = Modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = 96.dp)) {
        items(inquiries, key = { it.id }) { inquiry ->
            InquiryRow(inquiry = inquiry, onClick = { onInquiryClick(inquiry) })
        }
    }
}

@Composable
private fun InquiryRow(inquiry: Inquiry, onClick: () -> Unit) {
    val isUnread = inquiry.status == InquiryStatus.RESPONDED
    Box(
        modifier =
            Modifier.fillMaxWidth()
                .background(MaterialTheme.colorScheme.surface)
                .clickable(onClick = onClick)
    ) {
        if (isUnread) {
            Box(
                modifier =
                    Modifier.padding(top = 8.dp, bottom = 8.dp)
                        .width(3.dp)
                        .fillMaxHeight()
                        .clip(RoundedCornerShape(topEnd = 3.dp, bottomEnd = 3.dp))
                        .background(MaterialTheme.colorScheme.primary)
                        .align(Alignment.CenterStart)
            )
        }
        Row(
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // Avatar + listing thumb overlay
            Box(modifier = Modifier.size(54.dp)) {
                val initials =
                    remember(inquiry.realtor?.name) {
                        inquiry.realtor
                            ?.name
                            ?.split(" ")
                            ?.take(2)
                            ?.mapNotNull { it.firstOrNull()?.uppercase() }
                            ?.joinToString("")
                            .orEmpty()
                            .ifEmpty { "?" }
                    }
                Box(
                    modifier =
                        Modifier.size(48.dp)
                            .clip(CircleShape)
                            .drawBehind {
                                drawRect(
                                    brush =
                                        Brush.linearGradient(
                                            colors = listOf(Brand800, Brand500),
                                            start = Offset(0f, 0f),
                                            end = Offset(size.width, size.height),
                                        )
                                )
                            }
                            .align(Alignment.TopStart),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = initials,
                        style = MaterialTheme.typography.labelLarge,
                        fontWeight = FontWeight.SemiBold,
                        color = Color.White,
                    )
                }
                inquiry.listing?.primaryImage?.url?.let { url ->
                    AsyncImage(
                        model =
                            ImageRequest.Builder(LocalContext.current)
                                .data(url)
                                .crossfade(true)
                                .build(),
                        contentDescription = null,
                        contentScale = ContentScale.Crop,
                        modifier =
                            Modifier.size(22.dp)
                                .align(Alignment.BottomEnd)
                                .clip(RoundedCornerShape(6.dp))
                                .background(MaterialTheme.colorScheme.surface),
                    )
                }
            }
            Spacer(modifier = Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text =
                            inquiry.realtor?.name
                                ?: inquiry.listing?.title
                                ?: stringResource(R.string.property),
                        modifier = Modifier.weight(1f),
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = if (isUnread) FontWeight.Bold else FontWeight.SemiBold,
                        color = MaterialTheme.colorScheme.onSurface,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = formatIsoDate(inquiry.createdAt),
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = if (isUnread) FontWeight.SemiBold else FontWeight.Medium,
                        color =
                            if (isUnread) MaterialTheme.colorScheme.primary
                            else MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Spacer(modifier = Modifier.height(3.dp))
                val latestMessage = inquiry.responses.lastOrNull()?.message ?: inquiry.message
                Text(
                    text = latestMessage,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(modifier = Modifier.height(6.dp))
                InquiryStatusPill(status = inquiry.status)
            }
        }
        HorizontalDivider(
            modifier = Modifier.align(Alignment.BottomCenter),
            color = MaterialTheme.colorScheme.outlineVariant,
        )
    }
}

@Composable
private fun InquiryStatusPill(status: InquiryStatus) {
    val (bg, ink, label) =
        when (status) {
            InquiryStatus.PENDING ->
                Triple(
                    InquiryStatusColors.pendingBg,
                    InquiryStatusColors.pendingInk,
                    stringResource(R.string.status_pending),
                )
            InquiryStatus.RESPONDED ->
                Triple(
                    InquiryStatusColors.respondedBg,
                    InquiryStatusColors.respondedInk,
                    stringResource(R.string.status_responded),
                )
            InquiryStatus.CLOSED ->
                Triple(
                    InquiryStatusColors.closedBg,
                    InquiryStatusColors.closedInk,
                    stringResource(R.string.status_closed),
                )
        }
    Surface(shape = RoundedCornerShape(999.dp), color = bg) {
        Text(
            text = label.uppercase(),
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
            style =
                MaterialTheme.typography.labelSmall.copy(
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 0.04.sp,
                    fontSize = 10.sp,
                ),
            color = ink,
        )
    }
}

// ─── Viewings list (preserved) ───────────────────────────────────────────────

@Composable
private fun ViewingsList(
    viewings: List<ViewingRequest>,
    onViewingClick: (ViewingRequest) -> Unit,
    onCancelViewing: (String) -> Unit,
) {
    if (viewings.isEmpty()) {
        EmptyViewings()
        return
    }
    LazyColumn(
        contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 96.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        items(viewings, key = { it.id }) { viewing ->
            ViewingCard(
                viewing = viewing,
                onClick = { onViewingClick(viewing) },
                onCancel = { onCancelViewing(viewing.id) },
            )
        }
    }
}

@Composable
private fun ViewingCard(viewing: ViewingRequest, onClick: () -> Unit, onCancel: () -> Unit) {
    var showCancelDialog by remember { mutableStateOf(false) }
    Card(
        modifier = Modifier.fillMaxWidth(),
        onClick = onClick,
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                ViewingStatusPill(status = viewing.status)
                Spacer(modifier = Modifier.weight(1f))
                if (
                    viewing.status == ViewingStatus.PENDING ||
                        viewing.status == ViewingStatus.CONFIRMED
                ) {
                    IconButton(onClick = { showCancelDialog = true }) {
                        Icon(
                            Icons.Default.Cancel,
                            contentDescription = stringResource(R.string.cancel),
                            tint = MaterialTheme.colorScheme.error,
                        )
                    }
                }
            }
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = viewing.listing?.title ?: stringResource(R.string.property_viewing),
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(modifier = Modifier.height(8.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    Icons.Default.CalendarToday,
                    contentDescription = null,
                    modifier = Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.primary,
                )
                Spacer(modifier = Modifier.width(6.dp))
                val displayDate = viewing.confirmedDate ?: viewing.preferredDate
                val displayTime = viewing.confirmedTime ?: viewing.preferredTime
                Text(
                    text = "$displayDate · $displayTime",
                    style = MaterialTheme.typography.bodyMedium,
                )
                if (viewing.confirmedDate != null) {
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = stringResource(R.string.viewing_confirmed_label),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
            }
            viewing.message?.let { message ->
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = message,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }

    if (showCancelDialog) {
        AlertDialog(
            onDismissRequest = { showCancelDialog = false },
            title = { Text(stringResource(R.string.viewing_cancel_title)) },
            text = { Text(stringResource(R.string.viewing_cancel_message)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        onCancel()
                        showCancelDialog = false
                    },
                    colors =
                        ButtonDefaults.textButtonColors(
                            contentColor = MaterialTheme.colorScheme.error
                        ),
                ) {
                    Text(stringResource(R.string.viewing_cancel_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { showCancelDialog = false }) {
                    Text(stringResource(R.string.viewing_cancel_keep))
                }
            },
        )
    }
}

@Composable
private fun ViewingStatusPill(status: ViewingStatus) {
    val (bg, ink, label) =
        when (status) {
            ViewingStatus.PENDING ->
                Triple(
                    ViewingStatusColors.pendingBg,
                    ViewingStatusColors.pendingInk,
                    stringResource(R.string.status_pending),
                )
            ViewingStatus.CONFIRMED ->
                Triple(
                    ViewingStatusColors.confirmedBg,
                    ViewingStatusColors.confirmedInk,
                    stringResource(R.string.status_confirmed),
                )
            ViewingStatus.COMPLETED ->
                Triple(
                    ViewingStatusColors.completedBg,
                    ViewingStatusColors.completedInk,
                    stringResource(R.string.status_completed),
                )
            ViewingStatus.CANCELLED ->
                Triple(
                    ViewingStatusColors.cancelledBg,
                    ViewingStatusColors.cancelledInk,
                    stringResource(R.string.status_cancelled),
                )
        }
    Surface(shape = RoundedCornerShape(999.dp), color = bg) {
        Text(
            text = label.uppercase(),
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
            style =
                MaterialTheme.typography.labelSmall.copy(
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 0.04.sp,
                    fontSize = 10.sp,
                ),
            color = ink,
        )
    }
}

// ─── Empty / error / unauth states ──────────────────────────────────────────

@Composable
private fun NotSignedInContent(onSignInClick: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Icon(
            Icons.Default.Email,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = stringResource(R.string.inquiries_sign_in_title),
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.inquiries_sign_in_description),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(24.dp))
        Button(
            onClick = onSignInClick,
            shape = RoundedCornerShape(14.dp),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(stringResource(R.string.sign_in))
        }
    }
}

@Composable
private fun EmptyInquiries() {
    Column(
        modifier = Modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Icon(
            Icons.Default.MarkEmailRead,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = stringResource(R.string.inquiries_empty_title),
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.inquiries_empty_description),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun EmptyViewings() {
    Column(
        modifier = Modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Icon(
            Icons.Default.CalendarMonth,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = stringResource(R.string.viewings_empty_title),
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.viewings_empty_description),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun ErrorContent(message: String, onRetry: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Icon(
            Icons.Default.Error,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.error,
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = message,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(16.dp))
        Button(onClick = onRetry) { Text(stringResource(R.string.retry)) }
    }
}

/**
 * Trims an ISO-8601 timestamp (e.g. `2026-05-13T11:00:00Z`) down to its `YYYY-MM-DD` prefix. Real
 * relative-time formatting (kotlinx-datetime + locale-aware) is tracked as a follow-up.
 */
private fun formatIsoDate(dateString: String): String = dateString.take(10)
