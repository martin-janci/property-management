package three.two.bit.ppt.reality.ui.profile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.auth.AuthState
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.ui.auth.ErrorBanner
import three.two.bit.ppt.reality.ui.auth.SuccessBanner
import three.two.bit.ppt.reality.ui.auth.rememberAuthScope

/**
 * Profile edit screen for Reality Portal Android (UC-47.7).
 *
 * Wired via `Navigation.kt` to `SsoService.updateProfile`, which calls reality-server `PUT
 * /api/v1/users/me` and pushes the refreshed user back into the shared `authState`. The screen
 * delegates the actual network call through the supplied `onSubmit` callback so the UI stays
 * decoupled from the service.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProfileEditScreen(
    ssoService: SsoService,
    onBackClick: () -> Unit,
    onSubmit: suspend (displayName: String) -> Result<Unit>,
) {
    val authState by ssoService.authState.collectAsState()
    val authenticated = authState as? AuthState.Authenticated

    var displayName by
        remember(authenticated?.user?.userId) {
            mutableStateOf(authenticated?.user?.name.orEmpty())
        }
    var displayNameError by remember { mutableStateOf<String?>(null) }
    var generalError by remember { mutableStateOf<String?>(null) }
    var savedMessage by remember { mutableStateOf<String?>(null) }
    var isSaving by remember { mutableStateOf(false) }

    val scope = rememberAuthScope()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Edit profile") },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            if (authenticated == null) {
                Text(
                    text = "Sign in required",
                    style = MaterialTheme.typography.headlineSmall,
                    fontWeight = FontWeight.Bold,
                )
                Text(
                    text = "Please sign in to edit your profile.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                return@Column
            }

            Text(
                text = "Edit profile",
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
            )
            Text(
                text = "Update how your name appears in the app.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            generalError?.let { ErrorBanner(it) }
            savedMessage?.let { SuccessBanner(it) }

            OutlinedTextField(
                value = displayName,
                onValueChange = {
                    displayName = it
                    displayNameError = null
                    savedMessage = null
                },
                label = { Text("Display name") },
                singleLine = true,
                isError = displayNameError != null,
                supportingText = { displayNameError?.let { Text(it) } },
                modifier = Modifier.fillMaxWidth(),
            )

            OutlinedTextField(
                value = authenticated.user.email,
                onValueChange = {},
                label = { Text("Email") },
                singleLine = true,
                enabled = false,
                supportingText = { Text("Contact support to change the email on this account.") },
                modifier = Modifier.fillMaxWidth(),
            )

            Button(
                onClick = {
                    if (displayName.isBlank()) {
                        displayNameError = "Display name is required"
                        return@Button
                    }
                    isSaving = true
                    generalError = null
                    savedMessage = null
                    scope.launch {
                        val result = onSubmit(displayName.trim())
                        isSaving = false
                        result.fold(
                            onSuccess = { savedMessage = "Profile updated." },
                            onFailure = { generalError = it.message ?: "Could not save profile." },
                        )
                    }
                },
                enabled = !isSaving,
                modifier = Modifier.fillMaxWidth(),
                contentPadding = PaddingValues(vertical = 14.dp),
            ) {
                if (isSaving) {
                    CircularProgressIndicator(modifier = Modifier.padding(end = 8.dp))
                }
                Text("Save changes")
            }
        }
    }
}
