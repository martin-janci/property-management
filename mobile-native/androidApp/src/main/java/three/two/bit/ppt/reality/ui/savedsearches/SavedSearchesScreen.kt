package three.two.bit.ppt.reality.ui.savedsearches

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

/**
 * Saved searches screen for Reality Portal Android (UC-45.2).
 *
 * Renders the user's saved searches with toggles for alert delivery. The
 * mobile-native module doesn't yet expose `/saved-searches` endpoints; the
 * screen reads/writes via the supplied callbacks so the navigation path
 * exists end-to-end.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SavedSearchesScreen(
    searches: List<SavedSearch>,
    isLoading: Boolean,
    onBackClick: () -> Unit,
    onSearchClick: (id: String) -> Unit,
    onToggleAlerts: (id: String, enabled: Boolean) -> Unit,
    onDelete: (id: String) -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Saved searches") },
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
                    modifier = Modifier.fillMaxSize().padding(padding).padding(24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center,
                ) {
                    Text(
                        text = "Loading saved searches…",
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }
            }
            searches.isEmpty() -> EmptyState(modifier = Modifier.padding(padding))
            else -> {
                LazyColumn(
                    modifier = Modifier.fillMaxSize().padding(padding),
                    contentPadding = PaddingValues(16.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    items(searches, key = { it.id }) { search ->
                        SavedSearchCard(
                            search = search,
                            onClick = { onSearchClick(search.id) },
                            onToggleAlerts = { enabled -> onToggleAlerts(search.id, enabled) },
                            onDelete = { onDelete(search.id) },
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun SavedSearchCard(
    search: SavedSearch,
    onClick: () -> Unit,
    onToggleAlerts: (Boolean) -> Unit,
    onDelete: () -> Unit,
) {
    var deleteConfirm by remember { mutableStateOf(false) }
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(
                text = search.name,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
            )
            Text(
                text = search.summary,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.height(4.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Switch(checked = search.alertsEnabled, onCheckedChange = onToggleAlerts)
                    Text(
                        text = if (search.alertsEnabled) "Alerts on" else "Alerts off",
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }
                if (deleteConfirm) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        TextButton(onClick = { deleteConfirm = false }) { Text("Cancel") }
                        TextButton(
                            onClick = {
                                deleteConfirm = false
                                onDelete()
                            }
                        ) {
                            Text(text = "Delete", color = MaterialTheme.colorScheme.error)
                        }
                    }
                } else {
                    TextButton(onClick = { deleteConfirm = true }) { Text("Delete") }
                }
            }
            TextButton(onClick = onClick) { Text("Run search") }
        }
    }
}

@Composable
private fun EmptyState(modifier: Modifier) {
    Column(
        modifier = modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Icon(
            Icons.Default.Search,
            contentDescription = null,
            modifier = Modifier.padding(bottom = 16.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            text = "No saved searches yet",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold,
        )
        Spacer(Modifier.height(8.dp))
        Text(
            text =
                "Save a search to get notified about new listings that match your criteria.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/** Lightweight VM-shaped data class fed into the screen. */
data class SavedSearch(
    val id: String,
    val name: String,
    val summary: String,
    val alertsEnabled: Boolean,
)
