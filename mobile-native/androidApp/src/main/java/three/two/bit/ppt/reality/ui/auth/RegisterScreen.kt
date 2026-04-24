package three.two.bit.ppt.reality.ui.auth

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch

/**
 * Registration screen for Reality Portal Android (UC-47.1).
 *
 * Submits to reality-server `POST /api/v1/auth/register` (display name, email, password). On
 * success the user is asked to verify their email.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RegisterScreen(
    onBackClick: () -> Unit,
    onSubmit: suspend (displayName: String, email: String, password: String) -> Result<Unit>,
    onSignInClick: () -> Unit,
) {
    var displayName by remember { mutableStateOf("") }
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var confirmPassword by remember { mutableStateOf("") }
    var displayNameError by remember { mutableStateOf<String?>(null) }
    var emailError by remember { mutableStateOf<String?>(null) }
    var passwordError by remember { mutableStateOf<String?>(null) }
    var confirmError by remember { mutableStateOf<String?>(null) }
    var generalError by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }
    var submitted by remember { mutableStateOf(false) }

    val scope = rememberAuthScope()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(if (submitted) "Check your inbox" else "Create account") },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        }
    ) { padding ->
        Column(
            modifier =
                Modifier.fillMaxSize()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            if (submitted) {
                Text(
                    text = "Check your inbox",
                    style = MaterialTheme.typography.headlineSmall,
                    fontWeight = FontWeight.Bold,
                )
                Text(
                    text =
                        "We sent a verification link to $email. Click the link to activate your account.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Button(onClick = onSignInClick, modifier = Modifier.fillMaxWidth()) {
                    Text("Back to sign in")
                }
                return@Column
            }

            Text(
                text = "Create your account",
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
            )
            Text(
                text = "Save listings, set alerts, and contact agents.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            generalError?.let { ErrorBanner(it) }

            OutlinedTextField(
                value = displayName,
                onValueChange = {
                    displayName = it
                    displayNameError = null
                },
                label = { Text("Display name") },
                singleLine = true,
                isError = displayNameError != null,
                supportingText = { displayNameError?.let { Text(it) } },
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = email,
                onValueChange = {
                    email = it
                    emailError = null
                },
                label = { Text("Email") },
                singleLine = true,
                isError = emailError != null,
                supportingText = { emailError?.let { Text(it) } },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email),
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = password,
                onValueChange = {
                    password = it
                    passwordError = null
                },
                label = { Text("Password") },
                singleLine = true,
                isError = passwordError != null,
                supportingText = {
                    Text(passwordError ?: "At least $MIN_PASSWORD_LENGTH characters.")
                },
                visualTransformation = PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = confirmPassword,
                onValueChange = {
                    confirmPassword = it
                    confirmError = null
                },
                label = { Text("Confirm password") },
                singleLine = true,
                isError = confirmError != null,
                supportingText = { confirmError?.let { Text(it) } },
                visualTransformation = PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                modifier = Modifier.fillMaxWidth(),
            )

            Button(
                onClick = {
                    displayNameError =
                        if (displayName.isBlank()) "Display name is required" else null
                    emailError =
                        when {
                            email.isBlank() -> "Email is required"
                            !emailRegex.matches(email.trim()) -> "Enter a valid email address"
                            else -> null
                        }
                    passwordError =
                        when {
                            password.isEmpty() -> "Password is required"
                            password.length < MIN_PASSWORD_LENGTH ->
                                "Password must be at least $MIN_PASSWORD_LENGTH characters"
                            else -> null
                        }
                    confirmError =
                        if (confirmPassword != password) "Passwords do not match" else null
                    if (
                        listOf(displayNameError, emailError, passwordError, confirmError).any {
                            it != null
                        }
                    ) {
                        return@Button
                    }
                    isSubmitting = true
                    generalError = null
                    scope.launch {
                        val result = onSubmit(displayName.trim(), email.trim(), password)
                        isSubmitting = false
                        result.fold(
                            onSuccess = { submitted = true },
                            onFailure = { generalError = it.message ?: "Registration failed." },
                        )
                    }
                },
                enabled = !isSubmitting,
                modifier = Modifier.fillMaxWidth(),
                contentPadding = PaddingValues(vertical = 14.dp),
            ) {
                if (isSubmitting) {
                    CircularProgressIndicator(modifier = Modifier.padding(end = 8.dp))
                }
                Text("Create account")
            }

            TextButton(onClick = onSignInClick, modifier = Modifier.fillMaxWidth()) {
                Text("Already have an account? Sign in")
            }
        }
    }
}
