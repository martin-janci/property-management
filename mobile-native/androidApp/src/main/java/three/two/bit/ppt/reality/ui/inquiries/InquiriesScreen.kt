package three.two.bit.ppt.reality.ui.inquiries

import android.util.Log
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import coil.request.ImageRequest
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.api.ApiConfig
import three.two.bit.ppt.reality.auth.AuthState
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.inquiry.*
import three.two.bit.ppt.reality.listing.ListingRepository
import three.two.bit.ppt.reality.ui.theme.InquiryStatusColors
import three.two.bit.ppt.reality.ui.theme.ViewingStatusColors

private const val TAG = "InquiriesScreen"

/**
 * Inquiries screen for Reality Portal mobile app.
 *
 * Epic 48 - Story 48.6: Portal Mobile Inquiries
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InquiriesScreen(
    repository: ListingRepository,
    ssoService: SsoService,
    onListingClick: (String) -> Unit,
    onBackClick: () -> Unit
) {
    val scope = rememberCoroutineScope()
    val authState by ssoService.authState.collectAsState()

    var selectedTab by remember { mutableIntStateOf(0) }
    var inquiries by remember { mutableStateOf<List<Inquiry>>(emptyList()) }
    var viewings by remember { mutableStateOf<List<ViewingRequest>>(emptyList()) }
    var isLoading by remember { mutableStateOf(true) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    // Create inquiry repository with session token
    val inquiryRepository =
        remember(authState) {
            val token = (authState as? AuthState.Authenticated)?.sessionToken
            InquiryRepository(baseUrl = ApiConfig.requireBaseUrl(), sessionToken = token)
        }

    // Load data
    LaunchedEffect(authState) {
        if (authState is AuthState.Authenticated) {
            isLoading = true
            errorMessage = null

            // Load inquiries
            inquiryRepository
                .getInquiries()
                .fold(
                    onSuccess = { response -> inquiries = response.inquiries },
                    onFailure = { error -> errorMessage = error.message }
                )

            // Load viewings
            inquiryRepository
                .getViewings()
                .fold(
                    onSuccess = { response -> viewings = response.viewings },
                    onFailure = { error ->
                        Log.e(TAG, "Failed to load viewings", error)
                        // Don't overwrite error message if inquiries already failed
                    }
                )

            isLoading = false
        } else {
            isLoading = false
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.inquiries)) },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(
                            Icons.Default.ArrowBack,
                            contentDescription = stringResource(R.string.back)
                        )
                    }
                }
            )
        }
    ) { paddingValues ->
        when (authState) {
            is AuthState.Unauthenticated,
            is AuthState.Error -> {
                NotSignedInContent(modifier = Modifier.padding(paddingValues))
            }
            is AuthState.Loading -> {
                Box(
                    modifier = Modifier.fillMaxSize().padding(paddingValues),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            }
            is AuthState.Authenticated -> {
                Column(modifier = Modifier.fillMaxSize().padding(paddingValues)) {
                    // Tab row
                    TabRow(selectedTabIndex = selectedTab) {
                        Tab(
                            selected = selectedTab == 0,
                            onClick = { selectedTab = 0 },
                            text = {
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    Text(stringResource(R.string.inquiries_tab_messages))
                                    val pendingCount =
                                        inquiries.count { it.status == InquiryStatus.RESPONDED }
                                    if (pendingCount > 0) {
                                        Spacer(modifier = Modifier.width(4.dp))
                                        Badge { Text("$pendingCount") }
                                    }
                                }
                            },
                            icon = { Icon(Icons.Default.Email, contentDescription = null) }
                        )
                        Tab(
                            selected = selectedTab == 1,
                            onClick = { selectedTab = 1 },
                            text = {
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    Text(stringResource(R.string.inquiries_tab_viewings))
                                    val upcomingCount =
                                        viewings.count { it.status == ViewingStatus.CONFIRMED }
                                    if (upcomingCount > 0) {
                                        Spacer(modifier = Modifier.width(4.dp))
                                        Badge { Text("$upcomingCount") }
                                    }
                                }
                            },
                            icon = { Icon(Icons.Default.CalendarMonth, contentDescription = null) }
                        )
                    }

                    // Content
                    when {
                        isLoading -> {
                            Box(
                                modifier = Modifier.fillMaxSize(),
                                contentAlignment = Alignment.Center
                            ) {
                                CircularProgressIndicator()
                            }
                        }
                        errorMessage != null -> {
                            ErrorContent(
                                message = errorMessage!!,
                                onRetry = {
                                    scope.launch {
                                        isLoading = true
                                        errorMessage = null
                                        inquiryRepository
                                            .getInquiries()
                                            .fold(
                                                onSuccess = { response ->
                                                    inquiries = response.inquiries
                                                },
                                                onFailure = { error ->
                                                    errorMessage = error.message
                                                }
                                            )
                                        isLoading = false
                                    }
                                }
                            )
                        }
                        selectedTab == 0 -> {
                            InquiriesList(
                                inquiries = inquiries,
                                onInquiryClick = { inquiry -> onListingClick(inquiry.listingId) }
                            )
                        }
                        selectedTab == 1 -> {
                            ViewingsList(
                                viewings = viewings,
                                onViewingClick = { viewing -> onListingClick(viewing.listingId) },
                                onCancelViewing = { viewingId ->
                                    scope.launch {
                                        inquiryRepository
                                            .cancelViewing(viewingId)
                                            .fold(
                                                onSuccess = {
                                                    viewings =
                                                        viewings.filter { it.id != viewingId }
                                                },
                                                onFailure = { error ->
                                                    Log.e(TAG, "Failed to cancel viewing", error)
                                                    errorMessage = error.message
                                                }
                                            )
                                    }
                                }
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun NotSignedInContent(modifier: Modifier = Modifier) {
    Column(
        modifier = modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Icon(
            Icons.Default.Email,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = stringResource(R.string.inquiries_sign_in_title),
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.inquiries_sign_in_description),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun InquiriesList(inquiries: List<Inquiry>, onInquiryClick: (Inquiry) -> Unit) {
    if (inquiries.isEmpty()) {
        EmptyInquiries()
    } else {
        LazyColumn(
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            items(inquiries, key = { it.id }) { inquiry ->
                InquiryCard(inquiry = inquiry, onClick = { onInquiryClick(inquiry) })
            }
        }
    }
}

@Composable
private fun InquiryCard(inquiry: Inquiry, onClick: () -> Unit) {
    Card(modifier = Modifier.fillMaxWidth(), onClick = onClick) {
        Row(modifier = Modifier.fillMaxWidth().padding(12.dp), verticalAlignment = Alignment.Top) {
            // Listing image
            AsyncImage(
                model =
                    ImageRequest.Builder(LocalContext.current)
                        .data(inquiry.listing?.primaryImage?.url ?: "")
                        .crossfade(true)
                        .build(),
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.size(80.dp).clip(RoundedCornerShape(8.dp))
            )

            Spacer(modifier = Modifier.width(12.dp))

            Column(modifier = Modifier.weight(1f)) {
                // Status badge
                Row(verticalAlignment = Alignment.CenterVertically) {
                    StatusBadge(status = inquiry.status)
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = formatDate(inquiry.createdAt),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                Spacer(modifier = Modifier.height(4.dp))

                // Property title
                Text(
                    text = inquiry.listing?.title ?: stringResource(R.string.property),
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )

                Spacer(modifier = Modifier.height(4.dp))

                // Latest message preview
                val latestMessage = inquiry.responses.lastOrNull()?.message ?: inquiry.message
                Text(
                    text = latestMessage,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis
                )

                // Response count
                if (inquiry.responses.isNotEmpty()) {
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text =
                            stringResource(
                                R.string.inquiry_responses_count,
                                inquiry.responses.size
                            ),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary
                    )
                }
            }
        }
    }
}

@Composable
private fun StatusBadge(status: InquiryStatus) {
    val (bg, ink, textRes) =
        when (status) {
            InquiryStatus.PENDING ->
                Triple(
                    InquiryStatusColors.pendingBg,
                    InquiryStatusColors.pendingInk,
                    R.string.status_pending,
                )
            InquiryStatus.RESPONDED ->
                Triple(
                    InquiryStatusColors.respondedBg,
                    InquiryStatusColors.respondedInk,
                    R.string.status_responded,
                )
            InquiryStatus.CLOSED ->
                Triple(
                    InquiryStatusColors.closedBg,
                    InquiryStatusColors.closedInk,
                    R.string.status_closed,
                )
        }

    Surface(shape = RoundedCornerShape(4.dp), color = bg) {
        Text(
            text = stringResource(textRes),
            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
            style = MaterialTheme.typography.labelSmall,
            color = ink
        )
    }
}

@Composable
private fun EmptyInquiries() {
    Column(
        modifier = Modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Icon(
            Icons.Default.MarkEmailRead,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = stringResource(R.string.inquiries_empty_title),
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.inquiries_empty_description),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun ViewingsList(
    viewings: List<ViewingRequest>,
    onViewingClick: (ViewingRequest) -> Unit,
    onCancelViewing: (String) -> Unit
) {
    if (viewings.isEmpty()) {
        EmptyViewings()
    } else {
        LazyColumn(
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            items(viewings, key = { it.id }) { viewing ->
                ViewingCard(
                    viewing = viewing,
                    onClick = { onViewingClick(viewing) },
                    onCancel = { onCancelViewing(viewing.id) }
                )
            }
        }
    }
}

@Composable
private fun ViewingCard(viewing: ViewingRequest, onClick: () -> Unit, onCancel: () -> Unit) {
    var showCancelDialog by remember { mutableStateOf(false) }

    Card(modifier = Modifier.fillMaxWidth(), onClick = onClick) {
        Column(modifier = Modifier.fillMaxWidth().padding(16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                ViewingStatusBadge(status = viewing.status)
                Spacer(modifier = Modifier.weight(1f))

                if (
                    viewing.status == ViewingStatus.PENDING ||
                        viewing.status == ViewingStatus.CONFIRMED
                ) {
                    IconButton(onClick = { showCancelDialog = true }) {
                        Icon(
                            Icons.Default.Cancel,
                            contentDescription = stringResource(R.string.cancel),
                            tint = MaterialTheme.colorScheme.error
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            // Property
            Text(
                text = viewing.listing?.title ?: stringResource(R.string.property_viewing),
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold
            )

            Spacer(modifier = Modifier.height(8.dp))

            // Date and time
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    Icons.Default.CalendarToday,
                    contentDescription = null,
                    modifier = Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.primary
                )
                Spacer(modifier = Modifier.width(4.dp))

                val displayDate = viewing.confirmedDate ?: viewing.preferredDate
                val displayTime = viewing.confirmedTime ?: viewing.preferredTime

                Text(
                    text = "$displayDate at $displayTime",
                    style = MaterialTheme.typography.bodyMedium
                )

                if (viewing.confirmedDate != null) {
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = stringResource(R.string.viewing_confirmed_label),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.primary
                    )
                }
            }

            // Message
            viewing.message?.let { message ->
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = message,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis
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
                        )
                ) {
                    Text(stringResource(R.string.viewing_cancel_confirm))
                }
            },
            dismissButton = {
                TextButton(onClick = { showCancelDialog = false }) {
                    Text(stringResource(R.string.viewing_cancel_keep))
                }
            }
        )
    }
}

@Composable
private fun ViewingStatusBadge(status: ViewingStatus) {
    data class BadgeAttrs(
        val bg: androidx.compose.ui.graphics.Color,
        val ink: androidx.compose.ui.graphics.Color,
        val textRes: Int,
        val icon: androidx.compose.ui.graphics.vector.ImageVector
    )

    val attrs = when (status) {
        ViewingStatus.PENDING ->
            BadgeAttrs(ViewingStatusColors.pendingBg, ViewingStatusColors.pendingInk,
                R.string.status_pending, Icons.Default.Schedule)
        ViewingStatus.CONFIRMED ->
            BadgeAttrs(ViewingStatusColors.confirmedBg, ViewingStatusColors.confirmedInk,
                R.string.status_confirmed, Icons.Default.CheckCircle)
        ViewingStatus.COMPLETED ->
            BadgeAttrs(ViewingStatusColors.completedBg, ViewingStatusColors.completedInk,
                R.string.status_completed, Icons.Default.Done)
        ViewingStatus.CANCELLED ->
            BadgeAttrs(ViewingStatusColors.cancelledBg, ViewingStatusColors.cancelledInk,
                R.string.status_cancelled, Icons.Default.Cancel)
    }

    Surface(shape = RoundedCornerShape(4.dp), color = attrs.bg) {
        Row(
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                attrs.icon,
                contentDescription = null,
                modifier = Modifier.size(14.dp),
                tint = attrs.ink,
            )
            Spacer(modifier = Modifier.width(4.dp))
            Text(
                text = stringResource(attrs.textRes),
                style = MaterialTheme.typography.labelSmall,
                color = attrs.ink
            )
        }
    }
}

@Composable
private fun EmptyViewings() {
    Column(
        modifier = Modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Icon(
            Icons.Default.CalendarMonth,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = stringResource(R.string.viewings_empty_title),
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.viewings_empty_description),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun ErrorContent(message: String, onRetry: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Icon(
            Icons.Default.Error,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.error
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = message,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.height(16.dp))
        Button(onClick = onRetry) { Text(stringResource(R.string.retry)) }
    }
}

private fun formatDate(dateString: String): String {
    // Simple date formatting - in production use kotlinx-datetime
    return dateString.take(10)
}
