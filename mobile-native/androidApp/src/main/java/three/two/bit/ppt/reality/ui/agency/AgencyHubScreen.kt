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
import androidx.compose.material.icons.filled.ChevronRight
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
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
 * Entry point for agency owners on Reality Portal Android (UC-49).
 *
 * Lists the available agency-management workflows. Detailed dashboards live on the web; this screen
 * exposes a focused mobile menu so the surfaces are reachable from the app.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AgencyHubScreen(
    onBackClick: () -> Unit,
    onInquiriesClick: () -> Unit,
    onMyListingsClick: () -> Unit,
    onCreateListingClick: () -> Unit,
    onAnalyticsClick: () -> Unit,
) {
    val sections =
        listOf(
            AgencySection(
                "Inquiries",
                "Review messages received for your listings.",
                onInquiriesClick
            ),
            AgencySection("My listings", "Listings you currently manage.", onMyListingsClick),
            AgencySection(
                "Create listing",
                "Publish a new property to the portal.",
                onCreateListingClick
            ),
            AgencySection("Analytics", "Performance of your listings over time.", onAnalyticsClick),
        )

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Agency") },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            items(sections, key = { it.title }) { section -> AgencyCard(section) }
        }
    }
}

@Composable
private fun AgencyCard(section: AgencySection) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        onClick = section.onClick,
    ) {
        androidx.compose.foundation.layout.Row(
            modifier = Modifier.fillMaxWidth().padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = section.title,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                )
                Text(
                    text = section.description,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Icon(
                Icons.Default.ChevronRight,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

private data class AgencySection(
    val title: String,
    val description: String,
    val onClick: () -> Unit,
)
