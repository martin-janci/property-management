package three.two.bit.ppt.reality.ui.agency

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Inbox
import androidx.compose.material3.AssistChip
import androidx.compose.material3.AssistChipDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

/**
 * Agency inquiries screen for Reality Portal Android (UC-49.5).
 *
 * Lists inquiries received for the signed-in agency's listings. Caller supplies the data so the
 * screen stays decoupled from the API client.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AgencyInquiriesScreen(
    inquiries: List<AgencyInquiry>,
    isLoading: Boolean,
    onBackClick: () -> Unit,
    onInquiryClick: (id: String) -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Agency inquiries") },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        }
    ) { padding ->
        when {
            isLoading -> {
                Column(
                    modifier = Modifier.fillMaxSize().padding(padding),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center,
                ) {
                    Text("Loading inquiries…", style = MaterialTheme.typography.bodyMedium)
                }
            }
            inquiries.isEmpty() ->
                Column(
                    modifier = Modifier.fillMaxSize().padding(padding).padding(32.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center,
                ) {
                    Icon(
                        Icons.Default.Inbox,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        text = "No inquiries yet",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                    )
                    Text(
                        text = "Inquiries from interested buyers and renters will appear here.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            else ->
                LazyColumn(
                    modifier = Modifier.fillMaxSize().padding(padding),
                    contentPadding = PaddingValues(16.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    items(inquiries, key = { it.id }) { inquiry ->
                        InquiryCard(inquiry, onClick = { onInquiryClick(inquiry.id) })
                    }
                }
        }
    }
}

@Composable
private fun InquiryCard(inquiry: AgencyInquiry, onClick: () -> Unit) {
    Card(modifier = Modifier.fillMaxWidth(), onClick = onClick) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(
                text = inquiry.listingTitle,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
            )
            Text(
                text = "${inquiry.contactName} · ${inquiry.contactEmail}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                text = inquiry.message,
                style = MaterialTheme.typography.bodyMedium,
                maxLines = 3,
            )
            AssistChip(
                onClick = onClick,
                label = { Text(inquiry.status) },
                colors = AssistChipDefaults.assistChipColors(),
            )
        }
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
